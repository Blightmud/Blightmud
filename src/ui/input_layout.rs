//! Text arithmetic for the user input area: how a buffer divides into rows,
//! where the cursor lands, and how tall the area wants to be.
//!
//! This is separated from [`super::split_screen`] for the same reason
//! [`super::layout`] is: `SplitScreen::new` requires a real terminal, so
//! anything computed inside it cannot be exercised in CI. Every piece of
//! arithmetic lives here, where it is a pure function; the renderer stays a
//! blitter.
//!
//! Two coordinate spaces meet here and must not be confused:
//!
//! * **byte** offsets, which is what [`rows`] returns, because they slice the
//!   input directly;
//! * **char** indices, which is what `CommandBuffer` counts in, and therefore
//!   what [`cursor_row_col`] takes as `pos`.

use std::ops::Range;

use super::layout::{INPUT_HEIGHT_MAX, INPUT_HEIGHT_MIN};
use super::user_interface::wrap_line_hard;
use crate::tools::printable_chars::PrintableCharsIterator;

/// Partition `input` into the visual rows it occupies in a `width`-column
/// area, as byte ranges into `input`.
///
/// The ranges tile the input in order. `'\n'` characters are *not* part of any
/// range — they are row separators, not content — so the ranges do not cover
/// the whole string, but every non-newline byte belongs to exactly one.
///
/// Splitting on `'\n'` before wrapping is required, not stylistic:
/// `PrintableChars` drives a vte parser whose `Performer` implements only
/// `print`, so `'\n'` is a C0 control that is silently dropped. Measuring a
/// buffer containing one would silently fuse two logical rows into one.
pub fn rows(input: &str, width: u16) -> Vec<Range<usize>> {
    let width = width.max(1) as usize;
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut base = 0usize;

    for segment in input.split('\n') {
        let wrapped = wrap_line_hard(segment, width);
        let last = wrapped.len() - 1; // `wrap_line_hard` never returns empty
        let mut off = base;

        for (i, row) in wrapped.iter().enumerate() {
            out.push(off..off + row.len());
            off += row.len();

            // A logical row whose width is a nonzero exact multiple of the
            // terminal width needs a trailing empty visual row. Without it a
            // cursor sitting just past the last character addresses a row that
            // does not exist — and this is precisely where a terminal puts it,
            // since the cursor wraps to the next line rather than resting one
            // column off the right edge.
            if i == last && !row.is_empty() && row.display_width() >= width {
                out.push(off..off);
            }
        }

        base += segment.len() + 1; // `+ 1` for the '\n' that split consumed
    }

    out
}

/// Locate the flat **char** index `pos` as a `(row, column)` pair.
///
/// This is a lookup into [`rows`]'s partition rather than an independent
/// `cumulative_width / width` calculation, and that matters:
/// `byte_index_at_display_width` pushes a straddling double-width character
/// forward onto the next row and leaves a hole in the one it left. A parallel
/// reconstruction disagrees at every such hole by a full row, which renders
/// the cursor a line away from the character it is editing.
pub fn cursor_row_col(input: &str, pos: usize, width: u16) -> (u16, u16) {
    let rows = rows(input, width);
    let byte_off = char_to_byte(input, pos);

    // The last row that starts at or before the cursor. `rows` is ordered and
    // may contain empty ranges, so this is a scan rather than a search on
    // containment — an empty range contains nothing.
    let mut idx = 0;
    for (i, row) in rows.iter().enumerate() {
        if row.start <= byte_off {
            idx = i;
        } else {
            break;
        }
    }

    let row = &rows[idx];
    let end = byte_off.clamp(row.start, row.end);
    let before = &input[row.start..end];
    let col = before.display_width();

    (idx as u16, col as u16)
}

/// Byte offset of the `pos`-th character, saturating at the end of `input`.
fn char_to_byte(input: &str, pos: usize) -> usize {
    input
        .char_indices()
        .nth(pos)
        .map(|(i, _)| i)
        .unwrap_or(input.len())
}

/// How many rows the input area wants, given how many the content occupies.
///
/// `configured` is a *minimum*, not a fixed size: with auto-expand off it is
/// the height, and with it on the area grows past it as content requires and
/// shrinks back. One number, no mode enum.
pub fn desired_height(row_count: usize, configured: u16, auto_expand: bool) -> u16 {
    let configured = configured.clamp(INPUT_HEIGHT_MIN, INPUT_HEIGHT_MAX);
    if auto_expand {
        let rows = u16::try_from(row_count).unwrap_or(u16::MAX);
        rows.clamp(configured, INPUT_HEIGHT_MAX)
    } else {
        configured
    }
}

