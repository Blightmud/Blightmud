use std::{fs::File, io::BufReader};

use anyhow::Result;
use rodio::{Sink, Source};

pub struct Player {
    _stream: Option<rodio::OutputStream>,
    _handle: Option<rodio::OutputStreamHandle>,
    music: Option<Sink>,
    sfx: Option<Sink>,
}

impl Player {
    pub fn new() -> Self {
        let mut music = None;
        let mut sfx = None;
        let mut stream = None;
        let mut handle = None;
        if let Ok((ostream, ohandle)) = rodio::OutputStream::try_default() {
            music = rodio::Sink::try_new(&ohandle).ok();
            sfx = rodio::Sink::try_new(&ohandle).ok();
            stream = Some(ostream);
            handle = Some(ohandle);
        }

        Self {
            _stream: stream,
            _handle: handle,
            music,
            sfx,
        }
    }

    pub fn play_music(&self, fpath: &str, repeat: bool) -> Result<()> {
        if let Some(music) = &self.music {
            let file = File::open(fpath)?;
            let source = rodio::Decoder::new(BufReader::new(file))?;
            if repeat {
                music.append(source.repeat_infinite());
            } else {
                music.append(source);
            }
        }
        Ok(())
    }

    pub fn stop_music(&self) -> Result<()> {
        if let Some(music) = &self.music {
            music.stop();
        }
        Ok(())
    }

    pub fn play_sfx(&self, fpath: &str) -> Result<()> {
        if let Some(sfx) = &self.sfx {
            let file = File::open(fpath)?;
            let source = rodio::Decoder::new(BufReader::new(file))?;
            sfx.append(source);
        }
        Ok(())
    }

    pub fn stop_sfx(&self) -> Result<()> {
        if let Some(sfx) = &self.sfx {
            sfx.stop();
        }
        Ok(())
    }
}
