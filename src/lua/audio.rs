use mlua::{Table, UserData, UserDataMethods};

use crate::{audio::SourceOptions, event::Event};

use super::{backend::Backend, constants::BACKEND};

fn parse_audio_options(opts: &Option<Table>) -> SourceOptions {
    let mut options = SourceOptions::default();
    if let Some(opts) = &opts {
        options.repeat = opts.get("loop").unwrap_or(options.repeat);
        options.amplify = opts.get("amplify").unwrap_or(options.amplify);
    }
    options
}

pub struct Audio {}

impl UserData for Audio {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function(
            "play_music",
            |ctx, (path, opts): (String, Option<Table>)| {
                let options = parse_audio_options(&opts);
                let backend: Backend = ctx.named_registry_value(BACKEND)?;
                backend
                    .writer
                    .send(Event::PlayMusic(path, options))
                    .unwrap();
                Ok(())
            },
        );
        methods.add_function("stop_music", |ctx, ()| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::StopMusic).unwrap();
            Ok(())
        });
        methods.add_function("play_sfx", |ctx, (path, opts): (String, Option<Table>)| {
            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            let options = parse_audio_options(&opts);
            backend.writer.send(Event::PlaySFX(path, options)).unwrap();
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
    use mlua::Lua;

    use crate::event::Event;
    use crate::lua::{backend::Backend, constants::BACKEND};

    fn assert_event(lua_code: &str, event: Event) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer);
        let audio = Audio {};
        let lua = Lua::new();
        lua.set_named_registry_value(BACKEND, backend).unwrap();
        lua.globals().set("audio", audio).unwrap();
        lua.load(lua_code).exec().unwrap();

        assert_eq!(reader.recv(), Ok(event));
    }

    #[test]
    fn test_play_music() {
        assert_event(
            r#"audio.play_music("batman")"#,
            Event::PlayMusic("batman".to_string(), SourceOptions::default()),
        );
        assert_event(
            r#"audio.play_music("robin", {})"#,
            Event::PlayMusic("robin".to_string(), SourceOptions::default()),
        );
        assert_event(
            r#"audio.play_music("joker")"#,
            Event::PlayMusic("joker".to_string(), SourceOptions::default()),
        );
    }

    #[test]
    fn test_options() {
        assert_event(
            r#"audio.play_sfx("test", { loop=false, amplify=0.5 })"#,
            Event::PlaySFX(
                "test".to_string(),
                SourceOptions {
                    repeat: false,
                    amplify: 0.5,
                },
            ),
        );
        assert_event(
            r#"audio.play_music("test", { loop=true, amplify=2.5 })"#,
            Event::PlayMusic(
                "test".to_string(),
                SourceOptions {
                    repeat: true,
                    amplify: 2.5,
                },
            ),
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
            Event::PlaySFX("batman".to_string(), SourceOptions::default()),
        );
    }

    #[test]
    fn test_stop_sfx() {
        assert_event(r#"audio.stop_sfx()"#, Event::StopSFX);
    }
}
