use std::sync::mpsc::Sender;

use rlua::{UserData, UserDataMethods};

use crate::event::Event;

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
    }
}
