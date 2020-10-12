use std::{path::PathBuf, sync::mpsc::Sender};

use serde::{Deserialize, Serialize};

#[cfg(feature = "tts")]
use {
    super::speech_queue::SpeechQueue,
    anyhow::Result,
    log::{debug, error},
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
    Next(usize),
    Prev(usize),
    Begin,
    End,
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
        let settings = if !cfg!(test) {
            TTSSettings::load().unwrap_or_default()
        } else {
            TTSSettings::default()
        };
        let tts_ctrl = Self {
            rt,
            enabled,
            settings,
        };

        if let Some(rt) = &tts_ctrl.rt {
            rt.send(TTSEvent::SetRate(tts_ctrl.settings.rate)).ok();
        }
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
                for l in speak.lines() {
                    self.send(TTSEvent::Speak(l.to_string(), line.flags.tts_interrupt));
                }
            }
        }
    }

    pub fn speak_input(&self, line: &str) {
        if self.enabled {
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
        if let Some(rt) = &self.rt {
            if !cfg!(test) {
                self.settings.save().ok();
            }
            rt.send(TTSEvent::Shutdown).ok();
        }
    }
}

#[inline]
#[cfg(feature = "tts")]
fn speak(tts: &mut TTS, msg: &str, force: bool) -> bool {
    if let Err(err) = tts.speak(msg, force) {
        error!("[TTS]: {}", err.to_string());
        true
    } else {
        false
    }
}

#[cfg(feature = "tts")]
fn run_tts(tts: &mut TTS, rx: Receiver<TTSEvent>) -> Result<()> {
    let mut queue = SpeechQueue::new(1000);
    let rx = rx;
    let alphanum = Regex::new("[A-Za-z0-9]+").unwrap();

    while let Ok(event) = rx.recv() {
        debug!("[TTS]: Event: {:?}", event);
        match event {
            TTSEvent::Speak(msg, force) => {
                if msg.is_empty() || !alphanum.is_match(&msg) {
                    continue;
                }
                if let Some(msg) = queue.push(msg, force) {
                    if speak(tts, &msg, force) {
                        continue;
                    }
                }
            }
            TTSEvent::Next(step) => {
                if let Some(msg) = queue.next(step) {
                    if speak(tts, &msg, true) {
                        continue;
                    }
                }
            }
            TTSEvent::Prev(step) => {
                if let Some(msg) = queue.prev(step) {
                    if speak(tts, &msg, true) {
                        continue;
                    }
                }
            }
            TTSEvent::Begin => {
                if let Some(msg) = queue.current() {
                    if speak(tts, &msg, true) {
                        continue;
                    }
                }
            }
            TTSEvent::End => {
                if let Some(msg) = queue.next(1) {
                    if speak(tts, &msg, true) {
                        continue;
                    }
                }
            }
            TTSEvent::Flush => {
                queue.flush();
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
fn setup_callbacks(tts: &mut TTS, tx: Sender<TTSEvent>) -> Result<(), tts::Error> {
    tts.on_utterance_end(Some(Box::new(move |_| {
        tx.send(TTSEvent::Next(1)).ok();
    })))
}

#[cfg(feature = "tts")]
fn spawn_tts_thread() -> Option<Sender<TTSEvent>> {
    if !cfg!(test) {
        let (tx, rx): (Sender<TTSEvent>, Receiver<TTSEvent>) = channel();
        let ttx = tx.clone();
        thread::spawn(move || match TTS::default() {
            Ok(mut tts) => {
                if let Err(err) = setup_callbacks(&mut tts, ttx) {
                    error!("[TTS]: {}", err.to_string());
                }
                if let Err(err) = run_tts(&mut tts, rx) {
                    error!("[TTS]: {}", err.to_string());
                }
            }
            Err(err) => error!("[TTS]: {}", err.to_string()),
        });
        Some(tx)
    } else {
        None
    }
}

#[cfg(not(feature = "tts"))]
fn spawn_tts_thread() -> Option<Sender<TTSEvent>> {
    None
}
