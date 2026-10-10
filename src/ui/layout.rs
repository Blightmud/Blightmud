//! Vertical screen layout arithmetic for [`super::SplitScreen`].
//!
//! `SplitScreen::new` needs a real terminal, so anything computed inside it
//! cannot be exercised in CI. The row arithmetic lives here instead, as a pure
//! function of the terminal height and the configured region sizes, so it can
//! be tested directly.
//!
//! The screen is divided top to bottom:
//!
//! ```text
//! 1                    ┐
//! ..                   │ top area (`top_rows` rows, may be 0)
//! top_rows             ┘
//! output_start_line    ┐
//! ..                   │ output / scroll region (DECSTBM)
//! output_line          ┘
//! mud_prompt_line        the MUD's own prompt (always exactly 1 row)
//! ..                   ┐ status area (`status_height` rows, may be 0)
//! ..                   ┘
//! input_start_line     ┐
//! ..                   │ user input area (`input_height` rows, at least 1)
//! term_height          ┘
//! ```

/// The scroll region is programmed with DECSTBM (`CSI Pt ; Pb r`), which
/// requires `Pt < Pb` — a region must span at least two rows. A terminal
/// silently *discards* an out-of-range request rather than reporting an error,
/// leaving whatever region was previously in effect; output written afterwards
/// then scrolls the whole screen and drags the status and input areas with it.
/// So a layout that cannot give the output region two rows is not renderable at
/// all, and must be rejected rather than clamped.
pub const MIN_OUTPUT_ROWS: u16 = 2;

/// An input area is never zero rows — there would be nowhere to type.
pub const INPUT_HEIGHT_MIN: u16 = 1;

/// Upper bound on a user-requested input height, mirroring `STATUS_HEIGHT_MAX`.
pub const INPUT_HEIGHT_MAX: u16 = 10;

/// Absolute, 1-based terminal row numbers for each region boundary.
///
/// Produced only by [`ScreenLayout::compute`], which guarantees the invariants
/// the renderer relies on — most importantly that the output region is valid to
/// program as a scroll region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenLayout {
    /// First row of the output/scroll region.
    pub output_start_line: u16,
    /// Last row of the output/scroll region.
    pub output_line: u16,
    /// The row carrying the MUD's own prompt.
    pub mud_prompt_line: u16,
    /// First row of the user input area.
    pub input_start_line: u16,
    /// Rows granted to the input area. May be less than requested.
    pub input_height: u16,
    /// Rows granted to the status area. May be less than requested.
    pub status_height: u16,
}

impl ScreenLayout {
    /// Divide a terminal `term_height` rows tall between the regions.
    ///
    /// `input_height` and `status_height` are *requests*. When the terminal is
    /// too short to honour them, they are reduced in that order — the input
    /// area shrinks toward [`INPUT_HEIGHT_MIN`] first, then the status area
    /// toward zero — so that the output region keeps [`MIN_OUTPUT_ROWS`].
    ///
    /// Returns `None` when even the minimal layout does not fit, which the
    /// caller must treat as "do not render", not as "render something".
    pub fn compute(
        term_height: u16,
        top_rows: u16,
        status_height: u16,
        input_height: u16,
    ) -> Option<Self> {
        let output_start_line = top_rows.checked_add(1)?;

        // Rows left for the output region, the MUD prompt, the status area and
        // the input area once the top area has taken its share.
        let budget = term_height.checked_sub(top_rows)?;

        // The MUD prompt row, the minimum output region and the one row the
        // input area is always entitled to are not negotiable; whatever remains
        // is what the two configurable regions may share.
        let fixed = MIN_OUTPUT_ROWS + 1 + INPUT_HEIGHT_MIN;
        let spare = budget.checked_sub(fixed)?;

        let mut input_height = input_height.clamp(INPUT_HEIGHT_MIN, INPUT_HEIGHT_MAX);
        let mut status_height = status_height;

        // Only the rows *beyond* the guaranteed minimum are drawn from `spare`.
        if input_height - INPUT_HEIGHT_MIN + status_height > spare {
            // Shrink the input area first, down to its minimum.
            let overflow = input_height - INPUT_HEIGHT_MIN + status_height - spare;
            let give = overflow.min(input_height - INPUT_HEIGHT_MIN);
            input_height -= give;

            // Then the status area, down to nothing.
            let overflow = overflow - give;
            status_height = status_height.saturating_sub(overflow);
        }

        // `spare` already reserved MIN_OUTPUT_ROWS, so the output region grows
        // into whatever the configurable regions did not take.
        let output_rows =
            MIN_OUTPUT_ROWS + spare - (input_height - INPUT_HEIGHT_MIN) - status_height;

        let output_line = output_start_line + output_rows - 1;
        let mud_prompt_line = output_line + 1;
        let input_start_line = mud_prompt_line + status_height + 1;

        let layout = Self {
            output_start_line,
            output_line,
            mud_prompt_line,
            input_start_line,
            input_height,
            status_height,
        };

        debug_assert!(layout.is_scroll_region_valid());
        debug_assert_eq!(
            layout.input_start_line + layout.input_height - 1,
            term_height,
            "input area must end on the last terminal row"
        );

        Some(layout)
    }

    /// Whether the output region may be programmed with DECSTBM.
    ///
    /// [`Self::compute`] never produces a layout for which this is false; it is
    /// exposed so the renderer can assert the invariant at the point of use.
    pub fn is_scroll_region_valid(&self) -> bool {
        self.output_line > self.output_start_line
    }

    /// Number of rows in the output region.
    #[cfg(test)]
    pub fn output_rows(&self) -> u16 {
        self.output_line - self.output_start_line + 1
    }
}

