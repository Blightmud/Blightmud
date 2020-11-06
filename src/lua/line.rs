use rlua::{UserData, UserDataMethods};

use crate::model::Line as mLine;

#[derive(Clone)]
pub struct Line {
    pub inner: mLine,
}

impl From<mLine> for Line {
    fn from(inner: mLine) -> Self {
        Self { inner }
    }
}

impl UserData for Line {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("line", |_, this, _: ()| -> rlua::Result<String> {
            Ok(this.inner.clean_line().to_string())
        });
        methods.add_method("raw", |_, this, _: ()| -> rlua::Result<String> {
            Ok(this.inner.line().to_string())
        });
        methods.add_method_mut("gag", |_, this, gag: Option<bool>| -> rlua::Result<bool> {
            if let Some(gag) = gag {
                this.inner.flags.gag = gag;
            }
            Ok(this.inner.flags.gag)
        });
        methods.add_method_mut(
            "tts_gag",
            |_, this, gag: Option<bool>| -> rlua::Result<bool> {
                if let Some(gag) = gag {
                    this.inner.flags.tts_gag = gag;
                }
                Ok(this.inner.flags.tts_gag)
            },
        );
        methods.add_method_mut(
            "tts_interrupt",
            |_, this, val: Option<bool>| -> rlua::Result<bool> {
                if let Some(val) = val {
                    this.inner.flags.tts_interrupt = val;
                }
                Ok(this.inner.flags.tts_interrupt)
            },
        );
        methods.add_method_mut(
            "skip_log",
            |_, this, val: Option<bool>| -> rlua::Result<bool> {
                if let Some(val) = val {
                    this.inner.flags.skip_log = val;
                }
                Ok(this.inner.flags.skip_log)
            },
        );
        methods.add_method("prompt", |_, this, _: ()| -> rlua::Result<bool> {
            Ok(this.inner.flags.prompt)
        });
        methods.add_method_mut(
            "matched",
            |_, this, val: Option<bool>| -> rlua::Result<bool> {
                if let Some(val) = val {
                    this.inner.flags.matched = val;
                }
                Ok(this.inner.flags.matched)
            },
        );
    }
}

#[cfg(test)]
mod test_lua_line {
    use rlua::Lua;

    use super::Line;
    use crate::model::Line as mLine;

    fn get_lua() -> Lua {
        let state = Lua::new();
        state.context(|ctx| {
            ctx.globals()
                .set(
                    "test_line",
                    Line::from(mLine::from("\x1b[31mA testing line\x1b[0m")),
                )
                .unwrap();
        });
        state
    }

    #[test]
    fn test_content() {
        let state = get_lua();
        assert_eq!(
            state
                .context(|ctx| -> String { ctx.load("return test_line:line()").call(()).unwrap() }),
            "A testing line"
        );
        assert_eq!(
            state.context(|ctx| -> String { ctx.load("return test_line:raw()").call(()).unwrap() }),
            "\x1b[31mA testing line\x1b[0m"
        );
    }

    #[test]
    fn test_gag() {
        let state = get_lua();
        assert!(
            !state.context(|ctx| -> bool { ctx.load("return test_line:gag()").call(()).unwrap() })
        );
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(!line.inner.flags.gag);
        assert!(state
            .context(|ctx| -> bool { ctx.load("return test_line:gag(true)").call(()).unwrap() }));
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(line.inner.flags.gag);
    }

    #[test]
    fn test_tts_gag() {
        let state = get_lua();
        assert!(!state
            .context(|ctx| -> bool { ctx.load("return test_line:tts_gag()").call(()).unwrap() }));
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(!line.inner.flags.tts_gag);
        assert!(state.context(|ctx| -> bool {
            ctx.load("return test_line:tts_gag(true)").call(()).unwrap()
        }));
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(line.inner.flags.tts_gag);
    }

    #[test]
    fn test_tts_interrupt() {
        let state = get_lua();
        assert!(!state.context(|ctx| -> bool {
            ctx.load("return test_line:tts_interrupt()")
                .call(())
                .unwrap()
        }));
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(!line.inner.flags.tts_interrupt);
        assert!(state.context(|ctx| -> bool {
            ctx.load("return test_line:tts_interrupt(true)")
                .call(())
                .unwrap()
        }));
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(line.inner.flags.tts_interrupt);
    }

    #[test]
    fn test_skip_log() {
        let state = get_lua();
        assert!(!state
            .context(|ctx| -> bool { ctx.load("return test_line:skip_log()").call(()).unwrap() }));
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(!line.inner.flags.skip_log);
        assert!(state.context(|ctx| -> bool {
            ctx.load("return test_line:skip_log(true)")
                .call(())
                .unwrap()
        }));
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(line.inner.flags.skip_log);
    }

    #[test]
    fn test_matched() {
        let state = get_lua();
        assert!(!state
            .context(|ctx| -> bool { ctx.load("return test_line:matched()").call(()).unwrap() }));
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(!line.inner.flags.matched);
        assert!(state.context(|ctx| -> bool {
            ctx.load("return test_line:matched(true)").call(()).unwrap()
        }));
        let line = state.context(|ctx| -> Line { ctx.globals().get("test_line").unwrap() });
        assert!(line.inner.flags.matched);
    }

    #[test]
    fn test_prompt() {
        let state = get_lua();
        assert!(!state
            .context(|ctx| -> bool { ctx.load("return test_line:prompt()").call(()).unwrap() }));
    }
}
