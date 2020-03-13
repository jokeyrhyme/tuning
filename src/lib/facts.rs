#![deny(clippy::all)]

use std::path::PathBuf;

use dirs;
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
}
impl Facts {
    pub fn gather() -> Result {
        Ok(Self {
            cache_dir: dirs::cache_dir().ok_or(Error::CacheDir)?,
            config_dir: dirs::config_dir().ok_or(Error::ConfigDir)?,
            home_dir: dirs::home_dir().ok_or(Error::HomeDir)?,
        })
    }
}
impl Default for Facts {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::new(),
            config_dir: PathBuf::new(),
            home_dir: PathBuf::new(),
        }
    }
}

pub type Result = std::result::Result<Facts, Error>;
