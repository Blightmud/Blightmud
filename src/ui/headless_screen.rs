use std::io::Write;

use anyhow::bail;

use super::UserInterface;

pub struct HeadlessScreen {}

impl UserInterface for HeadlessScreen {
    fn setup(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn print_error(&mut self, output: &str) {
        println!("[!!] {output}");
    }

    fn print_info(&mut self, output: &str) {
        println!("[**] {output}");
    }

    fn print_output(&mut self, line: &crate::model::Line) {
        println!("[<<] {line}");
    }

    fn print_prompt(&mut self, prompt: &crate::model::Line) {
        println!("[%%] {prompt}");
    }

    fn print_prompt_input(&mut self, _input: &str, _pos: usize) {}

    fn print_send(&mut self, send: &crate::model::Line) {
        if let Some(print_line) = send.print_line() {
            println!("[>>] {print_line}");
        }
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn reset_scroll(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn scroll_down(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn scroll_lock(&mut self, _lock: bool) -> anyhow::Result<()> {
        Ok(())
    }

    fn scroll_to(&mut self, _row: usize) -> anyhow::Result<()> {
        Ok(())
    }

    fn scroll_top(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn scroll_up(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn find_up(&mut self, _pattern: &crate::model::Regex) -> anyhow::Result<()> {
        Ok(())
    }

    fn find_down(&mut self, _pattern: &crate::model::Regex) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_host(&mut self, _host: &str, _port: u16) -> anyhow::Result<()> {
        Ok(())
    }

    fn add_tag(&mut self, _proto: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn remove_tag(&mut self, _proto: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn clear_tags(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_status_area_height(&mut self, _height: u16) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_status_line(&mut self, _line: usize, _info: String) -> anyhow::Result<()> {
        Ok(())
    }

    fn flush(&mut self) {
        std::io::stdout().flush().ok();
    }

    fn width(&self) -> u16 {
        0
    }

    fn height(&self) -> u16 {
        0
    }

    fn destroy(
        self: Box<Self>,
    ) -> anyhow::Result<(Box<dyn std::io::Write>, super::history::History)> {
        bail!("Can't destroy a headless ui")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Line, Regex};

    #[test]
    fn test_headless_screen_setup() {
        let mut screen = HeadlessScreen {};
        assert!(screen.setup().is_ok());
    }

    #[test]
    fn test_headless_screen_reset() {
        let mut screen = HeadlessScreen {};
        assert!(screen.reset().is_ok());
    }

    #[test]
    fn test_headless_screen_reset_scroll() {
        let mut screen = HeadlessScreen {};
        assert!(screen.reset_scroll().is_ok());
    }

    #[test]
    fn test_headless_screen_scroll_down() {
        let mut screen = HeadlessScreen {};
        assert!(screen.scroll_down().is_ok());
    }

    #[test]
    fn test_headless_screen_scroll_up() {
        let mut screen = HeadlessScreen {};
        assert!(screen.scroll_up().is_ok());
    }

    #[test]
    fn test_headless_screen_scroll_lock() {
        let mut screen = HeadlessScreen {};
        assert!(screen.scroll_lock(true).is_ok());
        assert!(screen.scroll_lock(false).is_ok());
    }

    #[test]
    fn test_headless_screen_scroll_to() {
        let mut screen = HeadlessScreen {};
        assert!(screen.scroll_to(100).is_ok());
    }

    #[test]
    fn test_headless_screen_scroll_top() {
        let mut screen = HeadlessScreen {};
        assert!(screen.scroll_top().is_ok());
    }

    #[test]
    fn test_headless_screen_find_up() {
        let mut screen = HeadlessScreen {};
        let pattern = Regex::new("test", None).unwrap();
        assert!(screen.find_up(&pattern).is_ok());
    }

    #[test]
    fn test_headless_screen_find_down() {
        let mut screen = HeadlessScreen {};
        let pattern = Regex::new("test", None).unwrap();
        assert!(screen.find_down(&pattern).is_ok());
    }

    #[test]
    fn test_headless_screen_set_host() {
        let mut screen = HeadlessScreen {};
        assert!(screen.set_host("example.com", 4000).is_ok());
    }

    #[test]
    fn test_headless_screen_add_tag() {
        let mut screen = HeadlessScreen {};
        assert!(screen.add_tag("GMCP").is_ok());
    }

    #[test]
    fn test_headless_screen_remove_tag() {
        let mut screen = HeadlessScreen {};
        assert!(screen.remove_tag("GMCP").is_ok());
    }

    #[test]
    fn test_headless_screen_clear_tags() {
        let mut screen = HeadlessScreen {};
        assert!(screen.clear_tags().is_ok());
    }

    #[test]
    fn test_headless_screen_set_status_area_height() {
        let mut screen = HeadlessScreen {};
        assert!(screen.set_status_area_height(5).is_ok());
    }

    #[test]
    fn test_headless_screen_set_status_line() {
        let mut screen = HeadlessScreen {};
        assert!(screen.set_status_line(0, "Status".to_string()).is_ok());
    }

    #[test]
    fn test_headless_screen_width() {
        let screen = HeadlessScreen {};
        assert_eq!(screen.width(), 0);
    }

    #[test]
    fn test_headless_screen_height() {
        let screen = HeadlessScreen {};
        assert_eq!(screen.height(), 0);
    }

    #[test]
    fn test_headless_screen_destroy_fails() {
        let screen = Box::new(HeadlessScreen {});
        let result = screen.destroy();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Can't destroy"));
        }
    }

    #[test]
    fn test_headless_screen_print_and_flush_methods() {
        // Consolidate all void-returning print/flush methods into one test
        // These methods just write to stdout and don't return values to assert on
        let mut screen = HeadlessScreen {};
        let line = Line::from("test content");

        // Test all print methods complete without panic
        screen.print_error("test error");
        screen.print_info("test info");
        screen.print_output(&line);
        screen.print_prompt(&line);
        screen.print_send(&line);
        screen.print_prompt_input("test input", 5);
        screen.flush();

        // If we got here, all methods executed successfully
        assert!(true);
    }
}
