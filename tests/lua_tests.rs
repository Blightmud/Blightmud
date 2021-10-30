use anyhow::Result;
use blightmud::RuntimeConfig;

mod common;

fn test_script(script: &str) {
    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.integration_test = true;
    rt.script = Some(script.to_string());
    let handle = common::start_blightmud(rt);
    common::join_blightmud(handle);
}

#[test]
fn test_server() {
    test_script("tests/test_server.lua");
}

#[test]
fn test_mud() -> Result<()> {
    let mut server = common::Server::bind(0);
    let mut rt = RuntimeConfig::default();
    rt.headless_mode = true;
    rt.integration_test = true;
    rt.script = Some("tests/test_mud.lua".to_string());
    rt.connect = Some(server.local_addr.to_string());
    let handle = common::start_blightmud(rt);

    let mut connection = server.listen()?;

    loop {
        let data = connection.recv_string();
        if !data.is_empty() {
            println!("[ECHO]: {}", data);
            connection.send(format!("{}", data).as_bytes());
        } else {
            break;
        }
    }

    common::join_blightmud(handle);
    Ok(())
}
