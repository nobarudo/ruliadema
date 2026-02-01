use std::time::Duration;
use anyhow::Result;
use tokio::time;

use ruliadema::{Config, checker::HttpChecker};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_file("config.toml")?;
    let checker = HttpChecker::new(&config)?;

    let mut interval = time::interval(Duration::from_secs(config.interval_seconds));

    loop {
        interval.tick().await;
        checker.run_once(&config).await;
    }
}
