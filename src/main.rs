#![deny(clippy::all)]

mod lib;

use std::{fs, io};

use dirs;

use lib::{
    jobs::{self, Main},
    runner,
};

fn main() -> io::Result<()> {
    let config_path = dirs::config_dir()
        .expect("cannot find user's config directory")
        .join(env!("CARGO_PKG_NAME"))
        .join("main.toml");

    println!("reading: {}", &config_path.display());
    let text = fs::read_to_string(&config_path)?;
    let m = Main::from(text);
    runner::run(m.jobs);

    Ok(())
}
