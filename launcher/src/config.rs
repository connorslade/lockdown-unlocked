use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub injection: PathBuf,
    pub lockdown_browser: PathBuf,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let config = fs::read_to_string(path)?;
        let config = toml::from_str(&config)?;
        Ok(config)
    }
}
