#![deny(clippy::all)]

use std::{env::consts::OS, path::PathBuf};

use serde::Serialize;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("unable to find cache_dir")]
    CacheDir,
    #[error("unable to find config_dir")]
    ConfigDir,
    #[error("unable to find home_dir")]
    HomeDir,
}

#[derive(Serialize)]
pub struct Facts {
    pub cache_dir: PathBuf,
    pub config_dir: PathBuf,
    pub home_dir: PathBuf,
    pub is_os_linux: bool,
    pub is_os_macos: bool,
    pub is_os_windows: bool,
}
impl Facts {
    pub fn gather() -> Result {
        Ok(Self {
            cache_dir: dirs::cache_dir().ok_or(Error::CacheDir)?,
            config_dir: dirs::config_dir().ok_or(Error::ConfigDir)?,
            home_dir: dirs::home_dir().ok_or(Error::HomeDir)?,
            is_os_linux: OS == "linux",
            is_os_macos: OS == "macos",
            is_os_windows: OS == "windows",
        })
    }
}
impl Default for Facts {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::new(),
            config_dir: PathBuf::new(),
            home_dir: PathBuf::new(),
            is_os_linux: false,
            is_os_macos: false,
            is_os_windows: false,
        }
    }
}

pub type Result = std::result::Result<Facts, Error>;
