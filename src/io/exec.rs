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

    #[test]
    fn test_exec_with_exit_code() {
        let output = exec("exit 0").unwrap();
        assert!(output.status.success());
    }

    #[test]
    fn test_exec_with_stderr() {
        let output = exec("echo error >&2").unwrap();
        assert_eq!(b"error\n".to_vec(), output.stderr);
    }

    #[test]
    fn test_exec_args_empty() {
        let result = exec_args(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_exec_args_single() {
        let output = exec_args(&["true".to_string()]).unwrap();
        assert!(output.status.success());
    }

    #[test]
    fn test_exec_args_multiple() {
        let output =
            exec_args(&["echo".to_string(), "hello".to_string(), "world".to_string()]).unwrap();
        assert_eq!(b"hello world\n".to_vec(), output.stdout);
    }
}
