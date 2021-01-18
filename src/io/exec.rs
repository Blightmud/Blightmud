use std::process::{Command, Output};

use anyhow::Result;
use simple_error::bail;

pub fn exec(cmd: &str) -> Result<Output> {
    match Command::new("sh").arg("-c").arg(cmd).output() {
        Ok(output) => Ok(output),
        Err(err) => bail!(err),
    }
}

#[cfg(test)]
mod test_exec {

    use super::*;

    #[test]
    fn test() {
        assert_eq!(b"test\n".to_vec(), exec("echo 'test'").unwrap().stdout);
    }
}
