use std::{sync::mpsc::channel, sync::mpsc::Receiver, sync::mpsc::Sender, thread};

use log::{debug, error};
use regex::Regex;
use tts::TTS;

use crate::model::Line;

use anyhow::Result;

#[derive(Debug, PartialEq, Clone)]
pub enum TTSEvent {
    Speak(String, bool),
    Flush,
    SetRate(f32),
    ChangeRate(f32),
    Shutdown,
}

pub struct TTSController {
    rt: Sender<TTSEvent>,
    enabled: bool,
}

impl TTSController {
    pub fn new(enabled: bool) -> Self {
        let rt = spawn_tts_thread();
        if enabled {
            rt.send(TTSEvent::Speak("Text to speech enabled".to_string(), false))
                .ok();
        }
        Self { rt, enabled }
    }

    pub fn handle(&mut self, event: TTSEvent) {
        self.rt.send(event).ok();
    }

    pub fn enabled(&mut self, enabled: bool) {
        if enabled {
            self.rt
                .send(TTSEvent::Speak("Text to speech enabled".to_string(), false))
                .ok();
        } else {
            self.rt.send(TTSEvent::Flush).ok();
            self.rt
                .send(TTSEvent::Speak("Text to speech disabled".to_string(), true))
                .ok();
        }
        self.enabled = enabled;
    }

    pub fn speak_line(&self, line: &Line) {
        if (self.enabled && !line.flags.tts_gag) || line.flags.tts_force {
            let speak = line.clean_line().trim();
            if !speak.is_empty() {
                self.rt
                    .send(TTSEvent::Speak(speak.to_string(), line.flags.tts_interrupt))
                    .ok();
            }
        }
    }

    pub fn speak_input(&self, line: &str) {
        if self.enabled {
            self.flush();
            let input = line.trim();
            if !input.is_empty() {
                debug!("Speaking input: {}", input);
                let speak = format!("input: {}", input);
                self.rt.send(TTSEvent::Speak(speak, true)).ok();
            }
        }
    }

    pub fn speak(&self, msg: &str, interupt: bool) {
        self.rt
            .send(TTSEvent::Speak(msg.to_string(), interupt))
            .ok();
    }

    pub fn speak_info(&self, msg: &str) {
        if self.enabled {
            self.rt
                .send(TTSEvent::Speak(format!("info: {}", msg), false))
                .ok();
        }
    }

    pub fn speak_error(&self, msg: &str) {
        if self.enabled {
            self.rt
                .send(TTSEvent::Speak(format!("error: {}", msg), false))
                .ok();
        }
    }

    pub fn flush(&self) {
        if self.enabled {
            self.rt.send(TTSEvent::Flush).ok();
        }
    }

    pub fn shutdown(&self) {
        if self.enabled {
            self.rt.send(TTSEvent::Shutdown).ok();
        }
    }
}

fn run_tts(tts: &mut TTS, rx: Receiver<TTSEvent>) -> Result<()> {
    let rx = rx;
    let alphanum = Regex::new("[A-Za-z0-9]+").unwrap();
    tts.set_rate((tts.max_rate() - tts.normal_rate()) / 2.0)?;
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
        }
    }
    Ok(())
}

fn spawn_tts_thread() -> Sender<TTSEvent> {
    let (tx, rx): (Sender<TTSEvent>, Receiver<TTSEvent>) = channel();
    thread::spawn(|| match TTS::default() {
        Ok(mut tts) => {
            if let Err(err) = run_tts(&mut tts, rx) {
                error!("[TTS]: {}", err.to_string());
            }
        }
        Err(err) => error!("[TTS]: {}", err.to_string()),
    });
    tx
}