/// Which rows are on screen when the content is taller than the input area.
///
/// The cursor's row is always within the returned range; the window is pinned
/// to the bottom of the content once it reaches the end.
pub fn visible_rows(row_count: usize, cursor_row: usize, height: u16) -> Range<usize> {
    let height = height.max(INPUT_HEIGHT_MIN) as usize;
    if row_count <= height {
        return 0..row_count;
    }

    let max_start = row_count - height;
    let start = cursor_row.saturating_sub(height - 1).min(max_start);
    start..start + height
}

#[cfg(test)]
mod input_layout_test {
    use super::*;

    fn row_strs(input: &str, width: u16) -> Vec<&str> {
        rows(input, width)
            .into_iter()
            .map(|r| &input[r])
            .collect::<Vec<_>>()
    }

    #[test]
    fn plain_line_shorter_than_width_is_one_row() {
        assert_eq!(row_strs("hello", 20), vec!["hello"]);
    }

    #[test]
    fn empty_input_is_one_empty_row() {
        assert_eq!(row_strs("", 20), vec![""]);
    }

    #[test]
    fn newlines_start_new_rows_and_are_not_content() {
        assert_eq!(row_strs("a\nb\nc", 20), vec!["a", "b", "c"]);
        // A trailing newline opens an empty row the cursor can sit on.
        assert_eq!(row_strs("a\n", 20), vec!["a", ""]);
        // Consecutive newlines produce genuinely blank rows.
        assert_eq!(row_strs("a\n\nb", 20), vec!["a", "", "b"]);
    }

    #[test]
    fn long_line_wraps_losslessly() {
        assert_eq!(row_strs("abcdefgh", 3), vec!["abc", "def", "gh"]);
    }

    /// The guard the plan calls out: at an exact multiple of the width there
    /// must be a row for the cursor to wrap onto.
    #[test]
    fn exact_multiple_of_width_gets_a_trailing_empty_row() {
        assert_eq!(row_strs("abc", 3), vec!["abc", ""]);
        assert_eq!(row_strs("abcdef", 3), vec!["abc", "def", ""]);
        // ...but only at the end of a logical row, not at every break.
        assert_eq!(row_strs("abcd", 3), vec!["abc", "d"]);
        // ...and not for an empty row, which is already the cursor's row.
        assert_eq!(row_strs("", 3), vec![""]);
    }

    #[test]
    fn wide_characters_wrap_by_display_width_not_char_count() {
        // Each CJK glyph is two columns, so only two fit in five.
        assert_eq!(row_strs("中文中文", 5), vec!["中文", "中文"]);
    }

    /// A glyph wider than the whole area must not spin the wrap loop. Each
    /// glyph overflows its row by a column, and the trailing empty row still
    /// appears — the cursor cannot rest in the overflowed column, so it needs
    /// somewhere to go.
    #[test]
    fn glyph_wider_than_area_terminates() {
        assert_eq!(row_strs("中文", 1), vec!["中", "文", ""]);
        assert_eq!(cursor_row_col("中文", 2, 1), (2, 0));
    }

    /// Escape sequences occupy no columns and must not force a wrap.
    #[test]
    fn escapes_consume_no_columns() {
        let input = "\x1b[31mabcde\x1b[0m";
        assert_eq!(rows(input, 5).len(), 2); // content row + wrap row
        assert_eq!(&input[rows(input, 5)[0].clone()], input);
    }

    /// Every byte except the row separators belongs to exactly one row, and
    /// the rows are in order. This is the property the cursor depends on.
    #[test]
    fn rows_partition_the_input() {
        for input in [
            "",
            "a",
            "hello world",
            "a\nb",
            "a\n\nb\n",
            "\n\n\n",
            "中文test中文",
            "\x1b[31mred\x1b[0m text",
            "trailing   ",
        ] {
            for width in 1..=12u16 {
                let rows = rows(input, width);
                assert!(!rows.is_empty(), "{input:?} @ {width} produced no rows");

                let mut expected = 0usize;
                for row in &rows {
                    assert!(row.start >= expected, "rows out of order in {input:?}");
                    assert!(row.end >= row.start);
                    assert!(row.end <= input.len());
                    // Only a '\n' may be skipped between rows.
                    if row.start > expected {
                        assert_eq!(
                            &input[expected..row.start],
                            "\n",
                            "unexpected gap in {input:?} @ {width}"
                        );
                    }
                    expected = row.end;
                }

                let joined: String = rows.iter().map(|r| &input[r.clone()]).collect();
                assert_eq!(
                    joined,
                    input.replace('\n', ""),
                    "lost content in {input:?} @ {width}"
                );
            }
        }
    }

