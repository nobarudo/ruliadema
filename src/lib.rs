use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use tokio::sync::Semaphore;

mod config;
pub use config::Config;

pub mod checker;

pub struct HttpChecker {
    client: Client,
    semaphore: Arc<Semaphore>,
}

pub struct CheckResult {
    pub url: String,
    pub success: bool,
    pub status_code: Option<u16>,
    pub latency: Duration,
}

impl HttpChecker {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()?;

        Ok(Self {
            client,
            semaphore: Arc::new(Semaphore::new(config.max_concurrency)),
        })
    }

    pub async fn run_once(&self, config: &Config) {
        let mut handles = Vec::new();

        for target in &config.targets {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let client = self.client.clone();
            let url = target.url.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;

                match client.get(&url).send().await {
                    Ok(resp) => {
                        println!("[OK] {} status={}", url, resp.status());
                    }
                    Err(e) => {
                        println!("[ERROR] {} {}", url, e);
                    }
                }
            });

            handles.push(handle);
        }

        for h in handles {
            let _ = h.await;
        }
    }
}
