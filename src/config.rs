use anyhow::Result;
use serde::Deserialize;
use std::fs;

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
    #[serde(default = "default_latency")] // ← 設定がない場合はデフォルト値を使う
    pub acceptable_latency_ms: u64,
}

// デフォルトの許容時間は1000ms（1秒）とする
fn default_latency() -> u64 {
    1000
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
