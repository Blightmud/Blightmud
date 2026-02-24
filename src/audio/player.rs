use std::{fs::File, io::BufReader};

use anyhow::Result;
use rodio::{source::Source, Player as RodioPlayer};

pub struct Player {
    _stream: Option<rodio::MixerDeviceSink>,
    music: Option<RodioPlayer>,
    sfx: Option<RodioPlayer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceOptions {
    pub repeat: bool,
    pub amplify: f32,
}

impl Default for SourceOptions {
    fn default() -> Self {
        Self {
            repeat: false,
            amplify: 1.0,
        }
    }
}

impl Player {
    pub fn new() -> Self {
        let mut music = None;
        let mut sfx = None;
        let mut stream = None;
        if let Ok(ostream) = rodio::DeviceSinkBuilder::open_default_sink() {
            music = Some(RodioPlayer::connect_new(ostream.mixer()));
            sfx = Some(RodioPlayer::connect_new(ostream.mixer()));
            stream = Some(ostream);
        }

        Self {
            _stream: stream,
            music,
            sfx,
        }
    }

    pub fn disabled() -> Self {
        Self {
            _stream: None,
            music: None,
            sfx: None,
        }
    }

    pub fn play_music(&mut self, fpath: &str, options: SourceOptions) -> Result<()> {
        if self.music.is_none() {
            if let Some(ostream) = &self._stream {
                self.music = Some(RodioPlayer::connect_new(ostream.mixer()));
            }
        }
        if let Some(music) = &self.music {
            let file = File::open(fpath)?;
            let source = rodio::Decoder::new(BufReader::new(file))?;
            let source = source.amplify(options.amplify);
            if options.repeat {
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

    pub fn play_sfx(&mut self, fpath: &str, options: SourceOptions) -> Result<()> {
        if self.sfx.is_none() {
            if let Some(ostream) = &self._stream {
                self.sfx = Some(RodioPlayer::connect_new(ostream.mixer()));
            }
        }
        if let Some(sfx) = &self.sfx {
            let file = File::open(fpath)?;
            let source = rodio::Decoder::new(BufReader::new(file))?;
            let source = source.amplify(options.amplify);
            sfx.append(source);
        }
        Ok(())
    }

    pub fn stop_sfx(&mut self) -> Result<()> {
        self.sfx = None;
        Ok(())
    }
}
