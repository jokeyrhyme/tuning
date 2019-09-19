use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Execute;

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

impl Execute for Command {
    fn execute(&mut self) {}
}
