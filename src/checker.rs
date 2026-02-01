use std::time::{Duration, Instant};

use crate::Config;

pub struct HttpChecker {
    client: reqwest::Client,
}

impl HttpChecker {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self { client })
    }

    async fn check(&self, url: &str) {
        let start = Instant::now();

        let result = self.client.get(url).send().await;

        let latency = start.elapsed();

        match result {
            Ok(resp) => {
                println!(
                    "[OK] {} status={} latency={}ms",
                    url,
                    resp.status(),
                    latency.as_millis()
                );
            }
            Err(err) => {
                println!(
                    "[ERROR] {} error={} latency={}ms",
                    url,
                    err,
                    latency.as_millis()
                );
            }
        }
    }

    pub async fn run_once(&self, config: &Config) {
        for target in &config.targets {
            self.check(&target.url).await;
        }
    }
}
