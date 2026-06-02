use anyhow::Result;
use std::collections::HashSet;
use std::io::Write;
use termion::color::{self, Bg, Fg};

use crate::tabs::TabInfo;
use crate::tools::printable_chars::PrintableCharsIterator;

/// Reserved names for the built-in top rows. Lua scripts address these
/// by name via `blight.set_top_row(name, ...)`.
pub mod row_names {
    pub const TAB_INDICATOR: &str = "tab_indicator";
    pub const HOST_STATUS: &str = "host_status";
}

/// Visual style applied to a row's prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopPrefixStyle {
    /// Bare green text, no extra styling. The text is wrapped in the row's
    /// bar character on each side (e.g. `═══ {text} ═══`).
    Plain,
    /// Bold light-white text framed by green bar segments (e.g. the
    /// `═══ Blightmud ══` brand badge).
    Brand,
}

/// Optional decorative leader on a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopPrefix {
    /// Visible label text (no styling). Typically `" Blightmud "`.
    pub text: String,
    pub style: TopPrefixStyle,
}

/// The content slot of a row. Built-in dynamic bodies (`HostTags`,
/// `TabIndicator`) are computed from the render context at draw time;
/// `Text` is the static escape hatch used by `blight.top_line` and
/// `blight.set_top_row`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopRowBody {
    /// Literal user-supplied text (from `blight.top_line` /
    /// `set_top_row body=...`).
    Text(String),
    /// Built-in: render the active connection's `host:port` plus sorted
    /// tags. The default body for the `host_status` row.
    HostTags,
    /// Built-in: render the styled tabs segment. The default body for the
    /// `tab_indicator` row. Renders nothing when fewer than 2 tabs exist.
    TabIndicator,
    /// Empty body — produces an unbroken bar across the row's width.
    Empty,
}

/// A single addressable row in the top area.
#[derive(Debug, Clone)]
pub struct TopRow {
    /// Unique name used for Lua addressing.
    pub name: String,
    pub bar_char: char,
    pub prefix: Option<TopPrefix>,
    pub body: TopRowBody,
    /// Hidden rows are skipped during rendering and don't consume a
    /// terminal line.
    pub visible: bool,
}

/// How a row is addressed from Lua / events. Index is zero-based,
/// counting both visible and hidden rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopRowSelector {
    Index(usize),
    Name(String),
}

/// Partial update to a row's fields. `None` means "leave unchanged".
/// `prefix` uses the doubly-nested `Option` so callers can distinguish
/// "no change" from "clear the prefix":
///   - `prefix: None`              ⇒ no change
///   - `prefix: Some(None)`        ⇒ remove existing prefix
///   - `prefix: Some(Some(prefix))`⇒ replace with given prefix
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TopRowOpts {
    pub name: Option<String>,
    pub bar_char: Option<char>,
    pub prefix: Option<Option<TopPrefix>>,
    pub body: Option<TopRowBody>,
    pub visible: Option<bool>,
}

/// Top area = ordered list of rows. Built-ins come first (`tab_indicator`,
/// `host_status`); Lua-added rows are appended.
#[derive(Debug, Clone)]
pub struct TopArea {
    rows: Vec<TopRow>,
}

/// State needed at render time but not stored in the row itself.
pub struct TopRenderContext<'a> {
    pub width: u16,
    pub connection: Option<&'a String>,
    pub tags: &'a HashSet<String>,
    pub tabs: &'a [TabInfo],
    /// When true, the `host_status` row's body is decorated by appending
    /// the tabs segment after it, and the dedicated `tab_indicator` row
    /// is expected to be hidden.
    pub inline_tabs_active: bool,
}

impl TopArea {
    /// Default layout: tab indicator row (hidden until tabs > 1) followed
    /// by the host status row.
    pub fn new_default() -> Self {
        Self {
            rows: vec![
                TopRow {
                    name: row_names::TAB_INDICATOR.to_string(),
                    bar_char: '═',
                    prefix: Some(TopPrefix {
                        text: " Blightmud ".to_string(),
                        style: TopPrefixStyle::Brand,
                    }),
                    body: TopRowBody::TabIndicator,
                    visible: false,
                },
                TopRow {
                    name: row_names::HOST_STATUS.to_string(),
                    bar_char: '═',
                    prefix: None,
                    body: TopRowBody::HostTags,
                    visible: true,
                },
            ],
        }
    }

