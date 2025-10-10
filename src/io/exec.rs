use std::process::{Command, Output};

use anyhow::bail;
use anyhow::Result;

pub fn exec(cmd: &str) -> Result<Output> {
    match Command::new("sh").arg("-c").arg(cmd).output() {
        Ok(output) => Ok(output),
        Err(err) => bail!(err),
    }
}

pub fn exec_args(cmd: &[String]) -> Result<Output> {
    match cmd {
        [exe, args @ ..] => match Command::new(exe).args(args).output() {
            Ok(output) => Ok(output),
            Err(err) => bail!(err),
        },
        _ => bail!("argument table must contain executable as first element"),
    }
}

#[cfg(test)]
mod test_exec {

    use super::*;

    #[test]
    fn test() {
        assert_eq!(b"test\n".to_vec(), exec("echo 'test'").unwrap().stdout);
        assert_eq!(
            b"test\n".to_vec(),
            exec_args(&["echo".to_string(), "test".to_string()])
                .unwrap()
                .stdout
        );
    }
}
