use regex::Regex;
use rlua::{MetaMethod, UserData, UserDataMethods};

#[derive(Clone)]
pub struct Alias {
    pub regex: Regex,
    pub enabled: bool,
}

impl Alias {
    pub fn create(regex: &str) -> Result<Self, String> {
        match Regex::new(regex) {
            Ok(regex) => Ok(Self {
                regex,
                enabled: true,
            }),
            Err(msg) => Err(msg.to_string()),
        }
    }
}

impl UserData for Alias {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |ctx, this, key: String| {
            match key.as_str() {
                "regex" => Ok(rlua::Value::String(ctx.create_string(this.regex.as_str())?)),
                "enabled" => Ok(rlua::Value::Boolean(this.enabled)),
                _ => Err(rlua::Error::RuntimeError(format!(
                    "No value {} exists on Alias object",
                    key
                ))),
            }
        });
    }
}
