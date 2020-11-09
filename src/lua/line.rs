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
    use super::Line;
    use crate::model::Line as mLine;

    fn test_line() -> Line {
        Line::from(mLine::from("\x1b[31mA testing line\x1b[0m"))
    }

    #[test]
    fn test_content() {
        test_lua!("test_line" => test_line());

        assert_lua_string!("test_line:line()", "A testing line");
        assert_lua_string!("test_line:raw()", "\x1b[31mA testing line\x1b[0m");
    }

    #[test]
    fn test_gag() {
        test_lua!("test_line" => test_line());

        assert_lua_bool!("test_line:gag()", false);
        let line: Line = global!("test_line");
        assert!(!line.inner.flags.gag);

        assert_lua_bool!("test_line:gag(true)", true);
        let line: Line = global!("test_line");
        assert!(line.inner.flags.gag);
    }

    #[test]
    fn test_tts_gag() {
        test_lua!("test_line" => test_line());

        assert_lua_bool!("test_line:tts_gag()", false);

        let line: Line = global!("test_line");
        assert!(!line.inner.flags.tts_gag);
        assert_lua_bool!("test_line:tts_gag(true)", true);

        let line: Line = global!("test_line");
        assert!(line.inner.flags.tts_gag);
    }

    #[test]
    fn test_tts_interrupt() {
        test_lua!("test_line" => test_line());

        assert_lua_bool!("test_line:tts_interrupt()", false);

        let line: Line = global!("test_line");
        assert!(!line.inner.flags.tts_interrupt);
        assert_lua_bool!("test_line:tts_interrupt(true)", true);

        let line: Line = global!("test_line");
        assert!(line.inner.flags.tts_interrupt);
    }

    #[test]
    fn test_skip_log() {
        test_lua!("test_line" => test_line());

        assert_lua_bool!("test_line:skip_log()", false);
        let line: Line = global!("test_line");
        assert!(!line.inner.flags.skip_log);

        assert_lua_bool!("test_line:skip_log(true)", true);
        let line: Line = global!("test_line");
        assert!(line.inner.flags.skip_log);
    }

    #[test]
    fn test_matched() {
        test_lua!("test_line" => test_line());

        assert_lua_bool!("test_line:matched()", false);
        let line: Line = global!("test_line");
        assert!(!line.inner.flags.matched);

        assert_lua_bool!("test_line:matched(true)", true);
        let line: Line = global!("test_line");
        assert!(line.inner.flags.matched);
    }

    #[test]
    fn test_prompt() {
        test_lua!("test_line" => test_line());
        assert_lua_bool!("test_line:prompt()", false);
    }
}