    #[cfg(test)]
    pub fn rows(&self) -> &[TopRow] {
        &self.rows
    }

    #[cfg(test)]
    pub fn rows_mut(&mut self) -> &mut Vec<TopRow> {
        &mut self.rows
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        self.rows.iter().position(|r| r.name == name)
    }

    pub fn get(&self, idx: usize) -> Option<&TopRow> {
        self.rows.get(idx)
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut TopRow> {
        self.rows.get_mut(idx)
    }

    pub fn visible_row_count(&self) -> u16 {
        self.rows.iter().filter(|r| r.visible).count() as u16
    }

    /// Resolve a selector to a row index.
    pub fn resolve(&self, sel: &TopRowSelector) -> Option<usize> {
        match sel {
            TopRowSelector::Index(i) => {
                if *i < self.rows.len() {
                    Some(*i)
                } else {
                    None
                }
            }
            TopRowSelector::Name(name) => self.find(name),
        }
    }

    /// Apply a `TopRowOpts` patch to the row at `idx`. Returns true when
    /// the row's visibility flag changed (so callers know to relayout).
    pub fn apply_opts(&mut self, idx: usize, opts: TopRowOpts) -> bool {
        let Some(row) = self.rows.get_mut(idx) else {
            return false;
        };
        let mut visibility_changed = false;
        if let Some(name) = opts.name {
            row.name = name;
        }
        if let Some(c) = opts.bar_char {
            row.bar_char = c;
        }
        if let Some(p) = opts.prefix {
            row.prefix = p;
        }
        if let Some(b) = opts.body {
            row.body = b;
        }
        if let Some(v) = opts.visible {
            if row.visible != v {
                visibility_changed = true;
            }
            row.visible = v;
        }
        visibility_changed
    }

    /// Reset the body of a built-in row to its dynamic default. For
    /// Lua-added rows this is a no-op.
    pub fn reset_body(&mut self, idx: usize) {
        let Some(row) = self.rows.get_mut(idx) else {
            return;
        };
        match row.name.as_str() {
            n if n == row_names::TAB_INDICATOR => row.body = TopRowBody::TabIndicator,
            n if n == row_names::HOST_STATUS => row.body = TopRowBody::HostTags,
            _ => {}
        }
    }

    /// Append a new row. Returns the assigned index. Auto-generates a
    /// unique name when the supplied opts didn't specify one.
    pub fn add_row(&mut self, opts: TopRowOpts) -> usize {
        let name = opts.name.clone().unwrap_or_else(|| {
            let mut i = self.rows.len();
            loop {
                let candidate = format!("custom_{i}");
                if self.find(&candidate).is_none() {
                    return candidate;
                }
                i += 1;
            }
        });
        let row = TopRow {
            name,
            bar_char: opts.bar_char.unwrap_or('═'),
            prefix: opts.prefix.unwrap_or(None),
            body: opts.body.unwrap_or(TopRowBody::Empty),
            visible: opts.visible.unwrap_or(true),
        };
        self.rows.push(row);
        self.rows.len() - 1
    }

    /// Remove the row at `idx`. The built-in rows (`tab_indicator`,
    /// `host_status`) refuse removal — returns `false` for those.
    pub fn remove_row(&mut self, idx: usize) -> bool {
        let Some(row) = self.rows.get(idx) else {
            return false;
        };
        if row.name == row_names::TAB_INDICATOR || row.name == row_names::HOST_STATUS {
            return false;
        }
        self.rows.remove(idx);
        true
    }

    /// Returns the 1-based screen line for the i-th row, given the start
    /// line of the top area (typically 1). Returns 0 when the row is
    /// hidden or out of bounds.
    #[cfg(test)]
    pub fn screen_line(&self, idx: usize, start_line: u16) -> u16 {
        if idx >= self.rows.len() || !self.rows[idx].visible {
            return 0;
        }
        let mut line = start_line;
        for prior in &self.rows[..idx] {
            if prior.visible {
                line += 1;
            }
        }
        line
    }

    /// Render every visible row in order.
    pub fn render(
        &self,
        start_line: u16,
        screen: &mut impl Write,
        ctx: &TopRenderContext,
    ) -> Result<()> {
        let mut line = start_line;
        for row in &self.rows {
            if !row.visible {
                continue;
            }
            render_row(row, line, screen, ctx)?;
            line += 1;
        }
        Ok(())
    }
}

pub(crate) fn repeat_char(c: char, n: usize) -> String {
    let mut s = String::with_capacity(n * c.len_utf8());
    for _ in 0..n {
        s.push(c);
    }
    s
}

/// Krendil's bare-bar rendering primitive (from #1436): clears the line,
/// writes `{bar} {text}{Bg::Reset}{Fg::Reset}{Fg::Green} ` (truncating
/// text past `width - 4` display columns), and fills the remainder with
/// `barchar`. Used by both this module's no-prefix render path and by
/// `StatusArea::redraw_line` for its `━` separators.
pub(crate) fn draw_bar(
    barchar: char,
    width: usize,
    line: usize,
    screen: &mut impl Write,
    custom_info: &str,
) -> Result<()> {
    write!(
        screen,
        "{}{}{}",
        termion::cursor::Goto(1, line as u16),
        termion::clear::CurrentLine,
        Fg(color::Green),
    )?;

    let trimmed = custom_info.trim();
    let custom_info = if !trimmed.is_empty() {
        let (max_bytes, _) = trimmed.byte_index_at_display_width(width - 4);
        format!(
            "{} {}{}{}{} ",
            barchar,
            trimmed.get(0..max_bytes).unwrap_or(trimmed),
            Bg(color::Reset),
            Fg(color::Reset),
            Fg(color::Green)
        )
    } else {
        String::new()
    };

    let remainder = width - custom_info.as_str().display_width();
    write!(screen, "{}", &custom_info)?;
    if remainder > 0 {
        screen.write_all(repeat_char(barchar, remainder).as_bytes())?;
    }
    write!(screen, "{}", Fg(color::Reset))?;
    Ok(())
}

/// Compute the styled host:port + sorted tags label, mirroring the
/// historical SplitScreen format.
fn format_host_tags(ctx: &TopRenderContext) -> String {
    let host = ctx.connection.map(String::as_str).unwrap_or("");
    let mut tags: Vec<String> = ctx.tags.iter().map(|s| format!("[{s}]")).collect();
    tags.sort();
    let tags = tags.join("");
    let mut output = format!("{host} {tags}");
    if !output.trim().is_empty() {
        output.push(' ');
    } else {
        output.clear();
    }
    output
}

/// Build the styled tabs segment (e.g. `[main] │ (chat·3) │ (combat)`)
/// along with its visible width. Returns `("", 0)` for <2 tabs.
pub fn format_tabs_segment(tabs: &[TabInfo]) -> (String, usize) {
    if tabs.len() < 2 {
        return (String::new(), 0);
    }
    let mut tabs_styled = String::new();
    let mut tabs_visible_width: usize = 0;
    let sep = " \u{2502} "; // " │ "
    for (i, tab) in tabs.iter().enumerate() {
        if i > 0 {
            tabs_styled.push_str(&format!("{}{}{}", Fg(color::Green), sep, Fg(color::Reset)));
            tabs_visible_width += sep.chars().count();
        }
        let body = if let Some(sc) = tab.shortcut.as_ref() {
            format!("{sc} - {}", tab.label)
        } else {
            tab.label.clone()
        };
        let (styled, plain) = if tab.active {
            let plain = format!("[{body}]");
            let styled = format!(
                "{}{}{}{}{}",
                termion::style::Bold,
                Fg(color::LightGreen),
                plain,
                Fg(color::Reset),
                termion::style::Reset,
            );
            (styled, plain)
        } else if tab.unread > 0 {
            let plain = format!("({body}·{})", tab.unread);
            let styled = format!("{}{}{}", Fg(color::LightYellow), plain, Fg(color::Reset));
            (styled, plain)
        } else {
            let plain = format!("({body})");
            let styled = format!("{}{}{}", Fg(color::LightBlack), plain, Fg(color::Reset));
            (styled, plain)
        };
        tabs_styled.push_str(&styled);
        tabs_visible_width += plain.chars().count();
    }
    (tabs_styled, tabs_visible_width)
}

/// Render the prefix to (styled bytes, visible width). Includes the
/// trailing spacer between prefix and body.
fn render_prefix(prefix: &Option<TopPrefix>, bar_char: char) -> (String, usize) {
    let Some(p) = prefix else {
        return (String::new(), 0);
    };
    match p.style {
        TopPrefixStyle::Plain => {
            // `═══ {text} ` — text with a leading triple-bar and trailing space.
            let lead = repeat_char(bar_char, 3);
            let visible = 3 + p.text.chars().count() + 1;
            let styled = format!(
                "{}{}{}{} ",
                Fg(color::Green),
                lead,
                p.text,
                Fg(color::Reset)
            );
            (styled, visible)
        }
        TopPrefixStyle::Brand => {
            // `═══ {bold light-white text} ══ ` — the canonical brand badge.
            let lead = repeat_char(bar_char, 3);
            let trail = repeat_char(bar_char, 2);
            let visible = 3 + p.text.chars().count() + 2 + 1;
            let styled = format!(
                "{}{}{}{}{}{}{}{}{}{} ",
                Fg(color::Green),
                lead,
                termion::style::Bold,
                Fg(color::LightWhite),
                p.text,
                Fg(color::Reset),
                termion::style::Reset,
                Fg(color::Green),
                trail,
                Fg(color::Reset),
            );
            (styled, visible)
        }
    }
}

/// Render a single row at the given 1-based terminal `line`. Mirrors the
/// behavior of the hybrid `redraw_top_bar` + `redraw_tab_indicator_internal`
/// before the refactor — same bytes on the wire for all built-in body kinds.
pub fn render_row(
    row: &TopRow,
    line: u16,
    screen: &mut impl Write,
    ctx: &TopRenderContext,
) -> Result<()> {
    let width = ctx.width as usize;

    // Step 1: build the body (styled + visible width) and any inline-tabs
    // decoration on the host_status row.
    let (body_styled, body_visible_width) = match &row.body {
        TopRowBody::Empty => (String::new(), 0_usize),
        TopRowBody::Text(s) => (s.clone(), s.chars().count()),
        TopRowBody::HostTags => {
            let s = format_host_tags(ctx);
            let n = s.chars().count();
            (s, n)
        }
        TopRowBody::TabIndicator => format_tabs_segment(ctx.tabs),
    };

    // Inline tabs decoration: appended after the body of the host_status
    // row when inline mode is active and there are tabs to show.
    let (body_styled, body_visible_width, has_inline_decoration) =
        if ctx.inline_tabs_active && row.name == row_names::HOST_STATUS && ctx.tabs.len() > 1 {
            let (tabs_styled, tabs_visible_width) = format_tabs_segment(ctx.tabs);
            // Body text already carries its own trailing space (format_host_tags
            // adds one when non-empty), so we splice tabs directly. The spacer
            // between tabs and trailing fill is added by the assembly step
            // below — same `+ 1` overhead as the hybrid layout, which lets the
            // fill calculation drop one column to avoid the stray-═-wrap that
            // bit us in the pre-refactor version.
            let merged = format!("{body_styled}{tabs_styled}");
            let merged_width = body_visible_width + tabs_visible_width;
            (merged, merged_width, true)
        } else {
            (body_styled, body_visible_width, false)
        };

    // Step 2: write the leader (Goto + clear + green) and the prefix.
    write!(
        screen,
        "{}{}{}",
        termion::cursor::Goto(1, line),
        termion::clear::CurrentLine,
        Fg(color::Green),
    )?;

    let (prefix_styled, prefix_visible_width) = render_prefix(&row.prefix, row.bar_char);
    if prefix_visible_width > 0 {
        screen.write_all(prefix_styled.as_bytes())?;
    }

    // Step 3: write body + trailing fill. Branches:
    //   (a) empty body  → fill rest with bar
    //   (b) prefixed    → `{body} {fill}`
    //   (c) bare        → `{bar} {body}{Bg::Reset}{Fg::Reset}{Fg::Green} {fill}`
    //   (d) inline-decorated → `{body} {Fg::Reset}{Fg::Green}{fill}` (no truncation)
    if body_visible_width == 0 {
        let remainder = width.saturating_sub(prefix_visible_width);
        if remainder > 0 {
            screen.write_all(repeat_char(row.bar_char, remainder).as_bytes())?;
        }
    } else if has_inline_decoration || prefix_visible_width > 0 {
        // Both paths consume `{prefix_visible_width + body_visible_width + 1}`
        // columns and pad the remainder with `bar_char`. For the inline
        // path, `prefix_visible_width` is 0 — for the prefixed path the
        // body is already preceded by the prefix written above.
        let used = prefix_visible_width + body_visible_width + 1;
        let remainder = width.saturating_sub(used);
        let trail = repeat_char(row.bar_char, remainder);
        write!(
            screen,
            "{} {}{}{}",
            body_styled,
            Fg(color::Reset),
            Fg(color::Green),
            trail,
        )?;
    } else {
        // Bare bar body — krendil's draw_bar style, including truncation
        // (drops bytes past `width - 4` of display width) and the
        // background-reset between body and trailing fill.
        let trimmed = body_styled.trim();
        let body_fragment = if !trimmed.is_empty() {
            let (max_bytes, _) = trimmed.byte_index_at_display_width(width.saturating_sub(4));
            let visible_text = trimmed.get(0..max_bytes).unwrap_or(trimmed);
            format!(
                "{} {}{}{}{} ",
                row.bar_char,
                visible_text,
                Bg(color::Reset),
                Fg(color::Reset),
                Fg(color::Green),
            )
        } else {
            String::new()
        };
        let used = body_fragment.as_str().display_width();
        let remainder = width.saturating_sub(used);
        screen.write_all(body_fragment.as_bytes())?;
        if remainder > 0 {
            screen.write_all(repeat_char(row.bar_char, remainder).as_bytes())?;
        }
    }

    write!(screen, "{}", Fg(color::Reset))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::printable_chars::PrintableCharsIterator;

    fn empty_tags() -> HashSet<String> {
        HashSet::new()
    }

    fn ctx<'a>(
        width: u16,
        connection: Option<&'a String>,
        tags: &'a HashSet<String>,
        tabs: &'a [TabInfo],
    ) -> TopRenderContext<'a> {
        TopRenderContext {
            width,
            connection,
            tags,
            tabs,
            inline_tabs_active: false,
        }
    }

    fn render_to_string(row: &TopRow, ctx: &TopRenderContext) -> String {
        let mut buf = Vec::<u8>::new();
        render_row(row, 1, &mut buf, ctx).unwrap();
        let raw = String::from_utf8(buf).unwrap();
        raw.as_str().printable_chars().collect::<String>()
    }

    #[test]
    fn default_layout_has_two_rows() {
        let area = TopArea::new_default();
        assert_eq!(area.rows().len(), 2);
        assert_eq!(area.rows()[0].name, row_names::TAB_INDICATOR);
        assert_eq!(area.rows()[1].name, row_names::HOST_STATUS);
        assert!(matches!(area.rows()[0].body, TopRowBody::TabIndicator));
        assert!(matches!(area.rows()[1].body, TopRowBody::HostTags));
    }

    #[test]
    fn visible_row_count_skips_hidden() {
        let mut area = TopArea::new_default();
        assert_eq!(area.visible_row_count(), 1);
        area.rows_mut()[0].visible = true;
        assert_eq!(area.visible_row_count(), 2);
    }

    #[test]
    fn screen_line_skips_hidden() {
        let mut area = TopArea::new_default();
        // Row 0 hidden by default → row 1 lands on start_line.
        assert_eq!(area.screen_line(0, 1), 0);
        assert_eq!(area.screen_line(1, 1), 1);
        // Both visible → row 0 on start_line, row 1 on start_line+1.
        area.rows_mut()[0].visible = true;
        assert_eq!(area.screen_line(0, 1), 1);
        assert_eq!(area.screen_line(1, 1), 2);
    }

    // Krendil's draw_bar test ports — Text body matches the bare-bar style.

    #[test]
    fn text_body_pads_to_width() {
        let row = TopRow {
            name: row_names::HOST_STATUS.to_string(),
            bar_char: '═',
            prefix: None,
            body: TopRowBody::Text("test".to_string()),
            visible: true,
        };
        let tags = empty_tags();
        let tabs: [TabInfo; 0] = [];
        let out = render_to_string(&row, &ctx(10, None, &tags, &tabs));
        assert_eq!(out, "═ test ═══");
    }

    #[test]
    fn empty_body_is_unbroken_bar() {
        let row = TopRow {
            name: row_names::HOST_STATUS.to_string(),
            bar_char: '═',
            prefix: None,
            body: TopRowBody::Empty,
            visible: true,
        };
        let tags = empty_tags();
        let tabs: [TabInfo; 0] = [];
        let out = render_to_string(&row, &ctx(10, None, &tags, &tabs));
        assert_eq!(out, "══════════");
    }

    #[test]
    fn long_text_body_is_truncated() {
        let row = TopRow {
            name: row_names::HOST_STATUS.to_string(),
            bar_char: '═',
            prefix: None,
            body: TopRowBody::Text("this text is too long".to_string()),
            visible: true,
        };
        let tags = empty_tags();
        let tabs: [TabInfo; 0] = [];
        let out = render_to_string(&row, &ctx(10, None, &tags, &tabs));
        assert_eq!(out, "═ this t ═");
    }

    #[test]
    fn host_tags_body_renders_connection() {
        let row = TopRow {
            name: row_names::HOST_STATUS.to_string(),
            bar_char: '═',
            prefix: None,
            body: TopRowBody::HostTags,
            visible: true,
        };
        let connection = "swmud.org:7777".to_string();
        let mut tags = empty_tags();
        tags.insert("tag1".to_string());
        let tabs: [TabInfo; 0] = [];
        let out = render_to_string(&row, &ctx(40, Some(&connection), &tags, &tabs));
        assert!(out.starts_with("═ swmud.org:7777 [tag1] "));
        assert_eq!(out.chars().count(), 40);
    }

    #[test]
    fn tab_indicator_body_renders_tabs() {
        let row = TopRow {
            name: row_names::TAB_INDICATOR.to_string(),
            bar_char: '═',
            prefix: Some(TopPrefix {
                text: " Blightmud ".to_string(),
                style: TopPrefixStyle::Brand,
            }),
            body: TopRowBody::TabIndicator,
            visible: true,
        };
        let tags = empty_tags();
        let tabs = vec![
            TabInfo {
                name: "main".to_string(),
                label: "main".to_string(),
                active: true,
                unread: 0,
                shortcut: None,
            },
            TabInfo {
                name: "chat".to_string(),
                label: "chat".to_string(),
                active: false,
                unread: 3,
                shortcut: None,
            },
        ];
        let out = render_to_string(&row, &ctx(60, None, &tags, &tabs));
        assert!(out.contains("Blightmud"));
        assert!(out.contains("[main]"));
        assert!(out.contains("(chat·3)"));
        assert_eq!(out.chars().count(), 60);
    }

    #[test]
    fn host_tags_with_inline_tabs_composes() {
        let row = TopRow {
            name: row_names::HOST_STATUS.to_string(),
            bar_char: '═',
            prefix: None,
            body: TopRowBody::HostTags,
            visible: true,
        };
        let connection = "swmud.org:7777".to_string();
        let tags = empty_tags();
        let tabs = vec![
            TabInfo {
                name: "main".to_string(),
                label: "main".to_string(),
                active: true,
                unread: 0,
                shortcut: None,
            },
            TabInfo {
                name: "chat".to_string(),
                label: "chat".to_string(),
                active: false,
                unread: 0,
                shortcut: None,
            },
        ];
        let mut c = ctx(50, Some(&connection), &tags, &tabs);
        c.inline_tabs_active = true;
        let out = render_to_string(&row, &c);
        assert!(out.contains("swmud.org:7777"));
        assert!(out.contains("[main]"));
        assert!(out.contains("(chat)"));
        assert_eq!(out.chars().count(), 50);
    }

    #[test]
    fn text_body_with_inline_tabs_composes() {
        let row = TopRow {
            name: row_names::HOST_STATUS.to_string(),
            bar_char: '═',
            prefix: None,
            body: TopRowBody::Text("HP 80/100".to_string()),
            visible: true,
        };
        let tags = empty_tags();
        let tabs = vec![
            TabInfo {
                name: "main".to_string(),
                label: "main".to_string(),
                active: true,
                unread: 0,
                shortcut: None,
            },
            TabInfo {
                name: "chat".to_string(),
                label: "chat".to_string(),
                active: false,
                unread: 0,
                shortcut: None,
            },
        ];
        let mut c = ctx(50, None, &tags, &tabs);
        c.inline_tabs_active = true;
        let out = render_to_string(&row, &c);
        assert!(out.contains("HP 80/100"));
        assert!(out.contains("[main]"));
        assert_eq!(out.chars().count(), 50);
    }

    #[test]
    fn find_returns_row_index() {
        let area = TopArea::new_default();
        assert_eq!(area.find(row_names::TAB_INDICATOR), Some(0));
        assert_eq!(area.find(row_names::HOST_STATUS), Some(1));
        assert_eq!(area.find("nonexistent"), None);
    }
}
