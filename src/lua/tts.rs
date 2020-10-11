use std::sync::mpsc::Sender;

use rlua::{UserData, UserDataMethods};

use crate::{event::Event, tts::TTSEvent};

use super::constants::TTS_GAG_NEXT_TRIGGER_LINE;

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
        methods.add_method("stop", |_, this, _: ()| {
            this.writer.send(Event::SpeakStop).unwrap();
            Ok(())
        });
        methods.add_method("gag", |ctx, _, _: ()| {
            ctx.globals().set(TTS_GAG_NEXT_TRIGGER_LINE, true)
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
    }
}
