mod command;
mod file;

use serde::{Deserialize, Serialize};
use toml;

use command::Command;
use file::File;

pub trait Execute {
    fn execute(&mut self) {}
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum Job {
    Command(Command),
    File(File),
}
impl Execute for Job {
    fn execute(&mut self) {
        match self {
            Job::Command(j) => j.execute(),
            Job::File(j) => j.execute(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Main {
    pub jobs: Vec<Job>,
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
                needs: None,
                argv: Some(vec![String::from("foo")]),
                chdir: None,
                command: String::from("something"),
                creates: None,
                removes: None,
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
