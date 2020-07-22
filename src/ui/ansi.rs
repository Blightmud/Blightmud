use ansi_parser::{AnsiParser, AnsiSequence, Output};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedSpan {
    pub content: String,
    pub style: Style,
}

impl<'a> From<OwnedSpan> for Span<'a> {
    fn from(owned: OwnedSpan) -> Span<'a> {
        Span::styled(owned.content, owned.style)
    }
}

impl From<String> for OwnedSpan {
    fn from(f: String) -> Self {
        OwnedSpan {
            content: f,
            style: Style::default(),
        }
    }
}

impl From<&str> for OwnedSpan {
    fn from(f: &str) -> Self {
        OwnedSpan {
            content: f.to_string(),
            style: Style::default(),
        }
    }
}

impl OwnedSpan {
    pub fn raw<T: Into<String>>(content: T) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
        }
    }

    pub fn styled<T: Into<String>>(content: T, style: Style) -> Self {
        Self {
            content: content.into(),
            style,
        }
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedSpans {
    pub spans: Vec<OwnedSpan>,
}

impl<'a> From<OwnedSpans> for Spans<'a> {
    fn from(owned: OwnedSpans) -> Spans<'a> {
        Spans::from(owned.spans.into_iter().map(Span::from).collect::<Vec<_>>())
    }
}

impl From<Vec<OwnedSpan>> for OwnedSpans {
    fn from(f: Vec<OwnedSpan>) -> Self {
        Self { spans: f }
    }
}

impl<T: Into<OwnedSpan>> From<T> for OwnedSpans {
    fn from(f: T) -> Self {
        Self {
            spans: vec![f.into()],
        }
    }
}

impl OwnedSpans {
    pub fn len(&self) -> usize {
        self.spans.iter().map(|x| x.len()).sum()
    }
}

fn ansistr_to_spans(input: &str) -> OwnedSpans {
    fn color8(idx: u32) -> Color {
        use Color::*;
        match idx {
            0 => Black,
            1 => Red,
            2 => Green,
            3 => Yellow,
            4 => Blue,
            5 => Magenta,
            6 => Cyan,
            7 => Gray,
            8 => DarkGray,
            9 => LightRed,
            10 => LightGreen,
            11 => LightYellow,
            12 => LightBlue,
            13 => LightMagenta,
            14 => LightCyan,
            15 => White,
            _ => Reset,
        }
    }

    let it = input.ansi_parse();

    let mut style = Style::default();
    let mut ret = Vec::new();

    if input.is_empty() {
        // A Spans should always contain a Span, even if it is ""
        ret.push(OwnedSpan::styled("", Style::default()));
    }

    for output in it {
        match output {
            Output::TextBlock(s) => ret.push(OwnedSpan::styled(s, style)),
            Output::Escape(AnsiSequence::SetGraphicsMode(sequence)) => {
                let mut it = sequence.iter();
                while let Some(mode) = it.next() {
                    match mode {
                        0 => style = Style::default(),
                        1 => style = style.add_modifier(Modifier::BOLD),
                        2 => style = style.add_modifier(Modifier::DIM),
                        3 => style = style.add_modifier(Modifier::ITALIC),
                        4 => style = style.add_modifier(Modifier::UNDERLINED),
                        5 => style = style.add_modifier(Modifier::SLOW_BLINK),
                        6 => style = style.add_modifier(Modifier::RAPID_BLINK),
                        7 => style = style.add_modifier(Modifier::REVERSED),
                        8 => style = style.add_modifier(Modifier::HIDDEN),
                        9 => style = style.add_modifier(Modifier::CROSSED_OUT),
                        22 => style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
                        23 => style = style.remove_modifier(Modifier::ITALIC),
                        24 => style = style.remove_modifier(Modifier::UNDERLINED),
                        25 => {
                            style =
                                style.remove_modifier(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK)
                        }
                        27 => style = style.remove_modifier(Modifier::REVERSED),
                        28 => style = style.remove_modifier(Modifier::HIDDEN),
                        29 => style = style.remove_modifier(Modifier::CROSSED_OUT),
                        30..=37 => style = style.fg(color8(mode - 30)),
                        38 => {
                            let typ = it.next();
                            match typ {
                                Some(5) => {
                                    if let Some(idx) = it.next() {
                                        style = style.fg(Color::Indexed(*idx as u8));
                                    }
                                }
                                Some(2) => {
                                    if let (Some(r), Some(g), Some(b)) =
                                        (it.next(), it.next(), it.next())
                                    {
                                        style = style.fg(Color::Rgb(*r as u8, *g as u8, *b as u8));
                                    }
                                }
                                _ => (),
                            }
                        } // TODO
                        39 => style = style.fg(Color::Reset),
                        40..=47 => style = style.bg(color8(mode - 40)),
                        48 => (), // TODO
                        49 => style = style.bg(Color::Reset),
                        90..=97 => style = style.fg(color8(mode - 90 + 8)),
                        100..=107 => style = style.bg(color8(mode - 100 + 8)),
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }

    OwnedSpans::from(ret)
}

pub fn parse_ansi(input: &str) -> Vec<OwnedSpans> {
    let mut ret = Vec::new();
    for line in input.lines() {
        ret.push(ansistr_to_spans(line));
    }
    if input.is_empty() {
        ret.push(OwnedSpans::from(""));
    }
    ret
}
