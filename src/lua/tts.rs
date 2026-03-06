use mlua::{AnyUserData, MetaMethod, UserData, UserDataMethods};

use crate::{event::Event, tts::TTSEvent};

use super::{backend::Backend, constants::BACKEND};

#[derive(Clone)]
pub struct Tts {
    pub enabled: bool,
}

impl Tts {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl UserData for Tts {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("is_available", |_, _: ()| Ok(cfg!(feature = "tts")));
        if cfg!(feature = "tts") {
            methods.add_function("speak", |ctx, (msg, interupt): (String, Option<bool>)| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::Speak(msg, interupt.unwrap_or_default()))
                    .unwrap();
                Ok(())
            });
            methods.add_function("speak_direct", |ctx, msg: String| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
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
            methods.add_function("is_enabled", |ctx, ()| {
                let tts_aud: AnyUserData = ctx.globals().get("tts")?;
                let tts = tts_aud.borrow::<Tts>()?;
                Ok(tts.enabled)
            });
            methods.add_function("set_rate", |ctx, rate: f64| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::SetRate(rate as f32)))
                    .unwrap();
                Ok(())
            });
            methods.add_function("change_rate", |ctx, rate: f64| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::ChangeRate(rate as f32)))
                    .unwrap();
                Ok(())
            });
            methods.add_function("echo_keypresses", |ctx, enabled: bool| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::EchoKeys(enabled)))
                    .unwrap();
                Ok(())
            });
            methods.add_function("step_back", |ctx, step: usize| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::Prev(step)))
                    .unwrap();
                Ok(())
            });
            methods.add_function("step_forward", |ctx, step: usize| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::Next(step)))
                    .unwrap();
                Ok(())
            });
            methods.add_function("scan_back", |ctx, step: usize| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::ScanBack(step)))
                    .unwrap();
                Ok(())
            });
            methods.add_function("scan_forward", |ctx, step: usize| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::ScanForward(step)))
                    .unwrap();
                Ok(())
            });
            methods.add_function("scan_input_back", |ctx, _: ()| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::ScanBackToInput))
                    .unwrap();
                Ok(())
            });
            methods.add_function("scan_input_forward", |ctx, _: ()| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::ScanForwardToInput))
                    .unwrap();
                Ok(())
            });
            methods.add_function("step_begin", |ctx, _: ()| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::TTSEvent(TTSEvent::Begin))
                    .unwrap();
                Ok(())
            });
            methods.add_function("step_end", |ctx, _: ()| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend.writer.send(Event::TTSEvent(TTSEvent::End)).unwrap();
                Ok(())
            });
        } else {
            methods.add_meta_function(MetaMethod::Index, |ctx, _: ()| {
                let func: mlua::Function = ctx.load("function () end").eval()?;
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::Error(
                        "Blightmud was not compiled with text-to-speech enabled".to_string(),
                    ))
                    .unwrap();
                Ok(func)
            });
            methods.add_meta_function_mut(MetaMethod::Index, |ctx, _: ()| {
                let func: mlua::Function = ctx.load("function () end").eval()?;
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::Error(
                        "Blightmud was not compiled with text-to-speech enabled".to_string(),
                    ))
                    .unwrap();
                Ok(func)
            });
        }
    }
}

#[cfg(test)]
mod test_tts {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use mlua::Lua;

    use crate::{
        event::Event,
        lua::{backend::Backend, constants::BACKEND},
    };

    use super::Tts;

