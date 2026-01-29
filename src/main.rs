use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use reqwest::Client;
use tokio::{signal, sync::Semaphore, time};

mod config;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_file("config.toml")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(config.timeout_seconds))
        .build()?;

    let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
    let mut interval = time::interval(Duration::from_secs(config.interval_seconds));

    println!("http-check daemon started");

    loop {
        tokio::select! {
            _ = interval.tick() => {
                run_check(&client, &config, semaphore.clone()).await;
            }
            _ = shutdown_signal() => {
                println!("shutdown signal received");
                break;
            }
        }
    }

    Ok(())
}

async fn run_check(client: &Client, config: &Config, semaphore: Arc<Semaphore>) {
    let mut handles = Vec::new();

    for target in &config.targets {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let c = client.clone();
        let url = target.url.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit; // scopeで自動解放

            match c.get(&url).send().await {
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

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
}
