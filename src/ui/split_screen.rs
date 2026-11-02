use super::history::History;
use super::input_layout;
use super::layout::{ScreenLayout, INPUT_HEIGHT_MIN};
use super::scroll_data::ScrollData;
use super::top_area::{
    self, row_names, TopArea, TopPrefix, TopPrefixStyle, TopRenderContext, TopRowBody,
};
use super::user_interface::TerminalSizeError;
use super::wrap_line;
use crate::io::SaveData;
use crate::model::{
    Settings, HIDE_TOPBAR, TAB_INDICATOR_BRAND, TAB_INDICATOR_INLINE, TAB_INDICATOR_VISIBLE,
};
use crate::tabs::TabInfo;
use crate::{
    model::Line, model::Regex, model::TagMask, model::ToLine,
    tools::printable_chars::PrintableCharsIterator, ui::ansi::*,
};
use anyhow::Result;
use std::collections::HashSet;
use std::io::Write;
use termion::color::{self, Bg, Fg};
use termion::cursor;

use super::UserInterface;

const SCROLL_LIVE_BUFFER_SIZE: u16 = 10;
const PROMPT_HEIGHT: u16 = 1;
const STATUS_HEIGHT_MIN: u16 = 0;
const STATUS_HEIGHT_MAX: u16 = 5;

struct StatusArea {
    start_line: u16,
    width: u16,
    status_lines: Vec<Option<String>>,
    scroll_marker: bool,
}

impl StatusArea {
    fn new(height: u16, start_line: u16, width: u16) -> Self {
        let height = Self::clamp_height(height);
        Self {
            start_line,
            width,
            status_lines: vec![None; height],
            scroll_marker: false,
        }
    }

    fn set_scroll_marker(&mut self, value: bool) {
        self.scroll_marker = value;
    }

    fn clamp_height(height: u16) -> usize {
        height.clamp(STATUS_HEIGHT_MIN, STATUS_HEIGHT_MAX) as usize
    }

    fn clamp_index(&self, index: usize) -> usize {
        index.clamp(0, self.status_lines.len() - 1)
    }

    fn set_height(&mut self, height: u16, start_line: u16) {
        self.clear();
        self.status_lines.resize(Self::clamp_height(height), None);
        self.update_pos(start_line);
    }

    fn update_pos(&mut self, start_line: u16) {
        self.start_line = start_line;
    }

    fn set_width(&mut self, width: u16) {
        self.width = width;
    }

    fn set_status_line(&mut self, index: usize, line: String) {
        let index = self.clamp_index(index);
        if !line.trim().is_empty() {
            self.status_lines[index] = Some(line);
        } else {
            self.status_lines[index] = None;
        }
    }

    fn clear(&mut self) {
        self.status_lines = vec![None; self.status_lines.len()];
    }

    fn redraw_line(&mut self, screen: &mut impl Write, line_no: usize) -> Result<()> {
        let line_no = self.clamp_index(line_no);
        let index = self.start_line as usize + line_no;

        let mut info = if self.scroll_marker && line_no == 0 {
            "(more) ".to_string()
        } else {
            String::new()
        };

        if let Some(Some(custom_info)) = self.status_lines.get(line_no) {
            info = if info.is_empty() {
                custom_info.to_string()
            } else {
                format!("{info}━ {custom_info} ")
            };
        }

        if line_no == 0 || line_no == self.status_lines.len() - 1 {
            top_area::draw_bar('━', self.width as usize, index, screen, &info)?;
        } else {
            self.draw_line(index, screen, &info)?;
        }

        Ok(())
    }

    fn redraw(&mut self, screen: &mut impl Write) -> Result<()> {
        for line in 0..self.status_lines.len() {
            self.redraw_line(screen, line)?;
        }
        Ok(())
    }

    fn draw_line(&self, line: usize, screen: &mut impl Write, info: &str) -> Result<()> {
        write!(
            screen,
            "{}{}",
            termion::cursor::Goto(1, line as u16),
            termion::clear::CurrentLine,
        )?;

        write!(screen, "{info}")?; // Print separator
        Ok(())
    }

    fn height(&self) -> u16 {
        self.status_lines.len() as u16
    }
}

pub struct SplitScreen {
    screen: Box<dyn Write>,
    width: u16,
    height: u16,
    output_start_line: u16,
    output_line: u16,
    mud_prompt_line: u16,
    mud_prompt: Line,
    prompt_line: u16,
    status_area: StatusArea,
    cursor_prompt_pos: u16,
    /// Cursor row *within* the input area, counted from `prompt_line`. Always
    /// zero while the input area is one row tall.
    cursor_prompt_row: u16,
    /// Rows granted to the user input area. Pinned at [`INPUT_HEIGHT_MIN`]
    /// until the height plumbing lands; the multi-row render path below is
    /// therefore compiled but not yet reachable.
    input_height: u16,
    /// Whether the input area grows past `input_height` as content requires.
    input_auto_expand: bool,
    history: History,
    scroll_data: ScrollData,
    connection: Option<String>,
    tags: HashSet<String>,
    prompt_input: String,
    prompt_input_pos: usize,
    show_tags: bool,
    tag_mask: TagMask,
    /// The ordered list of top-area rows. Subsumes the legacy
    /// `tab_indicator_line` / `topbar_line` / `tab_indicator_brand` /
    /// `tab_indicator_inline` / `top_line` fields — the visibility,
    /// prefix style, and body content of each row carry the same
    /// information in a uniform structure. See [`super::top_area`].
    top_area: TopArea,
    /// Snapshot of the current tab set, set by `set_tab_indicator`. The
    /// `top_area` reads this via the render context. Empty Vec ≡ "only
    /// the implicit `main` tab" and suppresses the indicator row.
    tabs_metadata: Vec<TabInfo>,
    /// Cached at `setup()` from the `tab_indicator_inline` setting. When
    /// `true` and the topbar is visible, tabs render alongside the
    /// host_status row's body instead of on a dedicated row above it.
    inline_tabs_setting: bool,
}