    fn setup_lua(enabled: bool) -> (Lua, Receiver<Event>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer);
        let lua = Lua::new();
        lua.set_named_registry_value(BACKEND, backend).unwrap();
        let tts = Tts::new(enabled);
        lua.globals().set("tts", tts).unwrap();
        (lua, reader)
    }

    #[test]
    fn test_tts_new_enabled() {
        let tts = Tts::new(true);
        assert!(tts.enabled);
    }

    #[test]
    fn test_tts_new_disabled() {
        let tts = Tts::new(false);
        assert!(!tts.enabled);
    }

    #[test]
    fn test_tts_clone() {
        let tts = Tts::new(true);
        let cloned = tts.clone();
        assert_eq!(tts.enabled, cloned.enabled);
    }

    #[test]
    fn test_tts_is_available() {
        let (lua, _reader) = setup_lua(false);
        let is_available: bool = lua.load("return tts.is_available()").call(()).unwrap();
        // This will be false in normal builds, true if compiled with tts feature
        assert_eq!(is_available, cfg!(feature = "tts"));
    }

    #[cfg(feature = "tts")]
    mod tts_enabled_tests {
        use super::*;
        use crate::tts::TTSEvent;

        #[test]
        fn test_tts_speak() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.speak('hello')").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::Speak("hello".to_string(), false)));
        }

        #[test]
        fn test_tts_speak_with_interrupt() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.speak('hello', true)").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::Speak("hello".to_string(), true)));
        }

        #[test]
        fn test_tts_speak_direct() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.speak_direct('direct message')")
                .exec()
                .unwrap();
            assert_eq!(
                reader.recv(),
                Ok(Event::TTSEvent(TTSEvent::SpeakDirect(
                    "direct message".to_string()
                )))
            );
        }

        #[test]
        fn test_tts_stop() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.stop()").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::SpeakStop));
        }

        #[test]
        fn test_tts_enable() {
            let (lua, reader) = setup_lua(false);
            lua.load("tts.enable(true)").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEnabled(true)));
        }

        #[test]
        fn test_tts_disable() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.enable(false)").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEnabled(false)));
        }

        #[test]
        fn test_tts_is_enabled() {
            let (lua, _reader) = setup_lua(true);
            let is_enabled: bool = lua.load("return tts.is_enabled()").call(()).unwrap();
            assert!(is_enabled);
        }

        #[test]
        fn test_tts_is_disabled() {
            let (lua, _reader) = setup_lua(false);
            let is_enabled: bool = lua.load("return tts.is_enabled()").call(()).unwrap();
            assert!(!is_enabled);
        }

        #[test]
        fn test_tts_set_rate() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.set_rate(1.5)").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEvent(TTSEvent::SetRate(1.5))));
        }

        #[test]
        fn test_tts_change_rate() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.change_rate(0.5)").exec().unwrap();
            assert_eq!(
                reader.recv(),
                Ok(Event::TTSEvent(TTSEvent::ChangeRate(0.5)))
            );
        }

        #[test]
        fn test_tts_echo_keypresses() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.echo_keypresses(true)").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEvent(TTSEvent::EchoKeys(true))));
        }

        #[test]
        fn test_tts_step_back() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.step_back(5)").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEvent(TTSEvent::Prev(5))));
        }

        #[test]
        fn test_tts_step_forward() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.step_forward(3)").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEvent(TTSEvent::Next(3))));
        }

        #[test]
        fn test_tts_scan_back() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.scan_back(2)").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEvent(TTSEvent::ScanBack(2))));
        }

        #[test]
        fn test_tts_scan_forward() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.scan_forward(4)").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEvent(TTSEvent::ScanForward(4))));
        }

        #[test]
        fn test_tts_scan_input_back() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.scan_input_back()").exec().unwrap();
            assert_eq!(
                reader.recv(),
                Ok(Event::TTSEvent(TTSEvent::ScanBackToInput))
            );
        }

        #[test]
        fn test_tts_scan_input_forward() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.scan_input_forward()").exec().unwrap();
            assert_eq!(
                reader.recv(),
                Ok(Event::TTSEvent(TTSEvent::ScanForwardToInput))
            );
        }

        #[test]
        fn test_tts_step_begin() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.step_begin()").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEvent(TTSEvent::Begin)));
        }

        #[test]
        fn test_tts_step_end() {
            let (lua, reader) = setup_lua(true);
            lua.load("tts.step_end()").exec().unwrap();
            assert_eq!(reader.recv(), Ok(Event::TTSEvent(TTSEvent::End)));
        }
    }
}
