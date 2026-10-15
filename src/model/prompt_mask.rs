use mlua::{Integer as LuaInt, Lua, LuaString, Result as LuaResult, Table as LuaTable};
use std::collections::BTreeMap;
use std::ops::{AddAssign, Deref, DerefMut};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PromptMask {
    mask: BTreeMap<i32, String>,
}

impl PromptMask {
    pub fn new() -> Self {
        PromptMask {
            mask: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.mask.clear()
    }

    pub fn mask_buffer(&self, buf: &[char]) -> String {
        let mut masked_buf = buf.to_owned();
        let mut offset = 0;
        for (idx, mask) in self.iter() {
            let adjusted_idx = offset + *idx as usize;
            if adjusted_idx > masked_buf.len() {
                return masked_buf.iter().collect();
            }
            masked_buf.splice(adjusted_idx..adjusted_idx, mask.chars());
            // `masked_buf` is a `Vec<char>`, so the splice grew it by the
            // mask's *character* count. `mask.len()` is its byte length, which
            // only agrees for ASCII masks — any multi-byte mask coming from
            // Lua pushed every subsequent index too far right.
            offset += mask.chars().count();
        }

        masked_buf.iter().collect()
    }

    /// Map a character index in the unmasked buffer to the equivalent index in
    /// the string [`Self::mask_buffer`] produces.
    ///
    /// Without this the cursor and the text it points at come from different
    /// coordinate spaces. A mask is not required to be an escape sequence —
    /// Lua may supply any string, and printable masks are the documented use
    /// (`"this is *important*, ok"`) — so the shift is real, not notional.
    ///
    /// `buf_len` is the unmasked buffer's character count, and exists so this
    /// reproduces `mask_buffer`'s behaviour exactly: an out-of-range index
    /// makes it stop applying masks altogether, so this must stop shifting at
    /// the same point or the two disagree.
    pub fn masked_index(&self, unmasked_idx: usize, buf_len: usize) -> usize {
        let mut offset = 0usize;
        for (idx, mask) in self.iter() {
            // The same cast `mask_buffer` performs, wrap included: a negative
            // index becomes enormous and trips the bounds check there too.
            let idx = *idx as usize;
            if idx > buf_len {
                break;
            }
            if idx > unmasked_idx {
                break;
            }
            // A mask anchored exactly at the cursor is spliced in *before* it,
            // so it pushes the cursor right.
            offset += mask.chars().count();
        }
        unmasked_idx + offset
    }

    pub fn to_table<'a>(&'a self, ctx: &'a Lua) -> LuaResult<LuaTable> {
        ctx.create_table_from(self.iter().map(|(idx, mask)| (*idx + 1, (*mask).clone())))
    }
}

impl Deref for PromptMask {
    type Target = BTreeMap<i32, String>;

    fn deref(&self) -> &Self::Target {
        &self.mask
    }
}

impl DerefMut for PromptMask {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mask
    }
}

impl AddAssign for PromptMask {
    fn add_assign(&mut self, rhs: Self) {
        self.mask.extend(rhs.mask)
    }
}

impl From<BTreeMap<i32, String>> for PromptMask {
    fn from(mask: BTreeMap<i32, String>) -> Self {
        PromptMask { mask }
    }
}

impl From<LuaTable> for PromptMask {
    fn from(mask_table: LuaTable) -> Self {
        let mask = mask_table
            .pairs::<LuaInt, LuaString>()
            .collect::<Result<Vec<(LuaInt, LuaString)>, _>>()
            .unwrap()
            .iter()
            .map(|(idx, mask)| ((*idx as i32) - 1, mask.to_str().unwrap().to_string()))
            .collect::<BTreeMap<i32, String>>();

        PromptMask { mask }
    }
}

#[cfg(test)]
mod test_prompt_mask {
    use crate::model::PromptMask;
    use mlua::{Lua, Table as LuaTable};
    use std::collections::BTreeMap;

    #[test]
    fn test_from_luatable() {
        let lua = Lua::new();
        let simple_mask: LuaTable = lua
            .load(
                r#"
    {
        [5] = "!",
        [1] = "*",
    }
"#,
            )
            .eval()
            .unwrap();
        // We expect the 1-indexed Lua table indices to have been translated to 0-indexing.
        let expected = BTreeMap::from([(0, "*".to_string()), (4, "!".to_string())]);

        let lua_mask = PromptMask::from(simple_mask);
        let expected_mask = PromptMask::from(expected);
        assert_eq!(lua_mask, expected_mask)
    }

    #[test]
    fn test_add_assign() {
        let mut mask_a = PromptMask::from(BTreeMap::from([
            (10, "*".to_string()),
            (15, "#".to_string()),
            (20, "!".to_string()),
        ]));

        let mask_b = PromptMask::from(BTreeMap::from([
            (1, "@".to_string()),
            (15, "%".to_string()),
            (25, "&".to_string()),
        ]));

        let expected = PromptMask::from(BTreeMap::from([
            (1, "@".to_string()),
            (10, "*".to_string()),
            (15, "%".to_string()), // NB: overwritten by map_b.
            (20, "!".to_string()),
            (25, "&".to_string()),
        ]));

        mask_a += mask_b;
        assert_eq!(mask_a, expected)
    }