impl UserInterface for SplitScreen {
    fn setup(&mut self) -> Result<()> {
        self.reset()?;

        let settings = Settings::try_load()?;

        // Get params in case screen resized
        let (width, height) = termion::terminal_size()?;
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;

            // Reconcile built-in top row config with current settings.
            // Each visible row consumes one screen line above the output
            // area. Inline tabs mode hides the dedicated indicator row
            // and decorates the host_status row at render time instead.
            let hide_topbar = settings.get(HIDE_TOPBAR)?;
            let brand = settings.get(TAB_INDICATOR_BRAND)?;
            self.inline_tabs_setting = settings.get(TAB_INDICATOR_INLINE)?;
            let inline_with_topbar = self.inline_tabs_setting && !hide_topbar;
            let show_indicator = settings.get(TAB_INDICATOR_VISIBLE)?
                && self.tabs_metadata.len() > 1
                && !inline_with_topbar;

            if let Some(idx) = self.top_area.find(row_names::TAB_INDICATOR) {
                if let Some(row) = self.top_area.get_mut(idx) {
                    row.visible = show_indicator;
                    row.prefix = Some(if brand {
                        TopPrefix {
                            text: " Blightmud ".to_string(),
                            style: TopPrefixStyle::Brand,
                        }
                    } else {
                        TopPrefix {
                            text: String::new(),
                            style: TopPrefixStyle::Plain,
                        }
                    });
                }
            }
            if let Some(idx) = self.top_area.find(row_names::HOST_STATUS) {
                if let Some(row) = self.top_area.get_mut(idx) {
                    row.visible = !hide_topbar;
                }
            }
            // The top area's height is only known once the rows above have
            // been reconciled, and it is an input to every other boundary.
            let layout = ScreenLayout::compute(
                height,
                self.top_area.visible_row_count(),
                self.status_area.height(),
                INPUT_HEIGHT_MIN,
            )
            .ok_or(TerminalSizeError)?;

            self.output_start_line = layout.output_start_line;
            self.output_line = layout.output_line;
            self.mud_prompt_line = layout.mud_prompt_line;
            self.prompt_line = layout.input_start_line;

            // The layout may have taken rows away from the status area to keep
            // the output region renderable.
            if layout.status_height != self.status_area.height() {
                self.status_area
                    .set_height(layout.status_height, self.mud_prompt_line + 1);
            } else {
                self.status_area.update_pos(self.mud_prompt_line + 1);
            }

            write!(
                self.screen,
                "{}{}",
                ScrollRegion(self.output_start_line, self.output_line),
                DisableOriginMode
            )
            .unwrap(); // Set scroll region, non origin mode
            self.redraw_top_area()?;
            self.reset_scroll()?;
            self.redraw_status_area()?;
            self.screen.flush()?;
            write!(
                self.screen,
                "{}{}",
                termion::cursor::Goto(1, self.output_start_line),
                termion::cursor::Save
            )?;
            Ok(())
        } else {
            Err(TerminalSizeError.into())
        }
    }

    fn print_error(&mut self, output: &str) {
        let line = &format!("{}[!!] {}{}", Fg(color::Red), output, Fg(color::Reset));
        self.print_line(line.to_internal_line());
    }

    fn print_info(&mut self, output: &str) {
        let line = &format!("[**] {output}");
        self.print_line(line.to_internal_line());
    }

    fn print_output(&mut self, line: &Line) {
        //debug!("UI: {:?}", line);
        // Handle screen clear request from server
        if line.flags.screen_clear {
            self.clear_output_area().ok();
        }
        let raw = match line.print_line() {
            Some(r) => r,
            None => return,
        };
        if !line.is_utf8() || raw.trim().is_empty() {
            self.print_line(line.clone());
        } else {
            let padding = if self.show_tags { 2 } else { 0 };
            let segments: Vec<String> = wrap_line(raw, self.width as usize, padding)
                .into_iter()
                .map(str::to_string)
                .collect();
            let count = segments.len();
            let cur_line = self.history.len();
            for segment in segments {
                let mut entry = line.clone();
                entry.set_content(&segment);
                self.print_line(entry);
            }
            if self.scroll_data.scroll_lock && count > self.height as usize {
                self.scroll_to(cur_line).ok();
            }
        }
    }

    fn print_prompt(&mut self, prompt: &Line) {
        //debug!("UI: {:?}", prompt);
        self.mud_prompt = prompt.clone();
        self.redraw_prompt();
    }

    fn print_prompt_input(&mut self, input: &str, pos: usize) {
        // Sanity check: pos is a character index
        debug_assert!(pos <= input.chars().count());

        self.prompt_input = input.to_string();
        self.prompt_input_pos = pos;

        if self.input_height > INPUT_HEIGHT_MIN {
            self.print_prompt_input_rows(input, pos);
            return;
        }

        // Single-row path: scrolls horizontally behind a '>' instead of
        // wrapping.
        self.cursor_prompt_row = 0;

        // `pos` is a character index, not a display column.
        let byte_idx_at_cursor = input
            .char_indices()
            .nth(pos)
            .map(|(idx, _)| idx)
            .unwrap_or(input.len());
        let mut cursor_display_pos = (&input[..byte_idx_at_cursor]).display_width();

        let mut input = input;
        let width = self.width as usize;
        let mut wrapped = false;

        // Scroll the view when cursor goes past the visible width
        while input.display_width() >= width && cursor_display_pos >= width {
            wrapped = true;
            let (byte_idx, skipped_width) = input.byte_index_at_display_width(width);
            if byte_idx < input.len() {
                input = input.split_at(byte_idx).1;
                cursor_display_pos -= skipped_width;
            } else {
                input = "";
                cursor_display_pos = 0;
            }
        }

        // When wrapped, reserve 1 column for the '>' indicator
        let effective_width = if wrapped { width - 1 } else { width };

        // Truncate input if it's still too wide for the display
        if input.display_width() >= effective_width {
            let (byte_idx, _) = input.byte_index_at_display_width(effective_width);
            input = input.split_at(byte_idx).0;
        }

        // Adjust cursor position for the wrap indicator
        let cursor_offset = if wrapped { 2 } else { 1 };
        self.cursor_prompt_pos = cursor_display_pos as u16 + cursor_offset;

        let wrap_indicator = if wrapped { ">" } else { "" };
        write!(
            self.screen,
            "{}{}{}{}{}{}{}{}",
            termion::cursor::Goto(1, self.prompt_line),
            Fg(termion::color::Reset),
            Bg(termion::color::Reset),
            termion::style::Reset,
            termion::clear::CurrentLine,
            wrap_indicator,
            input,
            self.goto_prompt(),
        )
        .unwrap();
    }

    fn print_send(&mut self, send: &Line) {
        if self.scroll_data.active && send.flags.source != Some("script".to_string()) {
            self.reset_scroll().ok();
        }
        if let Some(line) = send.print_line() {
            let line = &format!(
                "{}{}> {}{}",
                termion::style::Reset,
                Fg(color::LightYellow),
                line,
                Fg(color::Reset),
            );
            for line in wrap_line(
                line,
                self.width as usize,
                if self.show_tags { 2 } else { 0 },
            ) {
                self.print_line(line.to_internal_line());
            }
        }
    }

    fn reset(&mut self) -> Result<()> {
        write!(self.screen, "{}{}", termion::clear::All, ResetScrollRegion)?;
        Ok(())
    }

    fn reset_scroll(&mut self) -> Result<()> {
        let reset_split = self.scroll_data.split;
        let reset_scroll = self.scroll_data.active;
        self.scroll_data.reset(&self.history)?;
        if reset_split {
            write!(self.screen, "{ResetScrollRegion}")?;
            write!(
                self.screen,
                "{}{}",
                ScrollRegion(self.output_start_line, self.output_line),
                DisableOriginMode
            )?;
        } else if reset_scroll {
            self.status_area.set_scroll_marker(false);
            self.status_area.redraw_line(&mut self.screen, 0)?;
        }
        self.redraw_prompt();

        let output_range = self.output_range();
        let output_start_index = self.history.len() as i32 - output_range as i32;
        if output_start_index >= 0 {
            let output_start_index = output_start_index as usize;
            for i in 0..output_range {
                let index = output_start_index + i as usize;
                let line_no = self.output_start_line + i;
                let rendered = self.render_history_line(index);
                write!(
                    self.screen,
                    "{}{}{}",
                    termion::cursor::Goto(1, line_no),
                    termion::clear::CurrentLine,
                    rendered,
                )?;
            }
        } else {
            for i in 0..self.history.len() {
                let rendered = self.render_history_line(i);
                write!(
                    self.screen,
                    "{}\n{}",
                    termion::cursor::Goto(1, self.output_line),
                    rendered,
                )?;
            }
        }
        Ok(())
    }

    fn clear_output_area(&mut self) -> Result<()> {
        // Clear all lines in the output scroll region
        for line_no in self.output_start_line..=self.output_line {
            write!(
                self.screen,
                "{}{}",
                termion::cursor::Goto(1, line_no),
                termion::clear::CurrentLine,
            )?;
        }
        // Clear the history buffer as well
        self.history.clear();
        // Reset scroll state
        self.scroll_data.reset(&self.history)?;
        // Reposition cursor
        write!(
            self.screen,
            "{}{}",
            termion::cursor::Goto(1, self.output_start_line),
            self.goto_prompt(),
        )?;
        Ok(())
    }

    fn scroll_down(&mut self) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        if self.scroll_data.active {
            let output_range = self.scroll_range() as i32;
            let max_start_index: i32 = self.history.len() as i32 - output_range;
            let new_start_index = self.scroll_data.pos + 5;
            if new_start_index >= max_start_index as usize {
                self.reset_scroll()?;
            } else {
                self.scroll_data.pos = new_start_index;
                self.draw_scroll()?;
            }
        }
        Ok(())
    }

    fn scroll_lock(&mut self, lock: bool) -> Result<()> {
        self.scroll_data.lock(lock)
    }

    fn scroll_to(&mut self, row: usize) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        if self.history.len() > self.scroll_range() as usize {
            let max_start_index = self.history.len() as i32 - self.scroll_range() as i32;
            if max_start_index > 0 && row < max_start_index as usize {
                self.init_scroll()?;
                self.scroll_data.pos = row;
                self.draw_scroll()?;
            } else {
                self.reset_scroll()?;
            }
        }
        Ok(())
    }

    fn scroll_top(&mut self) -> Result<()> {
        if self.history.len() as u16 >= self.output_line {
            self.init_scroll()?;
            self.scroll_data.pos = 0;
            self.draw_scroll()?;
        }
        Ok(())
    }

    fn scroll_up(&mut self) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        let output_range: usize = self.scroll_range() as usize;
        if self.history.len() > output_range {
            if !self.scroll_data.active {
                self.init_scroll()?;
                self.scroll_data.pos = self.history.len() - output_range;
            }
            self.scroll_data.pos -= self.scroll_data.pos.min(5);
            self.draw_scroll()?;
        }
        Ok(())
    }

    fn find_up(&mut self, pattern: &Regex) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        let pos = if self.scroll_data.active {
            self.scroll_data.pos
        } else if self.history.len() > self.scroll_range() as usize {
            self.history.len() - self.scroll_range() as usize
        } else {
            self.history.len()
        };
        if let Some(line) = self.history.find_backward(pattern, pos) {
            self.scroll_data.hilite = Some(pattern.clone());
            self.scroll_to(0.max(line))?;
        }
        Ok(())
    }

    fn find_down(&mut self, pattern: &Regex) -> Result<()> {
        self.scroll_data.clamp(&self.history);
        if self.scroll_data.active {
            if let Some(line) = self
                .history
                .find_forward(pattern, self.history.len().min(self.scroll_data.pos + 1))
            {
                self.scroll_data.hilite = Some(pattern.clone());
                self.scroll_to(line.min(self.history.len() - 1))?;
            }
        }
        Ok(())
    }

    fn set_host(&mut self, host: &str, port: u16) -> Result<()> {
        self.connection = if !host.is_empty() {
            Some(format!("{host}:{port}"))
        } else {
            None
        };
        self.redraw_top_area()
    }

    fn add_tag(&mut self, tag: &str) -> Result<()> {
        self.tags.insert(tag.to_string());
        self.redraw_top_area()
    }

    fn remove_tag(&mut self, tag: &str) -> Result<()> {
        self.tags.remove(tag);
        self.redraw_top_area()
    }

    fn clear_tags(&mut self) -> Result<()> {
        self.tags.clear();
        self.redraw_top_area()
    }

    fn set_status_area_height(&mut self, height: u16) -> Result<()> {
        let height = StatusArea::clamp_height(height) as u16;
        self.status_area
            .set_height(height, self.height - height - PROMPT_HEIGHT);
        self.setup()?;
        let input_str = self.prompt_input.as_str().to_owned();
        self.print_prompt_input(&input_str, self.prompt_input_pos);
        Ok(())
    }

    fn set_show_tags(&mut self, show: bool) -> Result<()> {
        self.show_tags = show;
        self.setup()
    }

    fn set_tag_mask(&mut self, mask: TagMask) {
        self.tag_mask = mask.clone();
        self.history.set_tag_mask(mask);
        self.setup().ok();
    }

    fn set_history_capacity(&mut self, capacity: usize) {
        self.history.set_capacity(capacity);
    }

    fn set_status_line(&mut self, line: usize, info: String) -> Result<()> {
        self.status_area.set_status_line(line, info);
        self.status_area.redraw_line(&mut self.screen, line)?;
        write!(self.screen, "{}", self.goto_prompt())?;
        Ok(())
    }

    fn set_top_line(&mut self, line: Option<String>) -> Result<()> {
        if let Some(idx) = self.top_area.find(row_names::HOST_STATUS) {
            if let Some(row) = self.top_area.get_mut(idx) {
                row.body = match line {
                    Some(s) => TopRowBody::Text(s),
                    None => TopRowBody::HostTags,
                };
            }
        }
        self.redraw_top_area()
    }

    fn set_top_row(
        &mut self,
        selector: super::TopRowSelector,
        opts: super::TopRowOpts,
    ) -> Result<()> {
        let Some(idx) = self.top_area.resolve(&selector) else {
            return Ok(());
        };
        let prior_visible_rows = self.top_area.visible_row_count();
        self.top_area.apply_opts(idx, opts);
        if self.top_area.visible_row_count() != prior_visible_rows {
            self.setup()?;
        } else {
            self.redraw_top_area()?;
            self.screen.flush().ok();
        }
        Ok(())
    }

    fn reset_top_row(&mut self, selector: super::TopRowSelector) -> Result<()> {
        let Some(idx) = self.top_area.resolve(&selector) else {
            return Ok(());
        };
        self.top_area.reset_body(idx);
        self.redraw_top_area()
    }

    fn add_top_row(&mut self, opts: super::TopRowOpts) -> Result<()> {
        self.top_area.add_row(opts);
        // New row may consume a screen line, so relayout.
        self.setup()
    }

    fn remove_top_row(&mut self, selector: super::TopRowSelector) -> Result<()> {
        let Some(idx) = self.top_area.resolve(&selector) else {
            return Ok(());
        };
        if self.top_area.remove_row(idx) {
            self.setup()?;
        }
        Ok(())
    }

    fn flush(&mut self) {
        self.screen.flush().unwrap();
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn destroy(mut self: Box<Self>) -> Result<(Box<dyn Write>, History)> {
        self.reset()?;
        Ok((self.screen, self.history))
    }

    fn swap_history(&mut self, new: History) -> Result<History> {
        // Reset the per-history scroll state — scroll_pos is meaningless
        // against a different buffer.
        self.scroll_data = ScrollData::new();
        let old = std::mem::replace(&mut self.history, new);
        // Re-apply the current tag mask so the new history's `visible`
        // view is consistent with the screen's filter.
        self.history.set_tag_mask(self.tag_mask.clone());
        // Repaint with the new buffer.
        self.setup()?;
        Ok(old)
    }

    fn set_tab_indicator(&mut self, tabs: Vec<TabInfo>) -> Result<()> {
        // Detect whether the layout's row count is about to change
        // (the dedicated indicator row's visibility crosses the
        // "only main" ↔ "multi-tab" or inline-mode boundary). If so,
        // delegate to setup() so the scroll region shifts correctly.
        let was_visible_rows = self.top_area.visible_row_count();
        self.tabs_metadata = tabs;
        let host_visible = self
            .top_area
            .find(row_names::HOST_STATUS)
            .and_then(|i| self.top_area.get(i))
            .map(|r| r.visible)
            .unwrap_or(false);
        let inline_mode = self.inline_tabs_setting && host_visible;
        let multi_tab = self.tabs_metadata.len() > 1;
        let will_be_dedicated_row = multi_tab && !inline_mode;
        if let Some(idx) = self.top_area.find(row_names::TAB_INDICATOR) {
            if let Some(row) = self.top_area.get_mut(idx) {
                row.visible = will_be_dedicated_row;
            }
        }
        if was_visible_rows != self.top_area.visible_row_count() {
            self.setup()?;
        } else {
            self.redraw_top_area()?;
            self.screen.flush().ok();
        }
        Ok(())
    }
}

