use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Execute;

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
    pub src: Option<PathBuf>,
    pub path: PathBuf,
    pub state: FileState,
}

impl Execute for File {
    fn execute(&mut self) {}
}
