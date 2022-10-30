use std::error::Error;
use std::process::Command;

#[cfg(all(test, feature = "mockall"))]
use mockall::automock;

#[cfg_attr(test, cfg_attr(feature = "mockall", automock))]
pub trait Exec {
    fn exec<'a>(&mut self, command: &str, args: &'a [&'a str]) -> Result<String, Box<dyn Error>>;
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("error during execution: {0}")]
    Execution(String),
}

pub struct CommandExec {}

impl Exec for CommandExec {
    fn exec(&mut self, command: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
        let output = Command::new(command).args(args).output()?;

        match output.status.success() {
            true => Ok(String::from_utf8(output.stdout)?),
            false => Err(Box::new(ExecError::Execution(String::from_utf8(
                output.stderr,
            )?))),
        }
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(all(test, feature = "mockall"))]
mod tests {
    use super::*;

    #[test]
    fn example() {
        let mut mock = MockExec::new();

        mock.expect_exec()
            .once()
            .returning(|_command, _args| Ok("ok".to_string()));

        let res = mock.exec("", &[""]).unwrap();

        assert_eq!(res, "ok");
    }
}
