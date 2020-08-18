use regex::Regex;
use rlua::{MetaMethod, UserData, UserDataMethods};

#[derive(Clone)]
pub struct Trigger {
    pub regex: Regex,
    pub gag: bool,
    pub raw: bool,
    pub enabled: bool,
    pub prompt: bool,
}

impl Trigger {
    pub fn create(regex: &str) -> Result<Self, String> {
        match Regex::new(regex) {
            Ok(regex) => Ok(Self {
                regex,
                gag: false,
                raw: false,
                enabled: true,
                prompt: false,
            }),
            Err(msg) => Err(msg.to_string()),
        }
    }
}

impl UserData for Trigger {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |ctx, this, key: String| {
            match key.as_str() {
                "regex" => Ok(rlua::Value::String(ctx.create_string(this.regex.as_str())?)),
                "gag" => Ok(rlua::Value::Boolean(this.gag)),
                "raw" => Ok(rlua::Value::Boolean(this.gag)),
                "enabled" => Ok(rlua::Value::Boolean(this.enabled)),
                "prompt" => Ok(rlua::Value::Boolean(this.prompt)),
                _ => Err(rlua::Error::RuntimeError(format!(
                    "No value {} exists on Trigger object",
                    key
                ))),
            }
        });
    }
}
