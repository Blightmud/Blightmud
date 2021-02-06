use rlua::{UserData, UserDataMethods};

use crate::event::Event;

use super::{backend::Backend, constants::BACKEND};

pub struct Audio {}

impl UserData for Audio {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function(
            "play_music",
            |ctx, (path, repeat): (String, Option<bool>)| {
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::PlayMusic(path, repeat.unwrap_or_default()))
                    .unwrap();
                Ok(())
            },
        );
        methods.add_function("stop_music", |ctx, ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::StopMusic).unwrap();
            Ok(())
        });
        methods.add_function("play_sfx", |ctx, path: String| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::PlaySFX(path)).unwrap();
            Ok(())
        });
        methods.add_function("stop_sfx", |ctx, ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::StopSFX).unwrap();
            Ok(())
        });
    }
}

#[cfg(test)]
mod test_player {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use super::*;
    use rlua::Lua;

    use crate::event::Event;
    use crate::lua::{backend::Backend, constants::BACKEND};

    fn assert_event(lua_code: &str, event: Event) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer);
        let audio = Audio {};
        let lua = Lua::new();
        lua.context(|ctx| {
            ctx.set_named_registry_value(BACKEND, backend).unwrap();
            ctx.globals().set("audio", audio).unwrap();
            ctx.load(lua_code).exec().unwrap();
        });

        assert_eq!(reader.recv(), Ok(event));
    }

    #[test]
    fn test_play_music() {
        assert_event(
            r#"audio.play_music("batman", true)"#,
            Event::PlayMusic("batman".to_string(), true),
        );
        assert_event(
            r#"audio.play_music("robin", false)"#,
            Event::PlayMusic("robin".to_string(), false),
        );
        assert_event(
            r#"audio.play_music("joker")"#,
            Event::PlayMusic("joker".to_string(), false),
        );
    }

    #[test]
    fn test_stop_music() {
        assert_event(r#"audio.stop_music()"#, Event::StopMusic);
    }

    #[test]
    fn test_play_sfx() {
        assert_event(
            r#"audio.play_sfx("batman")"#,
            Event::PlaySFX("batman".to_string()),
        );
    }

    #[test]
    fn test_stop_sfx() {
        assert_event(r#"audio.stop_sfx()"#, Event::StopSFX);
    }
}