#[cfg(test)]
mod layout_test {
    use super::*;

    /// A single-row input area must produce exactly:
    ///   output_line      = height - status - 2
    ///   mud_prompt_line  = height - status - 1
    ///   input_start_line = height
    /// with `output_start_line = top_rows + 1`.
    #[test]
    fn single_row_input_layout_boundaries() {
        for height in [24_u16, 40, 50, 80] {
            for top_rows in [1_u16, 2] {
                for status in [0_u16, 1, 3, 5] {
                    let layout = ScreenLayout::compute(height, top_rows, status, 1).unwrap();
                    assert_eq!(layout.output_start_line, top_rows + 1);
                    assert_eq!(layout.output_line, height - status - 2);
                    assert_eq!(layout.mud_prompt_line, height - status - 1);
                    assert_eq!(layout.input_start_line, height);
                    assert_eq!(layout.status_height, status);
                }
            }
        }
    }

    /// A terminal too short for even the minimal layout is rejected.
    #[test]
    fn short_terminal_does_not_underflow() {
        assert!(ScreenLayout::compute(3, 1, 1, 1).is_none());
        assert!(ScreenLayout::compute(2, 1, 0, 1).is_none());
        assert!(ScreenLayout::compute(1, 1, 0, 1).is_none());
        assert!(ScreenLayout::compute(0, 0, 0, 1).is_none());
    }

    /// A 6-row terminal with a full status area must still leave a valid
    /// output region.
    #[test]
    fn status_height_five_on_six_row_terminal_leaves_output() {
        let layout = ScreenLayout::compute(6, 1, 5, 1).unwrap();
        assert!(layout.is_scroll_region_valid());
        assert!(layout.output_rows() >= MIN_OUTPUT_ROWS);
        // The status area gave up rows so the output region stayed valid.
        assert!(layout.status_height < 5);
    }

    /// Precedence: the input area yields before the status area does.
    #[test]
    fn input_height_squeezes_before_status_starves() {
        // 12 rows, 1 top row, status 3, input 5 -> needs 1+2+1+3+5 = 12, fits.
        let layout = ScreenLayout::compute(12, 1, 3, 5).unwrap();
        assert_eq!(layout.input_height, 5);
        assert_eq!(layout.status_height, 3);

        // One row tighter: the input area is what gives.
        let layout = ScreenLayout::compute(11, 1, 3, 5).unwrap();
        assert_eq!(layout.input_height, 4);
        assert_eq!(layout.status_height, 3);

        // Tighter still: input bottoms out at 1, then status starts yielding.
        let layout = ScreenLayout::compute(8, 1, 3, 5).unwrap();
        assert_eq!(layout.input_height, INPUT_HEIGHT_MIN);
        assert_eq!(layout.status_height, 3);

        let layout = ScreenLayout::compute(7, 1, 3, 5).unwrap();
        assert_eq!(layout.input_height, INPUT_HEIGHT_MIN);
        assert_eq!(layout.status_height, 2);
    }

    /// The regions must tile the terminal exactly, with no gap and no overlap,
    /// for every layout `compute` is willing to return.
    #[test]
    fn regions_tile_the_terminal_exactly() {
        for height in 0_u16..64 {
            for top_rows in 0_u16..4 {
                for status in 0_u16..6 {
                    for input in 1_u16..12 {
                        let Some(layout) = ScreenLayout::compute(height, top_rows, status, input)
                        else {
                            continue;
                        };

                        assert_eq!(layout.output_start_line, top_rows + 1);
                        assert_eq!(layout.mud_prompt_line, layout.output_line + 1);
                        assert_eq!(
                            layout.input_start_line,
                            layout.mud_prompt_line + layout.status_height + 1
                        );
                        assert_eq!(layout.input_start_line + layout.input_height - 1, height);

                        assert!(layout.input_height >= INPUT_HEIGHT_MIN);
                        assert!(layout.input_height <= input.max(INPUT_HEIGHT_MIN));
                        assert!(layout.status_height <= status);
                    }
                }
            }
        }
    }

    /// The reason `compute` returns `Option` at all: a scroll region with
    /// `bottom <= top` is discarded by the terminal, which is worse than not
    /// rendering, because the previous region silently stays in force.
    #[test]
    fn scroll_region_is_never_emitted_with_bottom_le_top() {
        for height in 0_u16..64 {
            for top_rows in 0_u16..4 {
                for status in 0_u16..6 {
                    for input in 1_u16..12 {
                        if let Some(layout) = ScreenLayout::compute(height, top_rows, status, input)
                        {
                            assert!(
                                layout.is_scroll_region_valid(),
                                "invalid scroll region {}..{} from ({height}, {top_rows}, {status}, {input})",
                                layout.output_start_line,
                                layout.output_line,
                            );
                            assert!(layout.output_rows() >= MIN_OUTPUT_ROWS);
                        }
                    }
                }
            }
        }
    }

    /// A request above the maximum is clamped, not rejected.
    #[test]
    fn input_height_clamps_to_max() {
        let layout = ScreenLayout::compute(80, 1, 1, 500).unwrap();
        assert_eq!(layout.input_height, INPUT_HEIGHT_MAX);
    }

    /// A zero request is raised to the minimum rather than leaving nowhere to
    /// type.
    #[test]
    fn input_height_zero_is_raised_to_minimum() {
        let layout = ScreenLayout::compute(80, 1, 1, 0).unwrap();
        assert_eq!(layout.input_height, INPUT_HEIGHT_MIN);
    }
}
