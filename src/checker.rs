use crate::Config;

pub struct HttpChecker;

impl HttpChecker {
    pub fn new(_config: &Config) -> anyhow::Result<Self> {
        Ok(Self)
    }

    pub async fn run_once(&self, config: &Config) {
        for target in &config.targets {
            println!("checking {}", target.url);
            // TODO: ここでHTTPリクエスト
        }
    }
}
