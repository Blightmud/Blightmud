use std::process::Output;

use mlua::{UserData, UserDataMethods};

pub struct ExecResponse {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl UserData for ExecResponse {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "code",
            |_, this, _: ()| -> Result<Option<i32>, mlua::Error> { Ok(this.code) },
        );
        methods.add_method("stdout", |_, this, _: ()| -> Result<String, mlua::Error> {
            Ok(this.stdout.clone())
        });
        methods.add_method("stderr", |_, this, _: ()| -> Result<String, mlua::Error> {
            Ok(this.stderr.clone())
        });
    }
}

fn bytes_to_string(bytes: &[u8]) -> String {
    if let Ok(output) = String::from_utf8(bytes.to_vec()) {
        output
    } else {
        String::from_utf8_lossy(bytes).to_mut().clone()
    }
}

impl From<Output> for ExecResponse {
    fn from(out: Output) -> Self {
        let code = out.status.code();
        let stdout = bytes_to_string(&out.stdout);
        let stderr = bytes_to_string(&out.stderr);

        Self {
            code,
            stdout,
            stderr,
        }
    }
}
