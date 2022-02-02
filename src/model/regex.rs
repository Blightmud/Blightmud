use core::ops::Deref;
use regex::{Regex as MRegex, RegexBuilder};
use std::ops::DerefMut;

use anyhow::Result;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct RegexOptions {
    pub case_insensitive: bool,
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
    pub swap_greed: bool,
    pub ignore_whitespace: bool,
}

#[derive(Debug, Clone)]
pub struct Regex {
    inner: MRegex,
}

impl Regex {
    pub fn new(pattern: &str, options: Option<RegexOptions>) -> Result<Self> {
        let mut regex_builder = RegexBuilder::new(pattern);
        if let Some(options) = options {
            regex_builder.case_insensitive(options.case_insensitive);
            regex_builder.multi_line(options.multi_line);
            regex_builder.dot_matches_new_line(options.dot_matches_new_line);
            regex_builder.swap_greed(options.swap_greed);
            regex_builder.ignore_whitespace(options.ignore_whitespace);
        }
        Ok(Self {
            inner: regex_builder.build()?,
        })
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.inner.as_str() == other.inner.as_str()
    }
}

impl Deref for Regex {
    type Target = MRegex;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Regex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
