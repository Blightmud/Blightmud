/// The `test_lua!()` macro should be invoked at the start of a
/// #[test] function to setup the Lua state and create the following
/// Lua-specific assertions macros:
///
/// * `assert_lua_bool!("true", true);`
/// * `assert_lua_string!("'Yes'", "Yes");`
/// * `assert_lua!(usize, "123", 123);`
///
/// It also adds two macros for setting and getting globals:
///
/// * `let var: TYPE = global!("key");`
/// * `set_global!("key", "value");`
///
/// `test_lua!()` can optionally be called with "key" => value pairs
/// as a convenience to set globals right away.
macro_rules! test_lua {
    // test_lua!(key => value);
    ($($key:literal => $val:expr),+) => {
        test_lua!();
        $( set_global!($key, $val); )+
    };

    // Allow trailing comma.
    ($($key:literal => $val:expr,)+) => { test_lua!($($key => $val),+) };

    () => {
        let state = rlua::Lua::new();

        #[allow(unused_macros)]
        macro_rules! run_lua {
            ($lua_code:literal) => {
                state.context(|ctx| -> rlua::Result<()> {
                    ctx.load($lua_code).call::<_,()>(()).unwrap();
                    Ok(())
                }).unwrap();
            };
        }

        #[allow(unused_macros)]
        macro_rules! assert_lua {
            ($return_type:ty, $lua_code:literal, $expect:expr) => {
                assert_eq!(
                    state.context(|ctx| -> $return_type {
                        ctx.load(concat!("return ", $lua_code)).call(()).unwrap()
                    }),
                    $expect
                );
            };
        }

        #[allow(unused_macros)]
        macro_rules! assert_lua_bool {
            ($lua_code:literal, $expect:expr) => {
                assert_lua!(bool, $lua_code, $expect)
            };
        }

        #[allow(unused_macros)]
        macro_rules! assert_lua_string {
            ($lua_code:literal, $expect:expr) => {
                assert_lua!(String, $lua_code, $expect)
            };
        }

        #[allow(unused_macros)]
        macro_rules! global {
            ($key:literal) => {
                state.context(|ctx| ctx.globals().get($key).unwrap())
            };
        }

        #[allow(unused_macros)]
        macro_rules! set_global {
            ($key:literal, $val:expr) => {
                state.context(|ctx| {
                    ctx.globals().set($key, $val).unwrap();
                });
            };
        }
    };
}