    /// `cursor_row_col` is asserted against `rows()` rather than hand-written
    /// expectations, so the two cannot drift apart. Every cursor position in
    /// every buffer at every width must land on a row that exists, at a column
    /// that row actually reaches.
    #[test]
    fn cursor_is_always_inside_a_real_row() {
        for input in [
            "",
            "a",
            "hello world",
            "a\nb\nc",
            "a\n\nb\n",
            "中文test中文",
            "\x1b[31mred\x1b[0m text",
            "abc",
            "abcdef",
        ] {
            for width in 1..=12u16 {
                let rows = rows(input, width);
                for pos in 0..=input.chars().count() {
                    let (row, col) = cursor_row_col(input, pos, width);

                    assert!(
                        (row as usize) < rows.len(),
                        "cursor row {row} out of range for {input:?} @ {width} pos {pos}"
                    );

                    let text = &input[rows[row as usize].clone()];
                    assert!(
                        col as usize <= text.display_width(),
                        "cursor col {col} past end of row {row} ({text:?}) \
                         for {input:?} @ {width} pos {pos}"
                    );
                }
            }
        }
    }

    #[test]
    fn cursor_at_start_is_row_zero_column_zero() {
        assert_eq!(cursor_row_col("hello", 0, 20), (0, 0));
        assert_eq!(cursor_row_col("", 0, 20), (0, 0));
    }

    #[test]
    fn cursor_tracks_logical_rows() {
        let input = "ab\ncd";
        assert_eq!(cursor_row_col(input, 2, 20), (0, 2)); // end of "ab"
        assert_eq!(cursor_row_col(input, 3, 20), (1, 0)); // start of "cd"
        assert_eq!(cursor_row_col(input, 5, 20), (1, 2)); // end of "cd"
    }

    #[test]
    fn cursor_wraps_onto_the_trailing_empty_row() {
        // "abc" exactly fills a 3-column area; the cursor after it belongs on
        // the next row, which is why that row is generated at all.
        assert_eq!(cursor_row_col("abc", 3, 3), (1, 0));
        assert_eq!(cursor_row_col("abc", 2, 3), (0, 2));
    }

    #[test]
    fn cursor_column_counts_display_width() {
        // Two CJK glyphs are two chars but four columns.
        assert_eq!(cursor_row_col("中文test", 2, 20), (0, 4));
    }

    /// A position past the end of the buffer must clamp, not panic.
    #[test]
    fn cursor_past_end_clamps() {
        let (row, col) = cursor_row_col("abc", 99, 20);
        assert_eq!((row, col), (0, 3));
    }

    #[test]
    fn desired_height_is_the_configured_value_when_not_expanding() {
        assert_eq!(desired_height(1, 3, false), 3);
        assert_eq!(desired_height(9, 3, false), 3);
        assert_eq!(desired_height(1, 1, false), 1);
    }

    #[test]
    fn desired_height_grows_and_shrinks_when_expanding() {
        assert_eq!(desired_height(1, 1, true), 1);
        assert_eq!(desired_height(4, 1, true), 4);
        // Never below the configured minimum...
        assert_eq!(desired_height(1, 3, true), 3);
        // ...and never above the hard maximum.
        assert_eq!(desired_height(500, 1, true), INPUT_HEIGHT_MAX);
        assert_eq!(desired_height(usize::MAX, 1, true), INPUT_HEIGHT_MAX);
    }

    #[test]
    fn desired_height_clamps_a_nonsense_configuration() {
        assert_eq!(desired_height(1, 0, false), INPUT_HEIGHT_MIN);
        assert_eq!(desired_height(1, 500, false), INPUT_HEIGHT_MAX);
    }

    #[test]
    fn all_rows_visible_when_they_fit() {
        assert_eq!(visible_rows(3, 0, 5), 0..3);
        assert_eq!(visible_rows(3, 2, 3), 0..3);
        assert_eq!(visible_rows(0, 0, 3), 0..0);
    }

    #[test]
    fn visible_window_follows_the_cursor() {
        // 10 rows in a 3-row area.
        assert_eq!(visible_rows(10, 0, 3), 0..3);
        assert_eq!(visible_rows(10, 2, 3), 0..3);
        assert_eq!(visible_rows(10, 3, 3), 1..4);
        assert_eq!(visible_rows(10, 9, 3), 7..10);
    }

    /// The window must contain the cursor for every combination, or the user
    /// types into a row they cannot see.
    #[test]
    fn visible_window_always_contains_the_cursor() {
        for row_count in 1..30usize {
            for height in 1..12u16 {
                for cursor in 0..row_count {
                    let window = visible_rows(row_count, cursor, height);
                    assert!(
                        window.contains(&cursor),
                        "cursor {cursor} outside {window:?} \
                         (rows {row_count}, height {height})"
                    );
                    assert!(window.end <= row_count);
                    assert!(window.len() <= height as usize);
                }
            }
        }
    }
}
