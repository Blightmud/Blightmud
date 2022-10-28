use std::{
    io::{stdout, Write},
    sync::{Arc, Mutex},
};

use crate::{
    io::SaveData,
    model::{Settings, MOUSE_ENABLED, READER_MODE},
    session::Session,
    tts::TTSController,
};

use super::{history::History, HeadlessScreen, ReaderScreen, SplitScreen, UserInterface};
use anyhow::Result;
use termion::{input::MouseTerminal, raw::IntoRawMode, screen::IntoAlternateScreen};

/// Creates the io::Write terminal handler we draw to.
fn create_screen_writer(mouse_support: bool) -> Result<Box<dyn Write>> {
    let screen = stdout().into_alternate_screen()?.into_raw_mode()?;
    if mouse_support {
        Ok(Box::new(MouseTerminal::from(screen)))
    } else {
        Ok(Box::new(screen))
    }
}

pub struct UiWrapper {
    screen: Box<dyn UserInterface>,
    tts_ctrl: Arc<Mutex<TTSController>>,
}

impl UiWrapper {
    pub fn new(session: &Session) -> Result<Self> {
        let settings = Settings::try_load()?;
        let reader_mode = settings.get(READER_MODE)?;
        let screen: Box<dyn UserInterface> = if reader_mode {
            Box::new(ReaderScreen::new(
                create_screen_writer(false)?,
                History::new(),
            )?)
        } else {
            Box::new(SplitScreen::new(
                create_screen_writer(settings.get(MOUSE_ENABLED)?)?,
                History::new(),
            )?)
        };
        let tts_ctrl = session.tts_ctrl.clone();

        Ok(Self { screen, tts_ctrl })
    }

    pub fn new_from(
        screen: Box<dyn UserInterface>,
        session: &Session,
        reader_mode: bool,
    ) -> Result<Self> {
        let (writer, history) = screen.destroy()?;
        let mut screen: Box<dyn UserInterface> = if reader_mode {
            Box::new(ReaderScreen::new(writer, history)?)
        } else {
            Box::new(SplitScreen::new(writer, history)?)
        };
        screen.setup()?;
        Ok(Self {
            screen,
            tts_ctrl: session.tts_ctrl.clone(),
        })
    }

    pub fn headless(session: &Session) -> Result<Self> {
        Ok(Self {
            screen: Box::new(HeadlessScreen {}),
            tts_ctrl: session.tts_ctrl.clone(),
        })
    }
}

impl UserInterface for UiWrapper {
    fn setup(&mut self) -> Result<()> {
        self.screen.setup()
    }

    fn print_error(&mut self, output: &str) {
        self.tts_ctrl.lock().unwrap().speak_error(output);
        self.screen.print_error(output);
    }

    fn print_info(&mut self, output: &str) {
        self.tts_ctrl.lock().unwrap().speak_info(output);
        self.screen.print_info(output);
    }

    fn print_output(&mut self, line: &crate::model::Line) {
        self.tts_ctrl.lock().unwrap().speak_line(line);
        self.screen.print_output(line);
    }

    fn print_prompt(&mut self, prompt: &crate::model::Line) {
        self.tts_ctrl.lock().unwrap().speak_line(prompt);
        self.screen.print_prompt(prompt);
    }

    fn print_prompt_input(&mut self, input: &str, pos: usize) {
        self.screen.print_prompt_input(input, pos);
    }

    fn print_send(&mut self, send: &crate::model::Line) {
        if let Some(line) = send.print_line() {
            self.tts_ctrl.lock().unwrap().speak_input(line);
        }
        self.screen.print_send(send);
    }

    fn reset(&mut self) -> Result<()> {
        self.screen.reset()
    }

    fn reset_scroll(&mut self) -> Result<()> {
        self.screen.reset_scroll()
    }

    fn scroll_down(&mut self) -> Result<()> {
        self.screen.scroll_down()
    }

    fn scroll_lock(&mut self, lock: bool) -> Result<()> {
        self.screen.scroll_lock(lock)
    }

    fn scroll_to(&mut self, row: usize) -> Result<()> {
        self.screen.scroll_to(row)
    }

    fn scroll_top(&mut self) -> Result<()> {
        self.screen.scroll_top()
    }

    fn scroll_up(&mut self) -> Result<()> {
        self.screen.scroll_up()
    }

    fn find_up(&mut self, pattern: &crate::model::Regex) -> Result<()> {
        self.screen.find_up(pattern)
    }

    fn find_down(&mut self, pattern: &crate::model::Regex) -> Result<()> {
        self.screen.find_down(pattern)
    }

    fn set_host(&mut self, host: &str, port: u16) -> Result<()> {
        self.screen.set_host(host, port)
    }

    fn add_tag(&mut self, proto: &str) -> Result<()> {
        self.screen.add_tag(proto)
    }

    fn clear_tags(&mut self) -> Result<()> {
        self.screen.clear_tags()
    }

    fn set_status_area_height(&mut self, height: u16) -> Result<()> {
        self.screen.set_status_area_height(height)
    }

    fn set_status_line(&mut self, line: usize, info: String) -> Result<()> {
        self.screen.set_status_line(line, info)
    }

    fn flush(&mut self) {
        self.screen.flush();
    }

    fn width(&self) -> u16 {
        self.screen.width()
    }

    fn height(&self) -> u16 {
        self.screen.height()
    }

    fn destroy(self: Box<Self>) -> Result<(Box<dyn Write>, History)> {
        self.screen.destroy()
    }
}
