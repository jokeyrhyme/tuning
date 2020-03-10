#![deny(clippy::all)]

mod command;
mod file;

use std::convert::TryFrom;

use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
use toml;

use command::Command;
use file::File;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    CommandJob {
        #[from]
        source: command::Error,
    },
    #[error(transparent)]
    FileJob {
        #[from]
        source: file::Error,
    },
    #[error(transparent)]
    ParseToml {
        #[from]
        source: toml::de::Error,
    },
    #[allow(dead_code)] // TODO: fake test-only errors should not be here
    #[error("fake test-only error")]
    SomethingBad,
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
            Job::Command(j) => j.execute().map_err(|e| Error::CommandJob { source: e }),
            Job::File(j) => j.execute().map_err(|e| Error::FileJob { source: e }),
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
impl TryFrom<&str> for Main {
    type Error = Error;
    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        toml::from_str(s).map_err(|e| Error::ParseToml { source: e })
    }
}

pub type Result = std::result::Result<Status, Error>;
pub fn is_result_settled(result: &Result) -> bool {
    match result {
        Ok(s) => match s {
            Status::Blocked => true,
            _ => s.is_done(),
        },
        Err(_) => true,
    }
}
pub fn is_result_done(result: &Result) -> bool {
    match result {
        Ok(s) => s.is_done(),
        Err(_) => false,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Blocked,                 // when "needs" are not yet Done
    Changed(String, String), // more specific kind of Done
    Done,
    InProgress,
    NoChange(String), // more specific kind of Done
    Pending,          // when no "needs" or "needs" are all Done
}
impl Status {
    pub fn is_done(&self) -> bool {
        match &self {
            Self::Changed(_, _) | Self::Done | Self::NoChange(_) => true,
            Self::Blocked | Self::InProgress | Self::Pending => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use file::FileState;

    use super::*;

    #[test]
    fn command_toml() -> std::result::Result<(), Error> {
        let input = r#"
            [[jobs]]
            name = "run something"
            type = "command"
            command = "something"
            argv = [ "foo" ]
            "#;

        let got = Main::try_from(input)?;

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

        Ok(())
    }

    #[test]
    fn file_toml() -> std::result::Result<(), Error> {
        let input = r#"
            [[jobs]]
            name = "mkdir /tmp"
            type = "file"
            path = "/tmp"
            state = "directory"
            "#;

        let got = Main::try_from(input)?;

        let want = Main {
            jobs: vec![Job::File(File {
                name: Some(String::from("mkdir /tmp")),
                needs: None,
                force: None,
                src: None,
                path: PathBuf::from("/tmp"),
                state: FileState::Directory,
            })],
        };

        assert_eq!(got.jobs.len(), 1);
        assert_eq!(got, want);

        Ok(())
    }
}
