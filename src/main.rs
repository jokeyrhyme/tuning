use std::{fs, io};

use dirs;

fn main() -> io::Result<()> {
    let config_path = dirs::config_dir()
        .expect("cannot find user's config directory")
        .join(env!("CARGO_PKG_NAME"))
        .join("main.toml");

    println!("reading: {}", &config_path.display());
    println!("{}", fs::read_to_string(&config_path)?);
    Ok(())
}
