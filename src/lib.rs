#[cfg(feature = "mockall")]
use mockall::automock;

#[cfg_attr(feature = "mockall", automock)]
pub trait Exec {
    /// Runs a command in the provided context
    ///
    /// * `command` - array of strings containing the command and arguments
    /// * `context` - either a local or a remote context
    ///
    fn exec<'a>(
        &mut self,
        command: &str,
        args: &[&'a str],
        context: &Context,
    ) -> Result<String, ExecError>;

    /// Runs several commands piping stdout of one command into stdin of the next
    ///
    /// * `commands` - a vector of tuples of arrays of string containing the command and arguments, and contexts
    ///
    fn exec_piped<'a>(
        &mut self,
        commands: &[(&'a str, &'a [&'a str], &'a Context)],
    ) -> Result<String, ExecError>;
}

#[derive(Debug, PartialEq, Clone)]
pub enum Context {
    /// Local context
    ///
    /// * `user` - name of the user who will execute the command
    ///
    Local { user: String },
    /// Remote context
    ///
    /// * `host` - name of the remote host
    /// * `user` - name of the user on the remote host
    /// * `identity` - path and filename of the ssh identity file
    ///
    Remote {
        host: String,
        user: String,
        identity: String,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("error during execution: {0}")]
    Execution(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("error getting output of preceding command")]
    Chaining,
    #[error("command was terminated by signal")]
    TerminationBySignal,
    #[error("command finished with status code {0}: {1}")]
    TerminationWithError(i32, String),
    #[error("command finished with status code {0}")]
    TerminationWithErrorCode(i32),
}

pub struct CommandExec {}

impl Exec for CommandExec {
    fn exec(
        &mut self,
        command: &str,
        args: &[&str],
        context: &Context,
    ) -> Result<String, ExecError> {
        self.run_piped(&vec![(command, args, context)])
    }

    fn exec_piped(&mut self, commands: &[(&str, &[&str], &Context)]) -> Result<String, ExecError> {
        self.run_piped(commands)
    }
}

impl CommandExec {
    fn run_piped(&mut self, commands: &[(&str, &[&str], &Context)]) -> Result<String, ExecError> {
        let mut child: Option<std::process::Child> = None;

        for (command, args, context) in commands {
            match child {
                Some(mut c) => {
                    child = Some(self.run_single(command, args, context, Some(&mut c))?);
                }
                None => {
                    child = Some(self.run_single(command, args, context, None)?);
                }
            }
        }

        let output = child.ok_or(ExecError::Chaining)?.wait_with_output()?;
        let output = CommandExec::check_output(&output)?;

        Ok(String::from_utf8(output)?)
    }

    fn run_single(
        &mut self,
        command: &str,
        args: &[&str],
        context: &Context,
        pre: Option<&mut std::process::Child>,
    ) -> Result<std::process::Child, ExecError> {
        let mut com = match context {
            Context::Local { user } => {
                let mut com = std::process::Command::new("sudo");
                com.arg("-nu").arg(user).arg("--");
                com
            }
            Context::Remote {
                host,
                user,
                identity,
            } => {
                let mut com = std::process::Command::new("ssh");
                com.arg("-i")
                    .arg(identity)
                    .arg(format!("{}@{}", user, host));
                com
            }
        };

        match pre {
            Some(child) => {
                let stdout = child.stdout.take().ok_or(ExecError::Chaining)?;
                com.stdin(stdout);
            }
            None => {}
        }

        com.stdout(std::process::Stdio::piped())
            .arg(command)
            .args(args)
            .spawn()
            .map_err(|e| ExecError::Io(e))
    }

    fn check_output(output: &std::process::Output) -> Result<Vec<u8>, ExecError> {
        match output.status.code() {
            Some(code) => {
                if code == 0 {
                    Ok(output.stdout.clone())
                } else {
                    match String::from_utf8(output.stderr.clone()) {
                        Ok(s) => Err(ExecError::TerminationWithError(code, s)),
                        Err(_) => Err(ExecError::TerminationWithErrorCode(code)),
                    }
                }
            }
            None => Err(ExecError::TerminationBySignal),
        }
    }
}

#[cfg(all(test, feature = "mockall"))]
mod tests {
    use super::*;

    #[test]
    fn example() {
        let mut mock = MockExec::new();

        mock.expect_exec()
            .once()
            .returning(|_command, _args, _context| Ok("ok".to_string()));

        let res = mock
            .exec(
                "",
                &[""],
                &Context::Local {
                    user: String::from(users::get_current_username().unwrap().to_str().unwrap()),
                },
            )
            .unwrap();

        assert_eq!(res, "ok");
    }

    #[test]
    fn run() {
        let mut com = CommandExec {};

        assert_eq!(
            com.exec(
                "ls",
                &["Cargo.toml"],
                &Context::Local {
                    user: String::from(users::get_current_username().unwrap().to_str().unwrap()),
                }
            )
            .unwrap(),
            "Cargo.toml\n"
        );
    }

    #[test]
    fn run_piped() {
        let mut com = CommandExec {};
        let context = Context::Local {
            user: String::from(users::get_current_username().unwrap().to_str().unwrap()),
        };

        assert_eq!(
            com.run_piped(&[
                ("cat", &["Cargo.toml"], &context),
                ("grep", &["name"], &context),
            ])
            .unwrap(),
            "name = \"exec-rs\"\n"
        );
    }
}
