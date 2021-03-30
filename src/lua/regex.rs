use regex::Regex as Re;
use rlua::{UserData, UserDataMethods};
use std::fmt::{Display, Formatter};

pub struct RegexLib;

impl UserData for RegexLib {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function("new", |_, pattern: String| -> rlua::Result<Regex> {
            match Re::new(&pattern) {
                Ok(re) => Ok(Regex { regex: re }),
                Err(msg) => Err(rlua::Error::RuntimeError(msg.to_string())),
            }
        });
    }
}

#[derive(Clone)]
pub struct Regex {
    regex: regex::Regex,
}

impl Display for Regex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.regex.fmt(f)
    }
}

impl UserData for Regex {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method(
            "test",
            |_, this, src: String| -> rlua::Result<rlua::Value> {
                Ok(rlua::Value::Boolean(this.regex.is_match(&src)))
            },
        );
        methods.add_method(
            "match",
            |_, this, src: String| -> rlua::Result<Option<Vec<String>>> {
                let re = &this.regex;
                let matches = re.captures(&src).map(|captures| {
                    captures
                        .iter()
                        .map(|c| match c {
                            Some(m) => m.as_str().to_string(),
                            None => String::new(),
                        })
                        .collect()
                });
                Ok(matches)
            },
        );
        methods.add_method_mut(
            "replace",
            |_,
             this,
             (src, replace, count): (String, String, Option<usize>)|
             -> rlua::Result<String> {
                let re = &this.regex;
                let limit = count.unwrap_or(0);
                Ok(re.replacen::<&str>(&src, limit, &replace).to_mut().clone())
            },
        );
        methods.add_method("regex", |_, this, ()| Ok(this.to_string()));
    }
}

#[cfg(test)]
mod test_regexp {
    use rlua::Lua;

    use super::RegexLib;

    fn get_lua() -> Lua {
        let state = Lua::new();
        state.context(|ctx| {
            ctx.globals().set("regex", RegexLib {}).unwrap();
        });
        state
    }

    #[test]
    fn test_match() {
        let state = get_lua();
        assert_eq!(
            state.context(|ctx| -> bool {
                ctx.load(
                    r#"
            local re = regex.new("^test$")
            return re:test("test")
            "#,
                )
                .call(())
                .unwrap()
            }),
            true
        );
        assert_eq!(
            state.context(|ctx| -> bool {
                ctx.load(
                    r#"
            local re = regex.new("^test$")
            return re:test("not a test")
            "#,
                )
                .call(())
                .unwrap()
            }),
            false
        );
    }

    #[test]
    fn test_group() {
        let state = get_lua();
        assert_eq!(
            state
                .context(|ctx| -> rlua::Result<Option<Vec<String>>> {
                    ctx.load(
                        r#"
            local re = regex.new("^(\\w+)$")
            return re:match("test")
            "#,
                    )
                    .call(())
                })
                .unwrap(),
            Some(vec!["test".to_string(), "test".to_string()])
        );
        let result = state
            .context(|ctx| -> rlua::Result<Option<Vec<String>>> {
                ctx.load(
                    r#"
            local re = regex.new("^(\\w+)$")
            return re:match("not a test")
            "#,
                )
                .call(())
            })
            .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_replace() {
        let state = get_lua();
        assert_eq!(
            state
                .context(|ctx| -> rlua::Result<String> {
                    ctx.load(
                        r#"
            local re = regex.new("(?P<y>\\d{4})-(?P<m>\\d{2})-(?P<d>\\d{2})")
            return re:replace("2012-03-14, 2013-01-01 and 2014-07-05", "$m/$d/$y")
            "#,
                    )
                    .call(())
                })
                .unwrap(),
            "03/14/2012, 01/01/2013 and 07/05/2014".to_string()
        );
        assert_eq!(
            state
                .context(|ctx| -> rlua::Result<String> {
                    ctx.load(
                        r#"
            local re = regex.new("(?P<y>\\d{4})-(?P<m>\\d{2})-(?P<d>\\d{2})")
            return re:replace("2012-03-14, 2013-01-01 and 2014-07-05", "$m/$d/$y", 1)
            "#,
                    )
                    .call(())
                })
                .unwrap(),
            "03/14/2012, 2013-01-01 and 2014-07-05".to_string()
        );
        assert_eq!(
            state
                .context(|ctx| -> rlua::Result<String> {
                    ctx.load(
                        r#"
            local re = regex.new("(?P<y>\\d{4})-(?P<m>\\d{2})-(?P<d>\\d{2})")
            return re:replace("2012-03-14, 2013-01-01 and 2014-07-05", "$m/$d/$y", 2)
            "#,
                    )
                    .call(())
                })
                .unwrap(),
            "03/14/2012, 01/01/2013 and 2014-07-05".to_string()
        );
        assert_eq!(
            state
                .context(|ctx| -> rlua::Result<String> {
                    ctx.load(
                        r#"
            local re = regex.new("(\\d{4})-(\\d{2})-(\\d{2})")
            return re:replace("2012-03-14, 2013-01-01 and 2014-07-05", "$2/$3/$1")
            "#,
                    )
                    .call(())
                })
                .unwrap(),
            "03/14/2012, 01/01/2013 and 07/05/2014".to_string()
        );
    }
}
