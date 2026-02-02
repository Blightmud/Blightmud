use mlua::{Function, Table, UserData, UserDataMethods};

use crate::event::Event;

use super::{
    backend::Backend,
    constants::{BACKEND, FS_LISTENERS},
};

pub struct Fs {}

impl UserData for Fs {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("monitor", |ctx, (path, cb): (String, Function)| {
            let table: Table = ctx.named_registry_value(FS_LISTENERS)?;
            table.set(table.raw_len() + 1, cb)?;

            let backend: Backend = ctx.named_registry_value(BACKEND)?;
            backend.writer.send(Event::FSMonitor(path)).unwrap();
            Ok(())
        })
    }
}

#[cfg(test)]
mod test_fs {
    use std::sync::mpsc::{channel, Receiver, Sender};

    use mlua::{Lua, Table};

    use crate::{
        event::Event,
        lua::{
            backend::Backend,
            constants::{BACKEND, FS_LISTENERS},
        },
    };

    use super::Fs;

    fn setup_lua() -> (Lua, Receiver<Event>) {
        let (writer, reader): (Sender<Event>, Receiver<Event>) = channel();
        let backend = Backend::new(writer);
        let lua = Lua::new();
        lua.set_named_registry_value(BACKEND, backend).unwrap();
        let listener_table = lua.create_table().unwrap();
        lua.set_named_registry_value(FS_LISTENERS, listener_table)
            .unwrap();
        lua.globals().set("fs", Fs {}).unwrap();
        (lua, reader)
    }

    #[test]
    fn test_fs_monitor() {
        let (lua, reader) = setup_lua();
        lua.load("fs.monitor('/test/path', function() end)")
            .exec()
            .unwrap();
        assert_eq!(
            reader.recv(),
            Ok(Event::FSMonitor("/test/path".to_string()))
        );
    }

    #[test]
    fn test_fs_monitor_adds_listener() {
        let (lua, _reader) = setup_lua();
        lua.load("fs.monitor('/test/path', function() end)")
            .exec()
            .unwrap();
        let table: Table = lua.named_registry_value(FS_LISTENERS).unwrap();
        assert_eq!(table.raw_len(), 1);
    }

    #[test]
    fn test_fs_monitor_multiple_paths() {
        let (lua, reader) = setup_lua();
        lua.load("fs.monitor('/path1', function() end)")
            .exec()
            .unwrap();
        lua.load("fs.monitor('/path2', function() end)")
            .exec()
            .unwrap();

        assert_eq!(reader.recv(), Ok(Event::FSMonitor("/path1".to_string())));
        assert_eq!(reader.recv(), Ok(Event::FSMonitor("/path2".to_string())));

        let table: Table = lua.named_registry_value(FS_LISTENERS).unwrap();
        assert_eq!(table.raw_len(), 2);
    }
}
