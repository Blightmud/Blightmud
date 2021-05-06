use core::ops::Deref;
use regex::Regex as MRegex;
use std::ops::DerefMut;

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Regex {
    inner: MRegex,
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Self> {
        Ok(Self {
            inner: MRegex::new(pattern)?,
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
