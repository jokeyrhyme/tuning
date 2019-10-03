#![deny(clippy::all)]

mod command;
mod file;

use std::{fmt, io};

use serde::{Deserialize, Serialize};
use subprocess::PopenError;
use toml;

use command::Command;
use file::File;

#[derive(Clone, Debug)]
pub enum Error {
    Other(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Other(s) => s,
            }
        )
    }
}
impl std::error::Error for Error {}
impl From<PopenError> for Error {
    fn from(src: PopenError) -> Self {
        Error::Other(format!("{:?}", src))
    }
}
impl From<io::Error> for Error {
    fn from(src: io::Error) -> Self {
        Error::Other(format!("{:?}", src))
    }
}

pub trait Execute {
    fn execute(&self) -> Result;
    fn name(&self) -> String;
    fn needs(&self) -> Vec<String>;
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Job {
    Command(Command),
    File(File),
}
impl Execute for Job {
    fn execute(&self) -> Result {
        match self {
            Job::Command(j) => j.execute(),
            Job::File(j) => j.execute(),
        }
    }
    fn name(&self) -> String {
        let name = match self {
            Job::Command(j) => j.name.clone(),
            Job::File(j) => j.name.clone(),
        };
        match name {
            Some(n) => n,
            None => format!("{:?}", self),
        }
    }
    fn needs(&self) -> Vec<String> {
        let needs = match self {
            Job::Command(j) => j.needs.clone(),
            Job::File(j) => j.needs.clone(),
        };
        match needs {
            Some(n) => n,
            None => vec![],
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Main {
    pub jobs: Vec<Job>,
}

pub type Result = std::result::Result<Status, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Blocked, // when "needs" are not yet Done
    InProgress,
    Done,
    Pending, // when no "needs" or "needs" are all Done
}

pub fn from_str<S>(s: S) -> Main
where
    S: AsRef<str>,
{
    // TODO: handle error
    toml::from_str(&s.as_ref()).unwrap()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use file::FileState;

    use super::*;

    #[test]
    fn command_toml() {
        let input = r#"
            [[jobs]]
            name = "run something"
            type = "command"
            command = "something"
            argv = [ "foo" ]
        "#;

        let got: Main = from_str(&input);

        let want = Main {
            jobs: vec![Job::Command(Command {
                name: Some(String::from("run something")),
                argv: Some(vec![String::from("foo")]),
                command: String::from("something"),
                ..Default::default()
            })],
        };

        assert_eq!(got.jobs.len(), 1);
        assert_eq!(got, want);
    }

    #[test]
    fn file_toml() {
        let input = r#"
            [[jobs]]
            name = "mkdir /tmp"
            type = "file"
            path = "/tmp"
            state = "directory"
        "#;

        let got: Main = from_str(&input);

        let want = Main {
            jobs: vec![Job::File(File {
                name: Some(String::from("mkdir /tmp")),
                needs: None,
                src: None,
                path: PathBuf::from("/tmp"),
                state: FileState::Directory,
            })],
        };

        assert_eq!(got.jobs.len(), 1);
        assert_eq!(got, want);
    }
}