impl SplitScreen {
    pub fn new(screen: Box<dyn Write>, history: History) -> Result<Self> {
        let (width, height) = termion::terminal_size()?;

        // `setup()` recomputes this from the live top-area row count before
        // anything is drawn; the initial guess only has to be self-consistent.
        let top_rows = TopArea::new_default().visible_row_count();
        let status_area_height = 1;
        let layout = ScreenLayout::compute(height, top_rows, status_area_height, INPUT_HEIGHT_MIN)
            .ok_or(TerminalSizeError)?;

        let status_area = StatusArea::new(layout.status_height, layout.mud_prompt_line + 1, width);

        Ok(Self {
            screen,
            width,
            height,
            output_start_line: layout.output_start_line,
            output_line: layout.output_line,
            mud_prompt_line: layout.mud_prompt_line,
            mud_prompt: Line::from(""),
            status_area,
            prompt_line: layout.input_start_line,
            cursor_prompt_pos: 1,
            cursor_prompt_row: 0,
            input_height: layout.input_height,
            input_auto_expand: false,
            history,
            scroll_data: ScrollData::new(),
            connection: None,
            tags: HashSet::new(),
            prompt_input: String::new(),
            prompt_input_pos: 0,
            show_tags: false,
            tag_mask: TagMask::default(),
            top_area: TopArea::new_default(),
            tabs_metadata: Vec::new(),
            inline_tabs_setting: false,
        })
    }

