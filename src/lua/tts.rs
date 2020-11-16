use std::sync::mpsc::Sender;

use rlua::{UserData, UserDataMethods};

use crate::{event::Event, tts::TTSEvent};

pub struct Tts {
    writer: Sender<Event>,
}

impl Tts {
    pub fn new(writer: Sender<Event>) -> Self {
        Self { writer }
    }
}

impl UserData for Tts {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method(
            "speak",
            |_, this, (msg, interupt): (String, Option<bool>)| {
                this.writer
                    .send(Event::Speak(msg, interupt.unwrap_or_default()))
                    .unwrap();
                Ok(())
            },
        );
        methods.add_method("speak_direct", |_, this, msg: String| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::SpeakDirect(msg)))
                .unwrap();
            Ok(())
        });
        methods.add_method("stop", |_, this, _: ()| {
            this.writer.send(Event::SpeakStop).unwrap();
            Ok(())
        });
        methods.add_method("enable", |_, this, enabled: bool| {
            this.writer.send(Event::TTSEnabled(enabled)).unwrap();
            Ok(())
        });
        methods.add_method("set_rate", |_, this, rate: f64| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::SetRate(rate as f32)))
                .unwrap();
            Ok(())
        });
        methods.add_method("change_rate", |_, this, rate: f64| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::ChangeRate(rate as f32)))
                .unwrap();
            Ok(())
        });
        methods.add_method("echo_keypresses", |_, this, enabled: bool| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::EchoKeys(enabled)))
                .unwrap();
            Ok(())
        });
        methods.add_method("step_back", |_, this, step: usize| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::Prev(step)))
                .unwrap();
            Ok(())
        });
        methods.add_method("step_forward", |_, this, step: usize| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::Next(step)))
                .unwrap();
            Ok(())
        });
        methods.add_method("scan_back", |_, this, step: usize| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::ScanBack(step)))
                .unwrap();
            Ok(())
        });
        methods.add_method("scan_forward", |_, this, step: usize| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::ScanForward(step)))
                .unwrap();
            Ok(())
        });
        methods.add_method("scan_input_back", |_, this, _: ()| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::ScanBackToInput))
                .unwrap();
            Ok(())
        });
        methods.add_method("scan_input_forward", |_, this, _: ()| {
            this.writer
                .send(Event::TTSEvent(TTSEvent::ScanForwardToInput))
                .unwrap();
            Ok(())
        });
        methods.add_method("step_begin", |_, this, _: ()| {
            this.writer.send(Event::TTSEvent(TTSEvent::Begin)).unwrap();
            Ok(())
        });
        methods.add_method("step_end", |_, this, _: ()| {
            this.writer.send(Event::TTSEvent(TTSEvent::End)).unwrap();
            Ok(())
        });
    }
}
