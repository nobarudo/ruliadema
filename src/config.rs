use serde::Deserialize;
use std::fs;
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
    pub max_concurrency: usize,
    pub targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    pub url: String,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;

        if path.ends_with(".toml") {
            Ok(toml::from_str(&content)?)
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            Ok(serde_yaml::from_str(&content)?)
        } else {
            anyhow::bail!("unsupported config format");
        }
    }
}
