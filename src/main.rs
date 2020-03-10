#![deny(clippy::all)]

mod lib;

use std::{convert::TryFrom, fs, io};

use dirs;
use thiserror::Error as ThisError;

use lib::{
    facts::Facts,
    jobs::{self, Main},
    runner, template,
};

#[derive(Debug, ThisError)]
enum Error {
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

type Result = std::result::Result<(), Error>;

fn main() -> Result {
    let config_path = dirs::config_dir()
        .expect("cannot find user's config directory")
        .join(env!("CARGO_PKG_NAME"))
        .join("main.toml");

    println!("reading: {}", &config_path.display());
    let text = fs::read_to_string(&config_path)?;

    let facts = Facts::default();
    let rendered = template::render(text, &facts)?;

    let m = Main::try_from(rendered.as_str())?;
    runner::run(m.jobs);

    Ok(())
}