    fn render_history_line(&self, index: usize) -> String {
        let line = self.history.get(index);
        if self.show_tags {
            line.tagged_line().unwrap_or_default()
        } else {
            line.print_line().unwrap_or_default().to_string()
        }
    }

    fn print_line(&mut self, line: Line) {
        let masked = line.is_masked(&self.tag_mask);
        self.history.append_line(line.clone());
        if masked {
            return;
        }
        let rendered = if self.show_tags {
            line.tagged_line().unwrap_or_default()
        } else {
            line.print_line().unwrap_or_default().to_string()
        };
        if self.scroll_data.not_scrolled_or_split() {
            write!(
                self.screen,
                "{}\r\n{}{}",
                termion::cursor::Goto(1, self.output_line),
                &rendered,
                self.goto_prompt(),
            )
            .unwrap();
        }
    }

    /// The height the input area wants for `row_count` rows of content.
    /// With auto-expand off this is just the configured height.
    fn effective_input_height(&self, row_count: usize) -> u16 {
        input_layout::desired_height(row_count, self.input_height, self.input_auto_expand)
    }

    /// Paint the input area as wrapped rows. Used when the area is taller than
    /// one row; `print_prompt_input` keeps the horizontal-scroll path for the
    /// single-row case.
    fn print_prompt_input_rows(&mut self, input: &str, pos: usize) {
        let rows = input_layout::rows(input, self.width);
        let (cursor_row, cursor_col) = input_layout::cursor_row_col(input, pos, self.width);
        let height = self.effective_input_height(rows.len());
        let window = input_layout::visible_rows(rows.len(), cursor_row as usize, height);

        self.cursor_prompt_row = (cursor_row as usize).saturating_sub(window.start) as u16;
        self.cursor_prompt_pos = cursor_col + 1;

        let mut out = String::new();
        // Erase by *region* extent, not content extent. Terminal cells persist
        // until something overwrites them, so a clear loop bounded by the row
        // count never visits the rows the input just gave up — backspacing
        // past a wrap or submitting would leave the old text sitting below the
        // live input.
        for offset in 0..height {
            let text = window
                .start
                .checked_add(offset as usize)
                .filter(|i| *i < window.end)
                .map(|i| &input[rows[i].clone()])
                .unwrap_or("");

            out.push_str(&format!(
                "{}{}{}{}{}{}",
                termion::cursor::Goto(1, self.prompt_line + offset),
                Fg(termion::color::Reset),
                Bg(termion::color::Reset),
                termion::style::Reset,
                termion::clear::CurrentLine,
                text,
            ));
        }
        out.push_str(&self.goto_prompt());

        write!(self.screen, "{out}").unwrap();
    }

