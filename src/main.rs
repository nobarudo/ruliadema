use ruliadema::checker::HttpChecker;
use ruliadema::Config;
use ruliadema::model::CheckHistory;
use ruliadema::output::print_log;
use std::fs::File;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    // config 読み込み
    let config = Config::from_file("config.toml")?;

    // checker 初期化
    let checker = Arc::new(HttpChecker::new(config.timeout_seconds)?);

    // URLごとの履歴
    let mut histories: HashMap<String, CheckHistory> = 
        if let Ok(file) = std::fs::File::open("status.json") {
            // 前回デーモンが終了した時のデータを復元！
            serde_json::from_reader(file).unwrap_or_default()
        } else {
            HashMap::new()
        };

    // config.toml に新しく追加されたURLがあれば、ここで history を作成してあげる
    for target in &config.targets {
        histories.entry(target.url.clone())
            .or_insert_with(|| CheckHistory::new(target.url.clone()));
    }

    // 並列数制限
    let semaphore = Arc::new(Semaphore::new(config.max_concurrency));

    let mut ticker = interval(Duration::from_secs(config.interval_seconds));

    loop {
        ticker.tick().await;

        let mut handles = Vec::new();

        for history in histories.values_mut() {
            let permit = semaphore.clone().acquire_owned().await?;
            let checker = checker.clone();
            let url = history.url.clone();

            // history をまたいで borrow できないので結果だけ返す
            handles.push(tokio::spawn(async move {
                let result = checker.check_once(&url).await;
                drop(permit);
                (url, result)
            }));
        }

        for handle in handles {
            let (url, result) = handle.await?;

            if let Some(history) = histories.get_mut(&url) {
                history.push(result);
                if let Some(latest_result) = history.results.back() {
                    print_log(&url, latest_result);
                }
            }
        }

        // ▼▼▼ ここを追加：毎回のチェックが終わったら JSON に上書き保存 ▼▼▼
        if let Ok(file) = File::create("status.json") {
            if let Err(e) = serde_json::to_writer_pretty(file, &histories) {
                eprintln!("JSONの保存に失敗しました: {}", e);
            }
        }
    }
}
