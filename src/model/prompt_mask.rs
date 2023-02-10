use mlua::{Integer as LuaInt, Lua, Result as LuaResult, String as LuaString, Table as LuaTable};
use std::collections::BTreeMap;
use std::ops::{AddAssign, Deref};

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
            masked_buf.splice(adjusted_idx..adjusted_idx, mask.chars());
            offset += mask.len();
        }

        masked_buf.iter().collect()
    }

    pub fn to_table<'a>(&'a self, ctx: &'a Lua) -> LuaResult<LuaTable> {
        let table = ctx.create_table()?;
        for (idx, mask) in self.iter() {
            let adjusted_idx = *idx + 1;
            table.set(adjusted_idx, (*mask).clone())?;
        }
        Ok(table)
    }
}

impl Deref for PromptMask {
    type Target = BTreeMap<i32, String>;

    fn deref(&self) -> &Self::Target {
        &self.mask
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

impl From<LuaTable<'_>> for PromptMask {
    fn from(mask_table: LuaTable) -> Self {
        let mut mask = BTreeMap::new();
        for pair in mask_table.pairs::<LuaInt, LuaString>() {
            let (offset, marker) = pair.unwrap();
            // Lua is 1-indexed, we handle that here as part of the conversion from LuaTable.
            // so that the Rust code can use natural 0-indexing.
            let adjusted_offset = (offset as i32) - 1;
            mask.insert(adjusted_offset as i32, marker.to_str().unwrap().to_string());
        }
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
        let mut expected = BTreeMap::new();
        // We expect the 1-indexed Lua table indices to have been translated to 0-indexing.
        expected.insert(0, "*".to_string());
        expected.insert(4, "!".to_string());

        let lua_mask = PromptMask::from(simple_mask);
        let expected_mask = PromptMask::from(expected);
        assert_eq!(lua_mask, expected_mask)
    }

    #[test]
    fn test_add_assign() {
        let mut mask_map_a = BTreeMap::new();
        mask_map_a.insert(10, "*".to_string());
        mask_map_a.insert(15, "#".to_string());
        mask_map_a.insert(20, "!".to_string());
        let mut mask_a = PromptMask::from(mask_map_a);

        let mut mask_map_b = BTreeMap::new();
        mask_map_b.insert(1, "@".to_string());
        mask_map_b.insert(15, "%".to_string());
        mask_map_b.insert(25, "&".to_string());
        let mask_b = PromptMask::from(mask_map_b);

        let mut expected_map = BTreeMap::new();
        expected_map.insert(1, "@".to_string());
        expected_map.insert(10, "*".to_string());
        expected_map.insert(15, "%".to_string()); // NB: overwritten by map_b.
        expected_map.insert(20, "!".to_string());
        expected_map.insert(25, "&".to_string());
        let expected = PromptMask::from(expected_map);

        mask_a += mask_b;
        assert_eq!(mask_a, expected)
    }

    #[test]
    fn test_masking() {
        let buf = vec![
            't', 'h', 'i', 's', ' ', 'i', 's', ' ', 'i', 'm', 'p', 'o', 'r', 't', 'a', 'n', 't',
            ',', ' ', 'o', 'k',
        ];
        let mut mask_map = BTreeMap::new();
        mask_map.insert(8, "*".to_string());
        mask_map.insert(17, "*".to_string());
        let mask = PromptMask::from(mask_map);

        let res = mask.mask_buffer(&buf);
        assert_eq!(res, "this is *important*, ok")
    }
}