    fn clear_prompt(&mut self) {
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, self.mud_prompt_line),
            termion::clear::CurrentLine,
            self.goto_prompt(),
        )
        .unwrap();
    }

    fn redraw_prompt(&mut self) {
        let prompt_line = if self.show_tags {
            self.mud_prompt.tagged_line().unwrap_or_default()
        } else {
            self.mud_prompt.print_line().unwrap_or("").to_string()
        };
        let prompt_line = prompt_line.as_str();
        if self.scroll_data.not_scrolled_or_split() {
            write!(
                self.screen,
                "{}{}{}{}",
                termion::cursor::Goto(1, self.mud_prompt_line),
                termion::clear::CurrentLine,
                prompt_line,
                self.goto_prompt(),
            )
            .unwrap();
        }
    }

    fn redraw_top_area(&mut self) -> Result<()> {
        let ctx = TopRenderContext {
            width: self.width,
            connection: self.connection.as_ref(),
            tags: &self.tags,
            tabs: &self.tabs_metadata,
            inline_tabs_active: self.inline_tabs_setting && self.tabs_metadata.len() > 1,
        };
        self.top_area.render(1, &mut self.screen, &ctx)?;
        write!(self.screen, "{}", self.goto_prompt())?;
        Ok(())
    }

    fn redraw_status_area(&mut self) -> Result<()> {
        self.status_area.set_width(self.width);
        self.status_area.update_pos(self.mud_prompt_line + 1);
        self.status_area.redraw(&mut self.screen)?;
        write!(self.screen, "{}", self.goto_prompt(),)?;
        Ok(())
    }

    fn goto_prompt(&self) -> String {
        format!(
            "{}",
            termion::cursor::Goto(
                self.cursor_prompt_pos,
                self.prompt_line + self.cursor_prompt_row
            ),
        )
    }

    fn init_scroll(&mut self) -> Result<()> {
        self.scroll_data.active = true;
        if self.scroll_range() < self.output_range() {
            self.scroll_data.split = true;
            let scroll_range = self.scroll_range();

            // The divider sits directly below the frozen scrollback rows;
            // the live region starts on the row after it.
            let divider_line = scroll_range + self.output_start_line;
            write!(self.screen, "{ResetScrollRegion}")?;
            write!(
                self.screen,
                "{}{}",
                ScrollRegion(divider_line + 1, self.output_line),
                DisableOriginMode
            )?;
            write!(
                self.screen,
                "{}{}{:━<4$}{}",
                cursor::Goto(1, divider_line),
                color::Fg(color::Green),
                "━ (scroll) ",
                color::Fg(color::Reset),
                self.width as usize
            )?;
        } else {
            self.status_area.set_scroll_marker(true);
            self.status_area.redraw_line(&mut self.screen, 0)?;
            self.clear_prompt();
        }
        Ok(())
    }

    fn draw_scroll(&mut self) -> Result<()> {
        let output_range = self.scroll_range();
        for i in 0..output_range {
            let index = self.scroll_data.pos + i as usize;
            if index >= self.history.len() {
                // History has been trimmed during scrolling
                // TODO: It should be possible to lock history during render perhaps?
                // The lock would prevent the drain function until scrolls is done.
                break;
            }
            let line_no = self.output_start_line + i;
            let mut rendered = self.render_history_line(index);
            if let Some(pattern) = &self.scroll_data.hilite {
                rendered = pattern
                    .replace_all(
                        &rendered,
                        format!(
                            "{}{}$0{}{}",
                            Fg(color::LightWhite),
                            Bg(color::Blue),
                            Bg(color::Reset),
                            Fg(color::Reset)
                        ),
                    )
                    .to_string();
            }
            write!(
                self.screen,
                "{}{}{}",
                termion::cursor::Goto(1, line_no),
                termion::clear::CurrentLine,
                rendered,
            )?;
        }
        Ok(())
    }

    /// Rows of frozen scrollback shown while scrolling.
    ///
    /// The split reserves `SCROLL_LIVE_BUFFER_SIZE` rows at the bottom of the
    /// output region for live output, so it is only possible when the output
    /// region has more rows than that.
    fn scroll_range(&self) -> u16 {
        if self.scroll_data.allow_split && self.output_range() > SCROLL_LIVE_BUFFER_SIZE {
            self.output_range() - SCROLL_LIVE_BUFFER_SIZE
        } else {
            self.output_range()
        }
    }

    fn output_range(&self) -> u16 {
        self.output_line - self.output_start_line + 1
    }
}

