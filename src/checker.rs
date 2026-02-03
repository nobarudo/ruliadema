use std::time::{Duration, Instant};
use chrono::Utc;
use reqwest::Client;

use crate::model::{CheckResult, CheckStatus};

pub struct HttpChecker {
    client: Client,
}

impl HttpChecker {
    pub fn new(timeout_seconds: u64) -> anyhow::Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()?;

        Ok(Self { client })
    }

    pub async fn check_once(&self, url: &str) -> CheckResult {
        let start = Instant::now();
        let result = self.client.get(url).send().await;

        match result {
            Ok(resp) => {
                let status = if resp.status().is_success() {
                    CheckStatus::Up
                } else {
                    CheckStatus::Down
                };

                CheckResult {
                    timestamp: Utc::now(),
                    status,
                    response_time: Some(start.elapsed()),
                    diff_from_prev: None,
                }
            }
            Err(_) => CheckResult {
                timestamp: Utc::now(),
                status: CheckStatus::Error,
                response_time: None,
                diff_from_prev: None,
            },
        }
    }
}
