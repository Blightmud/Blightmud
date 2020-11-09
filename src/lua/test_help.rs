macro_rules! test_lua {
    () => {
        let state = get_lua();

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
    };
}