#[cfg(test)]
mod screen_test {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_append_history() {
        let line = "a nice line\n\nwith a blank line\nand lines\nc\ntest\n";

        let mut history = History::new();
        history.append(line);
        let content: Vec<&str> = history.iter().map(|l| l.line()).collect();
        assert_eq!(
            content,
            vec![
                "a nice line",
                "",
                "with a blank line",
                "and lines",
                "c",
                "test",
            ]
        );
    }

    #[test]
    fn test_search_history() {
        let line = "a nice line\n\nwith a blank line\nand lines\nc\ntest\n";

        let mut history = History::new();
        history.append(line);
        let re = crate::model::Regex::new("and lines", None).unwrap();
        assert_eq!(history.find_forward(&re, 0), Some(3));
        assert_eq!(history.find_forward(&re, 4), None);
        assert_eq!(history.find_backward(&re, 4), Some(3));
        assert_eq!(history.find_backward(&re, 2), None);
    }

    #[test]
    fn test_drain_history() {
        let mut history = History::new();
        history.capacity = 20;
        history.drain_length = 10;
        assert!(history.is_empty());
        for _ in 0..19 {
            history.append("test");
        }
        assert_eq!(history.len(), 19);
        history.append("test");
        assert_eq!(history.len(), 10);
        for _ in 0..9 {
            history.append("test");
        }
        assert_eq!(history.len(), 19);
        history.append("test");
        assert_eq!(history.len(), 10);
    }

