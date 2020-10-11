use std::{path::PathBuf, sync::mpsc::Sender};

use serde::{Deserialize, Serialize};

#[cfg(feature = "tts")]
use {
    anyhow::Result,
    log::debug,
    log::error,
    regex::Regex,
    std::{
        sync::mpsc::{channel, Receiver},
        thread,
    },
    tts::TTS,
};

use crate::{io::SaveData, model::Line};

#[derive(Debug, PartialEq, Clone)]
pub enum TTSEvent {
    Speak(String, bool),
    Flush,
    SetRate(f32),
    ChangeRate(f32),
    EchoKeys(bool),
    KeyPress(char),
    Shutdown,
}

pub struct TTSController {
    rt: Option<Sender<TTSEvent>>,
    enabled: bool,
    pub settings: TTSSettings,
}

#[derive(Default, Serialize, Deserialize)]
pub struct TTSSettings {
    echo_keys: bool,
    rate: f32,
}

impl SaveData for TTSSettings {
    fn relative_path() -> PathBuf {
        PathBuf::from("data/tts_settings.ron")
    }
}

impl TTSController {
    pub fn new(enabled: bool) -> Self {
        let rt = spawn_tts_thread();
        let settings = TTSSettings::load().unwrap_or_default();
        let tts_ctrl = Self {
            rt,
            enabled,
            settings,
        };
        tts_ctrl.send(TTSEvent::SetRate(tts_ctrl.settings.rate));
        tts_ctrl.send(TTSEvent::Speak("Text to speech enabled".to_string(), false));
        tts_ctrl
    }

    fn send(&self, event: TTSEvent) {
        if self.enabled {
            if let Some(rt) = &self.rt {
                rt.send(event).ok();
            }
        }
    }

    pub fn handle(&mut self, event: TTSEvent) {
        match event {
            TTSEvent::ChangeRate(rate) => {
                self.settings.rate += rate;
                self.send(event);
            }
            TTSEvent::SetRate(rate) => {
                self.settings.rate = rate;
                self.send(event);
            }
            TTSEvent::EchoKeys(enabled) => {
                self.settings.echo_keys = enabled;
            }
            _ => {
                self.send(event);
            }
        }
    }

    pub fn key_press(&mut self, key: char) {
        if self.settings.echo_keys {
            self.send(TTSEvent::KeyPress(key));
        }
    }

    pub fn enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if enabled {
            self.send(TTSEvent::Speak("Text to speech enabled".to_string(), false));
        } else {
            self.send(TTSEvent::Flush);
            self.send(TTSEvent::Speak("Text to speech disabled".to_string(), true));
        }
    }

    pub fn speak_line(&self, line: &Line) {
        if (self.enabled && !line.flags.tts_gag) || line.flags.tts_force {
            let speak = line.clean_line().trim();
            if !speak.is_empty() {
                self.send(TTSEvent::Speak(speak.to_string(), line.flags.tts_interrupt));
            }
        }
    }

    pub fn speak_input(&self, line: &str) {
        if self.enabled {
            self.flush();
            let input = line.trim();
            if !input.is_empty() {
                let speak = format!("input: {}", input);
                self.send(TTSEvent::Speak(speak, true));
            }
        }
    }

    pub fn speak(&self, msg: &str, interupt: bool) {
        self.send(TTSEvent::Speak(msg.to_string(), interupt));
    }

    pub fn speak_info(&self, msg: &str) {
        self.send(TTSEvent::Speak(format!("info: {}", msg), false));
    }

    pub fn speak_error(&self, msg: &str) {
        self.send(TTSEvent::Speak(format!("error: {}", msg), false));
    }

    pub fn flush(&self) {
        self.send(TTSEvent::Flush);
    }

    pub fn shutdown(&self) {
        self.settings.save().ok();
        if let Some(rt) = &self.rt {
            rt.send(TTSEvent::Shutdown).ok();
        }
    }
}

#[cfg(feature = "tts")]
fn run_tts(tts: &mut TTS, rx: Receiver<TTSEvent>) -> Result<()> {
    let rx = rx;
    let alphanum = Regex::new("[A-Za-z0-9]+").unwrap();
    while let Ok(event) = rx.recv() {
        debug!("[TTS]: Event: {:?}", event);
        match event {
            TTSEvent::Speak(msg, force) => {
                if msg.is_empty() || !alphanum.is_match(&msg) {
                    continue;
                }
                if let Err(err) = tts.speak(msg, force) {
                    error!("[TTS]: {}", err.to_string());
                    continue;
                }
            }
            TTSEvent::Flush => {
                tts.stop().unwrap();
            }
            TTSEvent::SetRate(rate) => {
                tts.set_rate(rate.min(100.0).max(-100.0))?;
            }
            TTSEvent::ChangeRate(increment) => {
                tts.set_rate((tts.get_rate()? + increment).min(100.0).max(-100.0))?;
            }
            TTSEvent::Shutdown => {
                tts.stop().unwrap();
                break;
            }
            TTSEvent::KeyPress(key) => {
                tts.speak(key, true)?;
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(feature = "tts")]
fn spawn_tts_thread() -> Option<Sender<TTSEvent>> {
    let (tx, rx): (Sender<TTSEvent>, Receiver<TTSEvent>) = channel();
    thread::spawn(|| match TTS::default() {
        Ok(mut tts) => {
            if let Err(err) = run_tts(&mut tts, rx) {
                error!("[TTS]: {}", err.to_string());
            }
        }
        Err(err) => error!("[TTS]: {}", err.to_string()),
    });
    Some(tx)
}

#[cfg(not(feature = "tts"))]
fn spawn_tts_thread() -> Option<Sender<TTSEvent>> {
    None
}
