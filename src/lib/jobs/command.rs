#![deny(clippy::all)]

use std::{env, io, path::PathBuf, sync::Mutex, thread};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use subprocess::{Exec, PopenError, Redirection};
use thiserror::Error as ThisError;

use super::Status;

lazy_static! {
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub struct Command {
    pub name: Option<String>,
    pub needs: Option<Vec<String>>,
    pub argv: Option<Vec<String>>,
    pub chdir: Option<PathBuf>,
    pub command: String,
    pub creates: Option<PathBuf>,
    pub removes: Option<PathBuf>,
}
impl Default for Command {
    fn default() -> Self {
        Command {
            name: None,
            needs: None,
            argv: None,
            chdir: None,
            command: String::new(),
            creates: None,
            removes: None,
        }
    }
}
impl Command {
    pub fn execute(&self) -> Result {
        match &self.creates {
            Some(p) => {
                if p.exists() {
                    return Ok(Status::NoChange(format!("{:?} already created", p)));
                }
            }
            None => {}
        }
        match &self.removes {
            Some(p) => {
                if !p.exists() {
                    return Ok(Status::NoChange(format!("{:?} already removed", p)));
                }
            }
            None => {}
        }

        // we want exactly one "command" to use stdout at a time,
        // at least until we decide how sharing stdout should work
        let _ = MUTEX.lock().unwrap();

        let args = match &self.argv {
            Some(a) => a.clone(),
            None => Vec::<String>::new(),
        };
        let cwd = match &self.chdir {
            Some(c) => c.clone(),
            None => env::current_dir().unwrap(),
        };
        let mut p = Exec::cmd(&self.command)
            .args(&args)
            .cwd(&cwd)
            .stdout(Redirection::Pipe)
            .stderr(Redirection::Pipe)
            .popen()
            .map_err(|e| Error::CommandBegin {
                cmd: self.command.clone(),
                source: e,
            })?;
        let (mut stderr, mut stdout) = (p.stderr.take().unwrap(), p.stdout.take().unwrap());
        thread::spawn(move || io::copy(&mut stderr, &mut io::stderr()));
        thread::spawn(move || io::copy(&mut stdout, &mut io::stdout()));
        let status = p.wait().map_err(|e| Error::CommandWait {
            cmd: self.command.clone(),
            source: e,
        })?;
        if status.success() {
            Ok(Status::Done)
        } else {
            Err(Error::NonZeroExitStatus {
                cmd: self.command.clone(),
            })
        }
    }

    pub fn name(&self) -> String {
        let mut parts = Vec::<String>::new();
        if let Some(c) = &self.creates {
            parts.push(format!("[ ! -e {} ] &&", c.display()));
        }
        if let Some(r) = &self.removes {
            parts.push(format!("[ -e {} ] &&", r.display()));
        }
        if let Some(c) = &self.chdir {
            parts.push(format!("cd {} &&", c.display()));
        }
        parts.push(self.command.clone());
        if let Some(a) = &self.argv {
            parts.extend(a.clone());
        }
        parts.join(" ")
    }
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("`{}` could not begin: {}", cmd, source)]
    CommandBegin { cmd: String, source: PopenError },
    #[error("`{}` could not continue: {}", cmd, source)]
    CommandWait { cmd: String, source: PopenError },
    #[error("`{}` exited with non-zero status code", cmd)]
    NonZeroExitStatus { cmd: String },
}

pub type Result = std::result::Result<Status, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn done_after_running_command() {
        let cmd = Command {
            argv: Some(vec![String::from("--version")]),
            command: String::from("cargo"),
            ..Default::default()
        };
        match cmd.execute() {
            Ok(s) => assert_eq!(s, Status::Done),
            Err(_) => unreachable!(), // fail
        }
        // TODO: should also test stdout/stderr
    }

    #[test]
    fn error_after_running_failed_command() {
        let cmd = Command {
            argv: Some(vec![String::from("--flag-does-not-exist")]),
            command: String::from("cargo"),
            ..Default::default()
        };
        if cmd.execute().is_ok() {
            unreachable!(); // fail
        }
    }

    #[test]
    fn skips_when_creates_file_already_exists() {
        let cmd = Command {
            command: String::from("./throw_if_attempt_to_execute"),
            creates: Some(PathBuf::from("Cargo.toml")),
            ..Default::default()
        };
        match cmd.execute() {
            Ok(s) => assert_eq!(
                s,
                Status::NoChange(String::from(r#""Cargo.toml" already created"#))
            ),
            Err(_) => unreachable!(), // fail
        }
    }

    #[test]
    fn skips_when_removes_file_already_gone() {
        let cmd = Command {
            command: String::from("./throw_if_attempt_to_execute"),
            removes: Some(PathBuf::from("does_not_exist.toml")),
            ..Default::default()
        };
        match cmd.execute() {
            Ok(s) => assert_eq!(
                s,
                Status::NoChange(String::from(r#""does_not_exist.toml" already removed"#))
            ),
            Err(_) => unreachable!(), // fail
        }
    }

    #[test]
    fn name_with_command() {
        let cmd = Command {
            command: String::from("foo"),
            ..Default::default()
        };
        let got = cmd.name();
        let want = "foo";
        assert_eq!(got, want);
    }

    #[test]
    fn name_with_command_and_argv() {
        let cmd = Command {
            argv: Some(vec![String::from("--bar"), String::from("baz")]),
            command: String::from("foo"),
            ..Default::default()
        };
        let got = cmd.name();
        let want = "foo --bar baz";
        assert_eq!(got, want);
    }

    #[test]
    fn name_with_command_and_chdir() {
        let cmd = Command {
            chdir: Some(PathBuf::from("bar")),
            command: String::from("foo"),
            ..Default::default()
        };
        let got = cmd.name();
        let want = "cd bar && foo";
        assert_eq!(got, want);
    }

    #[test]
    fn name_with_command_and_creates() {
        let cmd = Command {
            command: String::from("foo"),
            creates: Some(PathBuf::from("bar")),
            ..Default::default()
        };
        let got = cmd.name();
        let want = "[ ! -e bar ] && foo";
        assert_eq!(got, want);
    }

    #[test]
    fn name_with_command_and_removes() {
        let cmd = Command {
            command: String::from("foo"),
            removes: Some(PathBuf::from("bar")),
            ..Default::default()
        };
        let got = cmd.name();
        let want = "[ -e bar ] && foo";
        assert_eq!(got, want);
    }
}
