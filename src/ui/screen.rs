use std::io::{stdout, Write};

use termion::{input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};

use crate::{
    io::SaveData,
    model::{Settings, MOUSE_ENABLED, READER_MODE},
    session::Session,
};

use super::{history::History, ReaderScreen, SplitScreen, UserInterface};
use anyhow::Result;

/// Creates the io::Write terminal handler we draw to.
pub fn create_screen_writer(mouse_support: bool) -> Result<Box<dyn Write>> {
    let screen = AlternateScreen::from(stdout().into_raw_mode()?);
    if mouse_support {
        Ok(Box::new(MouseTerminal::from(screen)))
    } else {
        Ok(Box::new(screen))
    }
}

pub fn create_screen(session: &Session, force_reader_mode: bool) -> Result<Box<dyn UserInterface>> {
    let settings = Settings::try_load()?;
    let reader_mode = settings.get(READER_MODE)? || force_reader_mode;
    let screen: Box<dyn UserInterface> = if reader_mode {
        Box::new(ReaderScreen::new(
            create_screen_writer(false)?,
            History::new(),
        )?)
    } else {
        Box::new(SplitScreen::new(
            create_screen_writer(settings.get(MOUSE_ENABLED)?)?,
            History::new(),
            session.tts_ctrl.clone(),
        )?)
    };

    Ok(screen)
}

pub fn switch_screen(
    screen: Box<dyn UserInterface>,
    session: &mut Session,
    reader_mode: bool,
) -> Result<Box<dyn UserInterface>> {
    let (screen, history) = screen.destroy()?;
    let mut screen: Box<dyn UserInterface> = if reader_mode {
        Box::new(ReaderScreen::new(screen, history)?)
    } else {
        Box::new(SplitScreen::new(screen, history, session.tts_ctrl.clone())?)
    };
    screen.setup()?;
    Ok(screen)
}
