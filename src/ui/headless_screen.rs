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