    #[test]
    fn test_masking() {
        let buf = vec![
            't', 'h', 'i', 's', ' ', 'i', 's', ' ', 'i', 'm', 'p', 'o', 'r', 't', 'a', 'n', 't',
            ',', ' ', 'o', 'k',
        ];
        let mask = PromptMask::from(BTreeMap::from([
            (8, "*".to_string()),
            (17, "*".to_string()),
        ]));

        let res = mask.mask_buffer(&buf);
        assert_eq!(res, "this is *important*, ok");

        let invalid_mask = PromptMask::from(BTreeMap::from([
            (8, "*".to_string()),
            (800, "!".to_string()),
        ]));
        let res = invalid_mask.mask_buffer(&buf);
        assert_eq!(res, "this is *important, ok");
    }

    /// `offset` used to advance by the mask's byte length while the buffer it
    /// indexes is a `Vec<char>`. A multi-byte mask therefore over-advanced,
    /// and every mask after the first landed too far right.
    #[test]
    fn mask_with_multibyte_content_keeps_subsequent_indices_aligned() {
        let buf: Vec<char> = "this is important, ok".chars().collect();

        // "»" is one char but two bytes; "«" likewise.
        let mask = PromptMask::from(BTreeMap::from([
            (8, "»".to_string()),
            (17, "«".to_string()),
        ]));

        assert_eq!(mask.mask_buffer(&buf), "this is »important«, ok");
    }

    /// A mask anchored exactly at the end of the buffer is in bounds and must
    /// be appended, not dropped.
    #[test]
    fn mask_at_exact_buffer_end_is_appended() {
        let buf: Vec<char> = "abc".chars().collect();
        let mask = PromptMask::from(BTreeMap::from([(3, "!".to_string())]));
        assert_eq!(mask.mask_buffer(&buf), "abc!");
    }

    /// The cursor index must be translated into the same coordinate space as
    /// the string it indexes. Asserted against `mask_buffer` rather than
    /// hand-written numbers, so the two cannot drift.
    #[test]
    fn masked_index_agrees_with_mask_buffer() {
        let buf: Vec<char> = "this is important, ok".chars().collect();

        for mask in [
            // Printable masks, which shift real columns.
            PromptMask::from(BTreeMap::from([
                (8, "*".to_string()),
                (17, "*".to_string()),
            ])),
            // Escape masks, which shift indices but no columns.
            PromptMask::from(BTreeMap::from([
                (0, "\x1b[44m".to_string()),
                (21, "\x1b[0m".to_string()),
            ])),
            // Multi-byte, multi-char masks.
            PromptMask::from(BTreeMap::from([(4, "»«".to_string())])),
            PromptMask::new(),
        ] {
            let masked: Vec<char> = mask.mask_buffer(&buf).chars().collect();

            for pos in 0..=buf.len() {
                let mapped = mask.masked_index(pos, buf.len());
                assert!(
                    mapped <= masked.len(),
                    "index {mapped} out of range for {masked:?}"
                );
                // The cursor must sit immediately before the same character it
                // sat before in the unmasked buffer. A mask anchored exactly
                // at the cursor belongs to its left — which is what makes the
                // boundary `<=` rather than `<`.
                if pos < buf.len() {
                    assert_eq!(
                        masked[mapped],
                        buf[pos],
                        "cursor {pos} -> {mapped} points at the wrong character \
                         in {:?}",
                        masked.iter().collect::<String>()
                    );
                }
            }

            // The end of the buffer maps to the end of the masked buffer.
            assert_eq!(mask.masked_index(buf.len(), buf.len()), masked.len());
        }
    }

    /// With no mask the mapping is the identity, which is what keeps the
    /// ordinary no-mask render path unchanged.
    #[test]
    fn masked_index_is_identity_without_a_mask() {
        let mask = PromptMask::new();
        for pos in 0..10 {
            assert_eq!(mask.masked_index(pos, 9), pos);
        }
    }

    /// `mask_buffer` stops applying masks at the first out-of-range index, so
    /// the index map must stop shifting at exactly the same point.
    #[test]
    fn masked_index_stops_where_mask_buffer_stops() {
        let buf: Vec<char> = "abc".chars().collect();
        let mask = PromptMask::from(BTreeMap::from([
            (1, "*".to_string()),
            (99, "!".to_string()),
            (2, "#".to_string()),
        ]));

        let masked: Vec<char> = mask.mask_buffer(&buf).chars().collect();
        assert_eq!(masked.iter().collect::<String>(), "a*b#c");

        for pos in 0..buf.len() {
            let mapped = mask.masked_index(pos, buf.len());
            assert_eq!(masked[mapped], buf[pos], "cursor {pos} -> {mapped}");
        }
        assert_eq!(mask.masked_index(buf.len(), buf.len()), masked.len());
    }

    /// Regression pin for PR #742: an out-of-range index must not panic. Note
    /// the behaviour it pins is that the remaining masks are *silently
    /// dropped* — the bounds check returns early rather than skipping the one
    /// bad entry.
    #[test]
    fn mask_past_end_drops_remainder_without_panicking() {
        let buf: Vec<char> = "abc".chars().collect();
        let mask = PromptMask::from(BTreeMap::from([
            (1, "*".to_string()),
            (99, "!".to_string()),
            (2, "#".to_string()),
        ]));

        // BTreeMap iterates in key order, so 1 and 2 are applied and the
        // out-of-range 99 ends the loop with nothing further applied.
        assert_eq!(mask.mask_buffer(&buf), "a*b#c");
    }
}
