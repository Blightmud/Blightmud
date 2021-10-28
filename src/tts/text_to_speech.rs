use std::{path::PathBuf, sync::mpsc::Sender};

use serde::{Deserialize, Serialize};

#[cfg(feature = "tts")]
use {
    super::speech_queue::SpeechQueue,
    anyhow::Result,
    log::{debug, error},
    std::{
        sync::mpsc::{channel, Receiver},
        thread,
    },
    tts::Tts as TTS,
};

use crate::{io::SaveData, model::Line};

#[derive(Debug, PartialEq, Clone)]
pub enum TTSEvent {
    Speak(String, bool),
    SpeakInput(String),
    SpeakDirect(String),
    Flush,
    SetRate(f32),
    ChangeRate(f32),
    EchoKeys(bool),
    KeyPress(char),
    Next(usize),
    Prev(usize),
    ScanBack(usize),
    ScanForward(usize),
    ScanBackToInput,
    ScanForwardToInput,
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
    pub echo_keys: bool,
    pub rate: f32,
}

impl SaveData for TTSSettings {
    fn is_pretty() -> bool {
        true
    }

    fn relative_path() -> PathBuf {
        crate::CONFIG_DIR.join("tts_settings.ron")
    }
}

impl TTSController {
    pub fn new(enabled: bool, no_thread: bool) -> Self {
        let rt = if !no_thread { spawn_tts_thread() } else { None };

        let settings = if !cfg!(test) {
            TTSSettings::load()
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

    fn reload_settings(&mut self) {
        self.settings = TTSSettings::load();
    }

    fn send(&self, event: TTSEvent) {
        if let Some(rt) = &self.rt {
            match event {
                TTSEvent::SetRate(_) | TTSEvent::ChangeRate(_) | TTSEvent::SpeakDirect(_) => {
                    rt.send(event).ok();
                }
                _ => {
                    if self.enabled {
                        rt.send(event).ok();
                    }
                }
            }
        }
    }

    pub fn handle(&mut self, event: TTSEvent) {
        match event {
            TTSEvent::ChangeRate(rate) => {
                self.reload_settings();
                self.settings.rate += rate;
                self.settings.save();
                self.send(event);
            }
            TTSEvent::SetRate(rate) => {
                self.reload_settings();
                self.settings.rate = rate;
                self.settings.save();
                self.send(event);
            }
            TTSEvent::EchoKeys(enabled) => {
                self.reload_settings();
                self.settings.echo_keys = enabled;
                self.settings.save();
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
            self.send(TTSEvent::SpeakDirect("Text to speech disabled".to_string()));
            self.send(TTSEvent::Flush);
        }
    }

    pub fn speak_line(&self, line: &Line) {
        if !line.flags.tts_gag {
            let speak = line.clean_line().trim();
            for l in speak.lines() {
                self.send(TTSEvent::Speak(l.to_string(), line.flags.tts_interrupt));
            }
        }
    }

    pub fn speak_input(&self, line: &str) {
        if self.enabled {
            self.flush();
            let input = line.trim();
            let speak = if !input.is_empty() {
                format!("input: {}", input)
            } else {
                "input: blank".to_string()
            };
            self.send(TTSEvent::SpeakInput(speak));
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

    while let Ok(event) = rx.recv() {
        debug!("[TTS]: Event: {:?}", event);
        match event {
            TTSEvent::Speak(msg, force) => {
                if let Some(msg) = queue.push(msg, force) {
                    if speak(tts, &msg, force) {
                        continue;
                    }
                }
            }
            TTSEvent::SpeakInput(msg) => {
                queue.push_input(msg);
            }
            TTSEvent::SpeakDirect(msg) => {
                if !msg.is_empty() {
                    tts.speak(msg, true).ok();
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
            TTSEvent::ScanBack(step) => {
                if let Some(msg) = queue.scan_back(step) {
                    if msg.is_empty() {
                        if speak(tts, "blank", true) {
                            continue;
                        }
                    } else if speak(tts, &msg, true) {
                        continue;
                    }
                }
            }
            TTSEvent::ScanForward(step) => {
                if let Some(msg) = queue.scan_forward(step) {
                    if msg.is_empty() {
                        if speak(tts, "blank", true) {
                            continue;
                        }
                    } else if speak(tts, &msg, true) {
                        continue;
                    }
                }
            }
            TTSEvent::ScanBackToInput => {
                if let Some(msg) = queue.scan_back_to_input() {
                    if msg.is_empty() {
                        if speak(tts, "blank", true) {
                            continue;
                        }
                    } else if speak(tts, &msg, true) {
                        continue;
                    }
                }
            }
            TTSEvent::ScanForwardToInput => {
                if let Some(msg) = queue.scan_forward_to_input() {
                    if msg.is_empty() {
                        if speak(tts, "blank", true) {
                            continue;
                        }
                    } else if speak(tts, &msg, true) {
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
                tts.set_rate(rate.clamp(-100.0, 100.0))?;
            }
            TTSEvent::ChangeRate(increment) => {
                tts.set_rate((tts.get_rate()? + increment).clamp(-100.0, 100.0))?;
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
    if cfg!(not(any(debug_assertions, test))) {
        println!(
            "spawn_tts_thread({}, {})",
            cfg!(debug_assertions),
            cfg!(test)
        );
        let (tx, rx): (Sender<TTSEvent>, Receiver<TTSEvent>) = channel();
        let ttx = tx.clone();
        thread::Builder::new()
            .name("tts-thread".to_string())
            .spawn(move || match TTS::default() {
                Ok(mut tts) => {
                    if let Err(err) = setup_callbacks(&mut tts, ttx) {
                        error!("[TTS]: {}", err.to_string());
                    }
                    if let Err(err) = run_tts(&mut tts, rx) {
                        error!("[TTS]: {}", err.to_string());
                    }
                }
                Err(err) => error!("[TTS]: {}", err.to_string()),
            })
            .unwrap();
        Some(tx)
    } else {
        None
    }
}

#[cfg(not(feature = "tts"))]
fn spawn_tts_thread() -> Option<Sender<TTSEvent>> {
    None
}
