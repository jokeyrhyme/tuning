#![deny(clippy::all)]

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Status;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileState {
    Absent,
    Directory,
    File,
    Hard,
    Link,
    Touch,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub struct File {
    pub name: Option<String>,
    pub needs: Option<Vec<String>>,
    pub path: PathBuf,
    pub src: Option<PathBuf>,
    pub state: FileState,
}

impl File {
    pub fn execute(&self) -> super::Result {
        Ok(Status::Done)
    }
}
