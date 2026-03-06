use std::fmt;

pub struct ScrollRegion(pub u16, pub u16);
impl fmt::Display for ScrollRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\x1b[{};{}r", self.0, self.1)
    }
}

pub struct ResetScrollRegion;
impl fmt::Display for ResetScrollRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\x1b[r")
    }
}

pub struct DisableOriginMode;
impl fmt::Display for DisableOriginMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\x1b[?6l")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_region_display() {
        let region = ScrollRegion(5, 20);
        assert_eq!(format!("{}", region), "\x1b[5;20r");
    }

    #[test]
    fn test_scroll_region_display_different_values() {
        let region = ScrollRegion(1, 100);
        assert_eq!(format!("{}", region), "\x1b[1;100r");
    }

    #[test]
    fn test_reset_scroll_region_display() {
        let reset = ResetScrollRegion;
        assert_eq!(format!("{}", reset), "\x1b[r");
    }

    #[test]
    fn test_disable_origin_mode_display() {
        let disable = DisableOriginMode;
        assert_eq!(format!("{}", disable), "\x1b[?6l");
    }
}
