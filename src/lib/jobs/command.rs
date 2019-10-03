#![deny(clippy::all)]

use std::{env, io, path::PathBuf, sync::Mutex, thread};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use subprocess::{Exec, Redirection};

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
    pub fn execute(&self) -> super::Result {
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
            .popen()?;
        let (mut stderr, mut stdout) = (p.stderr.take().unwrap(), p.stdout.take().unwrap());
        thread::spawn(move || io::copy(&mut stderr, &mut io::stderr()));
        thread::spawn(move || io::copy(&mut stdout, &mut io::stdout()));
        let status = p.wait()?;
        if status.success() {
            Ok(Status::Done)
        } else {
            Err(super::Error::Other(String::from("non-zero exit status")))
        }
    }
}
