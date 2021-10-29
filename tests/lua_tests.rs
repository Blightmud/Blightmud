use blightmud::RuntimeConfig;

mod common;

fn test_script(script: &str) {
    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.integration_test = true;
    rt.script = Some(script.to_string());
    let handle = common::start_blightmud(rt);
    common::join_blightmud(handle)
}

#[test]
fn test_server() {
    test_script("tests/test_server.lua");
}
