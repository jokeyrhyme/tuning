#![deny(clippy::all)]

mod command;
mod file;

use std::{convert::TryFrom, fmt};

use colored::*;
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
pub struct Job {
    #[serde(flatten)]
    metadata: Metadata,

    #[serde(flatten)]
    spec: Spec,
}
impl Execute for Job {
    fn execute(&self) -> Result {
        match &self.spec {
            Spec::Command(j) => j.execute().map_err(|e| Error::CommandJob { source: e }),
            Spec::File(j) => j.execute().map_err(|e| Error::FileJob { source: e }),
        }
    }
    fn name(&self) -> String {
        match &self.spec {
            Spec::Command(j) => self.metadata.name.clone().unwrap_or_else(|| j.name()),
            Spec::File(j) => self.metadata.name.clone().unwrap_or_else(|| j.name()),
        }
    }
    fn needs(&self) -> Vec<String> {
        self.metadata.needs.clone().unwrap_or_else(|| vec![])
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Metadata {
    name: Option<String>,
    needs: Option<Vec<String>>,
}
impl Default for Metadata {
    fn default() -> Self {
        Self {
            name: None,
            needs: None,
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Spec {
    Command(Command),
    File(File),
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
pub fn result_display(result: &Result) -> String {
    match result {
        Ok(s) => format!("{}", s),
        Err(e) => format!("{:#?}", e).red().to_string(),
    }
}
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
    Pending,          // when no "needs"; or "needs" are all Done
}
impl fmt::Display for Status {
    // TODO: should Display include terminal output concerns?
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Blocked => write!(f, "{}", "blocked".red().dimmed()),
            Self::Changed(from, to) => write!(
                f,
                "{}: {} => {}",
                "changed".yellow(),
                from.yellow().dimmed(),
                to.yellow()
            ),
            Self::Done => write!(f, "{}", "done".blue()),
            Self::InProgress => write!(f, "{}", "inprogress".cyan()),
            Self::NoChange(s) => write!(f, "{}: {}", "nochange".green(), s.green()),
            Self::Pending => write!(f, "{}", "pending".white()),
        }
    }
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
            jobs: vec![Job {
                metadata: Metadata {
                    name: Some(String::from("run something")),
                    ..Default::default()
                },
                spec: Spec::Command(Command {
                    argv: Some(vec![String::from("foo")]),
                    command: String::from("something"),
                    ..Default::default()
                }),
            }],
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
            jobs: vec![Job {
                metadata: Metadata {
                    name: Some(String::from("mkdir /tmp")),
                    ..Default::default()
                },
                spec: Spec::File(File {
                    force: None,
                    src: None,
                    path: PathBuf::from("/tmp"),
                    state: FileState::Directory,
                }),
            }],
        };

        assert_eq!(got.jobs.len(), 1);
        assert_eq!(got, want);

        Ok(())
    }
}
