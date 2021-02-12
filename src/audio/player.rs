use std::{fs::File, io::BufReader};

use anyhow::Result;
use cpal::traits::HostTrait;
use rodio::{Sink, Source};

pub struct Player {
    _stream: Option<rodio::OutputStream>,
    handle: Option<rodio::OutputStreamHandle>,
    music: Option<Sink>,
    sfx: Option<Sink>,
}

impl Player {
    pub fn new() -> Self {
        let mut music = None;
        let mut sfx = None;
        let mut stream = None;
        let mut handle = None;

        let host = cpal::default_host();
        if host.default_output_device().is_some() {
            if let Ok((ostream, ohandle)) = rodio::OutputStream::try_default() {
                music = rodio::Sink::try_new(&ohandle).ok();
                sfx = rodio::Sink::try_new(&ohandle).ok();
                stream = Some(ostream);
                handle = Some(ohandle);
            }
        }

        Self {
            _stream: stream,
            handle,
            music,
            sfx,
        }
    }

    pub fn play_music(&mut self, fpath: &str, repeat: bool) -> Result<()> {
        if self.music.is_none() {
            if let Some(handle) = &self.handle {
                self.music = rodio::Sink::try_new(&handle).ok();
            }
        }
        if let Some(music) = &self.music {
            let file = File::open(fpath)?;
            let source = rodio::Decoder::new(BufReader::new(file))?;
            if repeat {
                music.append(source.repeat_infinite());
            } else {
                music.append(source);
            }
            music.play();
        }
        Ok(())
    }

    pub fn stop_music(&mut self) -> Result<()> {
        self.music = None;
        Ok(())
    }

    pub fn play_sfx(&mut self, fpath: &str) -> Result<()> {
        if self.sfx.is_none() {
            if let Some(handle) = &self.handle {
                self.sfx = rodio::Sink::try_new(&handle).ok();
            }
        }
        if let Some(sfx) = &self.sfx {
            let file = File::open(fpath)?;
            let source = rodio::Decoder::new(BufReader::new(file))?;
            sfx.append(source);
        }
        Ok(())
    }

    pub fn stop_sfx(&mut self) -> Result<()> {
        self.sfx = None;
        Ok(())
    }
}