    // Tests for print_line tag mask behaviour. SplitScreen::new requires a real
    // terminal so we exercise the behaviour through History directly, which is
    // what print_line delegates to.

    #[test]
    fn test_print_line_masked_stored_in_inner_not_visible() {
        // Simulates print_line: always calls history.append_line, skips
        // rendering when masked. Verify masked line lands in inner but not
        // visible.
        let mut history = History::new();
        history.set_tag_mask(TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        });

        let mut masked = Line::from("combat hit");
        masked.tag.key = "combat".to_string();
        // print_line always appends regardless of mask
        history.append_line(masked);
        history.append_line(Line::from("normal line"));

        assert_eq!(history.len(), 1); // only unmasked in visible
        assert_eq!(history.get(0).clean_line(), "normal line");
    }

    #[test]
    fn test_print_line_unmasked_line_visible() {
        let mut history = History::new();
        history.set_tag_mask(TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        });

        history.append_line(Line::from("system message"));
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).clean_line(), "system message");
    }

    #[test]
    fn test_set_tag_mask_retroactive() {
        // Simulates set_tag_mask on SplitScreen: history is rebuilt from inner
        // so previously appended lines are filtered retroactively.
        let mut history = History::new();

        let mut combat = Line::from("combat hit");
        combat.tag.key = "combat".to_string();
        history.append_line(combat);
        history.append_line(Line::from("normal line"));
        assert_eq!(history.len(), 2);

        history.set_tag_mask(TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        });
        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).clean_line(), "normal line");
    }

    #[test]
    fn test_clear_tag_mask_restores_all_lines() {
        let mut history = History::new();
        let mut combat = Line::from("combat hit");
        combat.tag.key = "combat".to_string();
        history.append_line(combat);
        history.append_line(Line::from("normal line"));

        history.set_tag_mask(TagMask {
            key: Some("combat".to_string()),
            ..Default::default()
        });
        assert_eq!(history.len(), 1);

        history.set_tag_mask(TagMask::default());
        assert_eq!(history.len(), 2);
    }

    // Krendil's draw_bar tests (#1436) — preserved here to keep coverage on
    // the StatusArea consumer of `top_area::draw_bar`. Equivalent
    // body-level coverage on the topbar side lives in `top_area::tests`.

    #[test]
    fn test_draw_bar_pads_to_length_ignoring_escape_sequences() {
        let mut buf = Vec::<u8>::new();

        top_area::draw_bar('━', 10, 1, &mut buf, "test").unwrap();

        let clean_output = String::from_utf8(buf)
            .unwrap()
            .as_str()
            .printable_chars()
            .collect::<String>();

        assert_eq!(clean_output, "━ test ━━━");
    }

    #[test]
    fn test_draw_bar_is_unbroken_for_empty_string() {
        let mut buf = Vec::<u8>::new();

        top_area::draw_bar('━', 10, 1, &mut buf, "").unwrap();

        let clean_output = String::from_utf8(buf)
            .unwrap()
            .as_str()
            .printable_chars()
            .collect::<String>();

        assert_eq!(clean_output, "━━━━━━━━━━");
    }

    #[test]
    fn test_draw_bar_truncates_long_text() {
        let mut buf = Vec::<u8>::new();

        top_area::draw_bar('━', 10, 1, &mut buf, "this text is too long").unwrap();

        let clean_output = String::from_utf8(buf)
            .unwrap()
            .as_str()
            .printable_chars()
            .collect::<String>();

        assert_eq!(clean_output, "━ this t ━");
    }

    // ---- Input area rendering -------------------------------------------
    //
    // `SplitScreen::new` needs a real terminal, so these build the struct
    // directly against a capturing writer.

    #[derive(Clone)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A screen `width` columns wide with an `input_height`-row input area
    /// starting at row `prompt_line`, plus the buffer it renders into.
    fn test_screen(width: u16, input_height: u16) -> (SplitScreen, Arc<Mutex<Vec<u8>>>) {
        let sink = Arc::new(Mutex::new(Vec::new()));
        let height = 24;
        let layout = ScreenLayout::compute(height, 2, 1, input_height).unwrap();
        let screen = SplitScreen {
            screen: Box::new(SharedBuf(sink.clone())),
            width,
            height,
            output_start_line: layout.output_start_line,
            output_line: layout.output_line,
            mud_prompt_line: layout.mud_prompt_line,
            mud_prompt: Line::from(""),
            status_area: StatusArea::new(layout.status_height, layout.mud_prompt_line + 1, width),
            prompt_line: layout.input_start_line,
            cursor_prompt_pos: 1,
            cursor_prompt_row: 0,
            input_height: layout.input_height,
            input_auto_expand: false,
            history: History::new(),
            scroll_data: ScrollData::new(),
            connection: None,
            tags: HashSet::new(),
            prompt_input: String::new(),
            prompt_input_pos: 0,
            show_tags: false,
            tag_mask: TagMask::default(),
            top_area: TopArea::new_default(),
            tabs_metadata: Vec::new(),
            inline_tabs_setting: false,
        };
        (screen, sink)
    }

    fn rendered(sink: &Arc<Mutex<Vec<u8>>>) -> String {
        String::from_utf8(sink.lock().unwrap().clone()).unwrap()
    }

    /// The default configuration must not take the row path at all.
    #[test]
    fn single_row_input_keeps_the_horizontal_scroll_path() {
        let (mut screen, sink) = test_screen(20, 1);
        assert_eq!(screen.input_height, INPUT_HEIGHT_MIN);

        // Longer than the width, with the cursor at the end: the single-row
        // path scrolls sideways behind a '>' rather than wrapping.
        let input = "abcdefghijklmnopqrstuvwxyz";
        screen.print_prompt_input(input, input.chars().count());

        let out = rendered(&sink);
        assert!(
            out.contains('>'),
            "expected the scroll indicator in {out:?}"
        );
        assert_eq!(screen.cursor_prompt_row, 0);
        // Everything lands on the one input row.
        assert_eq!(
            out.matches(&format!("{}", termion::cursor::Goto(1, screen.prompt_line)))
                .count(),
            1
        );
    }

    #[test]
    fn multi_row_input_wraps_instead_of_scrolling() {
        let (mut screen, sink) = test_screen(10, 3);
        let input = "abcdefghijklmno"; // 15 chars over 10 columns -> 2 rows
        screen.print_prompt_input(input, input.chars().count());

        let out = rendered(&sink);
        assert!(!out.contains('>'), "row path must not scroll sideways");
        assert!(out.contains("abcdefghij"));
        assert!(out.contains("klmno"));
        assert_eq!(screen.cursor_prompt_row, 1);
        assert_eq!(screen.cursor_prompt_pos, 6); // 5 columns in, 1-based
    }

    /// Newlines start new rows rather than being emitted as raw control
    /// characters.
    #[test]
    fn multi_row_input_renders_logical_rows() {
        let (mut screen, sink) = test_screen(20, 3);
        screen.print_prompt_input("one\ntwo", 7);

        let out = rendered(&sink);
        assert!(out.contains("one"));
        assert!(out.contains("two"));
        assert_eq!(screen.cursor_prompt_row, 1);
        assert_eq!(screen.cursor_prompt_pos, 4);
    }

    /// The input area is erased by region extent. A content-bounded clear
    /// leaves the rows the input just gave up still showing their old text.
    #[test]
    fn every_input_row_is_erased_even_when_content_shrinks() {
        let (mut screen, sink) = test_screen(20, 3);

        // Three rows of content, then one.
        screen.print_prompt_input("a\nb\nc", 5);
        sink.lock().unwrap().clear();
        screen.print_prompt_input("a", 1);

        let out = rendered(&sink);
        for offset in 0..3 {
            let goto = format!("{}", termion::cursor::Goto(1, screen.prompt_line + offset));
            assert!(
                out.contains(&goto),
                "row {offset} was never visited, so its old content survives"
            );
        }
        assert_eq!(
            out.matches(&format!("{}", termion::clear::CurrentLine))
                .count(),
            3,
            "each of the three rows must be cleared"
        );
    }

    /// Content taller than the area scrolls within it, and the cursor stays
    /// inside the visible window.
    #[test]
    fn content_taller_than_the_area_scrolls_within_it() {
        let (mut screen, sink) = test_screen(20, 2);
        let input = "r0\nr1\nr2\nr3";
        screen.print_prompt_input(input, input.chars().count());

        let out = rendered(&sink);
        assert!(out.contains("r2") && out.contains("r3"));
        assert!(!out.contains("r0"), "row 0 has scrolled out of the area");
        assert!(screen.cursor_prompt_row < screen.input_height);
    }

    /// `goto_prompt` addresses the cursor's row within the area, not always
    /// the first row — every other draw path appends it, so a wrong row here
    /// would misplace the cursor after any redraw.
    #[test]
    fn goto_prompt_targets_the_cursor_row() {
        let (mut screen, _sink) = test_screen(20, 3);
        screen.print_prompt_input("a\nb\nc", 5);

        assert_eq!(screen.cursor_prompt_row, 2);
        assert_eq!(
            screen.goto_prompt(),
            format!("{}", termion::cursor::Goto(2, screen.prompt_line + 2))
        );
    }
}
