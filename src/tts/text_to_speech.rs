use std::{sync::mpsc::channel, sync::mpsc::Receiver, sync::mpsc::Sender, thread};

use log::error;
use tts::TTS;

pub enum TTSEvent {
    Speak(String, bool),
    Shutdown,
}

pub fn spawn_tts_thread() -> Sender<TTSEvent> {
    let (tx, rx): (Sender<TTSEvent>, Receiver<TTSEvent>) = channel();
    thread::spawn(|| {
        let mut tts = TTS::default().unwrap();
        let rx = rx;
        while let Ok(event) = rx.recv() {
            match event {
                TTSEvent::Speak(msg, force) => {
                    if let Err(err) = tts.speak(msg, force) {
                        error!("[TTS]: {}", err.to_string());
                    }
                }
                TTSEvent::Shutdown => break,
            }
        }
        tts.speak("Shutting down TTS", false).unwrap();
    });
    tx
}
