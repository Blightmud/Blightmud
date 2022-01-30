use core::ops::Deref;
use regex::{Regex as MRegex, RegexBuilder};
use std::ops::DerefMut;

use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct RegexOptions {
    pub case_insensitive: bool,
    pub multi_line: bool,
}

impl Default for RegexOptions {
    fn default() -> Self {
        Self {
            case_insensitive: false,
            multi_line: false,
        }
    }
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
