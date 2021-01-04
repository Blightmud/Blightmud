use rlua::{UserData, UserDataMethods};

use crate::{event::Event, tts::TTSEvent};

use super::{backend::Backend, constants::BACKEND};

pub struct Tts {}

impl Tts {
    pub fn new() -> Self {
        Self {}
    }
}

impl UserData for Tts {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function(
            "speak",
            |ctx, (msg, interupt): (String, Option<bool>)| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                    .send(Event::Speak(msg, interupt.unwrap_or_default()))
                    .unwrap();
                Ok(())
            },
        );
        methods.add_function("speak_direct", |ctx, msg: String| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::SpeakDirect(msg)))
                .unwrap();
            Ok(())
        });
        methods.add_function("stop", |ctx, _: ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::SpeakStop).unwrap();
            Ok(())
        });
        methods.add_function("enable", |ctx, enabled: bool| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::TTSEnabled(enabled)).unwrap();
            Ok(())
        });
        methods.add_function("set_rate", |ctx, rate: f64| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::SetRate(rate as f32)))
                .unwrap();
            Ok(())
        });
        methods.add_function("change_rate", |ctx, rate: f64| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::ChangeRate(rate as f32)))
                .unwrap();
            Ok(())
        });
        methods.add_function("echo_keypresses", |ctx, enabled: bool| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::EchoKeys(enabled)))
                .unwrap();
            Ok(())
        });
        methods.add_function("step_back", |ctx, step: usize| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::Prev(step)))
                .unwrap();
            Ok(())
        });
        methods.add_function("step_forward", |ctx, step: usize| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::Next(step)))
                .unwrap();
            Ok(())
        });
        methods.add_function("scan_back", |ctx, step: usize| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::ScanBack(step)))
                .unwrap();
            Ok(())
        });
        methods.add_function("scan_forward", |ctx, step: usize| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::ScanForward(step)))
                .unwrap();
            Ok(())
        });
        methods.add_function("scan_input_back", |ctx, _: ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::ScanBackToInput))
                .unwrap();
            Ok(())
        });
        methods.add_function("scan_input_forward", |ctx, _: ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer
                .send(Event::TTSEvent(TTSEvent::ScanForwardToInput))
                .unwrap();
            Ok(())
        });
        methods.add_function("step_begin", |ctx, _: ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::TTSEvent(TTSEvent::Begin)).unwrap();
            Ok(())
        });
        methods.add_function("step_end", |ctx, _: ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::TTSEvent(TTSEvent::End)).unwrap();
            Ok(())
        });
    }
}
