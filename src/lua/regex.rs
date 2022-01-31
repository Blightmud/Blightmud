use crate::model::{Regex as Re, RegexOptions};
use mlua::{Table, UserData, UserDataMethods};
use std::fmt::{Display, Formatter};

fn parse_regex_options(opts: &Option<Table>) -> RegexOptions {
    let mut options = RegexOptions::default();
    if let Some(opts) = &opts {
        options.case_insensitive = opts
            .get("case_insensitive")
            .unwrap_or(options.case_insensitive);
        options.multi_line = opts.get("multi_line").unwrap_or(options.multi_line);
    }
    options
}

pub struct RegexLib;

impl UserData for RegexLib {
    fn add_methods<'lua, T: UserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_function(
            "new",
            |_, (pattern, opts): (String, Option<Table>)| -> mlua::Result<Regex> {
                let options = parse_regex_options(&opts);
                match Re::new(&pattern, Some(options)) {
                    Ok(re) => Ok(Regex { regex: re }),
                    Err(msg) => Err(mlua::Error::RuntimeError(msg.to_string())),
                }
            },
        );
    }
}

#[derive(Clone)]
pub struct Regex {
    pub regex: Re,
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
            |_, this, src: String| -> mlua::Result<mlua::Value> {
                Ok(mlua::Value::Boolean(this.regex.is_match(&src)))
            },
        );
        methods.add_method(
            "match",
            |_, this, src: String| -> mlua::Result<Option<Vec<String>>> {
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
        methods.add_method(
            "match_all",
            |_, this, src: String| -> mlua::Result<Option<Vec<Vec<String>>>> {
                let re = &this.regex;
                let matches = re
                    .captures_iter(&src)
                    .map(|captures| {
                        captures
                            .iter()
                            .map(|c| match c {
                                Some(m) => m.as_str().to_string(),
                                None => String::new(),
                            })
                            .collect::<Vec<String>>()
                    })
                    .collect::<Vec<Vec<String>>>();

                if !matches.is_empty() {
                    Ok(Some(matches))
                } else {
                    Ok(None)
                }
            },
        );
        methods.add_method_mut(
            "replace",
            |_,
             this,
             (src, replace, count): (String, String, Option<usize>)|
             -> mlua::Result<String> {
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
    use mlua::Lua;

    use super::RegexLib;

    fn get_lua() -> Lua {
        let state = Lua::new();
        state.globals().set("regex", RegexLib {}).unwrap();
        state
    }

    #[test]
    fn test_match() {
        let state = get_lua();
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("^test$")
            return re:test("test")
            "#,
                )
                .call::<_, bool>(())
                .unwrap(),
            true
        );
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("^test$")
            return re:test("not a test")
            "#,
                )
                .call::<_, bool>(())
                .unwrap(),
            false
        );
    }

    #[test]
    fn test_group() {
        let state = get_lua();
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("^(\\w+)$")
            return re:match("test")
            "#,
                )
                .call::<_, Option<Vec<String>>>(())
                .unwrap(),
            Some(vec!["test".to_string(), "test".to_string()])
        );
        let result: Option<bool> = state
            .load(
                r#"
            local re = regex.new("^(\\w+)$")
            return re:match("not a test")
            "#,
            )
            .call(())
            .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_match_all() {
        let state = get_lua();
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("(\\w+): (\\d+)")
            return re:match_all("homer: 42, bart: 10, lisa: 8")
            "#,
                )
                .call::<_, Option<Vec<Vec<String>>>>(())
                .unwrap(),
            Some(vec![
                vec![
                    "homer: 42".to_string(),
                    "homer".to_string(),
                    "42".to_string()
                ],
                vec!["bart: 10".to_string(), "bart".to_string(), "10".to_string()],
                vec!["lisa: 8".to_string(), "lisa".to_string(), "8".to_string()],
            ])
        );
    }

    #[test]
    fn test_replace() {
        let state = get_lua();
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("(?P<y>\\d{4})-(?P<m>\\d{2})-(?P<d>\\d{2})")
            return re:replace("2012-03-14, 2013-01-01 and 2014-07-05", "$m/$d/$y")
            "#,
                )
                .call::<_, String>(())
                .unwrap(),
            "03/14/2012, 01/01/2013 and 07/05/2014".to_string()
        );
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("(?P<y>\\d{4})-(?P<m>\\d{2})-(?P<d>\\d{2})")
            return re:replace("2012-03-14, 2013-01-01 and 2014-07-05", "$m/$d/$y", 1)
            "#,
                )
                .call::<_, String>(())
                .unwrap(),
            "03/14/2012, 2013-01-01 and 2014-07-05".to_string()
        );
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("(?P<y>\\d{4})-(?P<m>\\d{2})-(?P<d>\\d{2})")
            return re:replace("2012-03-14, 2013-01-01 and 2014-07-05", "$m/$d/$y", 2)
            "#,
                )
                .call::<_, String>(())
                .unwrap(),
            "03/14/2012, 01/01/2013 and 2014-07-05".to_string()
        );
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("(\\d{4})-(\\d{2})-(\\d{2})")
            return re:replace("2012-03-14, 2013-01-01 and 2014-07-05", "$2/$3/$1")
            "#,
                )
                .call::<_, String>(())
                .unwrap(),
            "03/14/2012, 01/01/2013 and 07/05/2014".to_string()
        );
    }

    #[test]
    fn test_options() {
        let state = get_lua();
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("^test$", {case_insensitive = true})
            return re:test("TEST")
            "#,
                )
                .call::<_, bool>(())
                .unwrap(),
            true
        );
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("^test$", {multi_line = true})
            return re:test("test\ntest")
            "#,
                )
                .call::<_, bool>(())
                .unwrap(),
            true
        );
        assert_eq!(
            state
                .load(
                    r#"
            local re = regex.new("^test$")
            return re:test("test\ntest")
            "#,
                )
                .call::<_, bool>(())
                .unwrap(),
            false
        );
    }
}
