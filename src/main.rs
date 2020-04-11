#![deny(clippy::all)]

mod lib;

use std::{convert::TryFrom, fs, io};

use thiserror::Error as ThisError;

use lib::{
    facts::{self, Facts},
    jobs::{self, Main},
    runner, template,
};

const MAIN_TOML_FILE: &str = "main.toml";

#[derive(Debug, ThisError)]
enum Error {
    #[error("valid config file not found")]
    ConfigNotFound,
    #[error(transparent)]
    Facts {
        #[from]
        source: facts::Error,
    },
    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },
    #[error(transparent)]
    Job {
        #[from]
        source: jobs::Error,
    },
    #[error(transparent)]
    Template {
        #[from]
        source: template::Error,
    },
}

type Result<T> = std::result::Result<T, Error>;

fn main() -> Result<()> {
    let facts = Facts::gather()?;
    let m = read_config(&facts)?;
    runner::run(m.jobs);

    Ok(())
}

fn read_config(facts: &Facts) -> Result<Main> {
    let config_paths = [
        facts
            .config_dir
            .join(env!("CARGO_PKG_NAME"))
            .join(MAIN_TOML_FILE),
        facts
            .home_dir
            .join(".dotfiles")
            .join(env!("CARGO_PKG_NAME"))
            .join(MAIN_TOML_FILE),
    ];
    for config_path in config_paths.iter() {
        println!("reading: {}", &config_path.display());
        let text = match fs::read_to_string(&config_path) {
            Ok(s) => s,
            Err(e) => {
                println!("{:?}", e);
                continue;
            }
        };
        let rendered = match template::render(text, &facts) {
            Ok(s) => s,
            Err(e) => {
                println!("{:?}", e);
                continue;
            }
        };
        match Main::try_from(rendered.as_str()) {
            Ok(m) => {
                return Ok(m);
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }
    Err(Error::ConfigNotFound)
}
