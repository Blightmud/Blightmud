use anyhow::{anyhow, Result};
use hunspell_rs::{CheckResult, Hunspell};
use mlua::prelude::LuaError;
use mlua::{AnyUserData, Result as LuaResult, String as LuaString, Table, UserData};
use std::rc::Rc;

pub const LUA_GLOBAL_NAME: &str = "spellcheck";

pub struct Spellchecker {
    hunspell: Option<HunspellSafe>,
}

impl Spellchecker {
    pub fn new() -> Self {
        Spellchecker { hunspell: None }
    }

    pub fn init(&mut self, aff_path: &str, dict_path: &str) {
        self.hunspell
            .replace(HunspellSafe::from(Hunspell::new(aff_path, dict_path)));
    }

    fn check_initialized(&self) -> Result<()> {
        match self.hunspell.is_none() {
            true => Err(anyhow!("spellchecker not initialized")),
            false => Ok(()),
        }
    }

    pub fn check(&self, word: &str) -> Result<bool> {
        self.check_initialized()?;
        match self.hunspell.as_ref().unwrap().check(word) {
            CheckResult::MissingInDictionary => Ok(false),
            _ => Ok(true),
        }
    }

    pub fn suggest(&self, word: &str) -> Result<Vec<String>> {
        self.check_initialized()?;
        Ok(self.hunspell.as_ref().unwrap().suggest(word))
    }
}

impl UserData for Spellchecker {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function(
            "init",
            |ctx, (aff_path, dict_path): (LuaString, LuaString)| -> LuaResult<()> {
                let this_aux = ctx.globals().get::<_, AnyUserData>(LUA_GLOBAL_NAME)?;
                let mut this = this_aux
                    .borrow_mut::<Spellchecker>()
                    .map_err(LuaError::external)?;
                this.init(aff_path.to_str()?, dict_path.to_str()?);
                Ok(())
            },
        );
        methods.add_function("check", |ctx, word: LuaString| -> LuaResult<bool> {
            let this_aux = ctx.globals().get::<_, AnyUserData>(LUA_GLOBAL_NAME)?;
            let this = this_aux
                .borrow::<Spellchecker>()
                .map_err(LuaError::external)?;
            let found = this.check(word.to_str()?).map_err(LuaError::external)?;
            Ok(found)
        });
        methods.add_function("suggest", |ctx, word: LuaString| -> LuaResult<Table> {
            let this_aux = ctx.globals().get::<_, AnyUserData>(LUA_GLOBAL_NAME)?;
            let this = this_aux
                .borrow::<Spellchecker>()
                .map_err(LuaError::external)?;
            let res_table = ctx.create_table()?;
            this.suggest(word.to_str()?)
                .map_err(LuaError::external)?
                .iter()
                .enumerate()
                .for_each(|(i, v)| res_table.set(i, v.as_str()).unwrap());
            Ok(res_table)
        });
    }
}

#[derive(Clone)]
struct HunspellSafe(Rc<Hunspell>);

unsafe impl Send for HunspellSafe {}

impl std::ops::Deref for HunspellSafe {
    type Target = Hunspell;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Hunspell> for HunspellSafe {
    fn from(hunspell: Hunspell) -> Self {
        Self(Rc::new(hunspell))
    }
}

#[cfg(test)]
mod tests {
    use crate::lua::spellcheck::{Spellchecker, LUA_GLOBAL_NAME};
    use mlua::{Lua, Table};

    const AFF_PATH: &str = "tests/spellcheck/tiny.aff";
    const DICT_PATH: &str = "tests/spellcheck/tiny.dic";

    #[test]
    fn test_check_initialized() {
        let mut spellchecker = Spellchecker::new();
        assert_eq!(spellchecker.check_initialized().is_err(), true);
        spellchecker.init(AFF_PATH, DICT_PATH);
        assert_eq!(spellchecker.check_initialized().is_ok(), true);
    }

    #[test]
    fn test_check() {
        let mut spellchecker = Spellchecker::new();
        assert_eq!(spellchecker.check("not-initialized").is_err(), true);
        spellchecker.init(AFF_PATH, DICT_PATH);
        assert_eq!(spellchecker.check("cromulent").unwrap(), false);
        assert_eq!(spellchecker.check("cats").unwrap(), true);
    }

    #[test]
    fn test_suggest() {
        let mut spellchecker = Spellchecker::new();
        assert_eq!(spellchecker.suggest("not-initialized").is_err(), true);
        spellchecker.init(AFF_PATH, DICT_PATH);
        let results = spellchecker.suggest("progra");
        assert_eq!(results.unwrap(), vec!["program"])
    }

    #[test]
    fn test_lua_api() {
        let lua = Lua::new();
        lua.globals()
            .set(LUA_GLOBAL_NAME, Spellchecker::new())
            .unwrap();

        // Trying to use check before init should err.
        let check_script = r#"check_res = spellcheck.check("cat")"#;
        let no_init_check = lua.load(check_script).exec();
        assert_eq!(no_init_check.is_err(), true);

        // Trying to use suggest before init should err.
        let suggest_script = r#"suggest_res = spellcheck.suggest("progra")"#;
        let no_init_suggest = lua.load(suggest_script).exec();
        assert_eq!(no_init_suggest.is_err(), true);

        // We should be able to init without err.
        let init_script = format!("spellcheck.init({:?}, {:?})", AFF_PATH, DICT_PATH);
        lua.load(init_script.as_str()).exec().unwrap();

        // After init we should be able to check/suggest.
        lua.load(check_script).exec().unwrap();
        let check_res: bool = lua.globals().get("check_res").unwrap();
        assert_eq!(check_res, true);

        lua.load(suggest_script).exec().unwrap();
        let suggest_res: Table = lua.globals().get("suggest_res").unwrap();
        assert_eq!(suggest_res.get::<i32, String>(0).unwrap(), "program");
    }
}
