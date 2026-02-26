use ruliadema::checker::HttpChecker;
use ruliadema::Config;
use ruliadema::model::CheckHistory;
use ruliadema::output::print_log;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{interval, Duration};

// ▼ ログ保存用のパッケージを追加
use std::fs::{File, OpenOptions};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    // config 読み込み
    let config = Config::from_file("config.toml")?;

    // checker 初期化
    let checker = Arc::new(HttpChecker::new(config.timeout_seconds)?);

    // ▼ 状態の復元（前回終了時の status.json があれば読み込む）
    let mut histories: HashMap<String, CheckHistory> = 
        if let Ok(file) = File::open("status.json") {
            serde_json::from_reader(file).unwrap_or_default()
        } else {
            HashMap::new()
        };

    // 新規追加されたURLがあれば histories に登録
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

            handles.push(tokio::spawn(async move {
                let result = checker.check_once(&url).await;
                drop(permit);
                (url, result)
            }));
        }

        // 結果を反映
        for handle in handles {
            let (url, result) = handle.await?;

            if let Some(history) = histories.get_mut(&url) {
                history.push(result);

                // 最新の結果を取得して処理する
                if let Some(latest_result) = history.results.back() {
                    // ① コンソールへの出力
                    print_log(&url, latest_result);

                    // ② 永久保存用ログ (ruliadema.log) への追記
                    if let Ok(mut log_file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("ruliadema.log")
                    {
                        let log_entry = serde_json::json!({
                            "url": url,
                            "result": latest_result
                        });
                        
                        if let Ok(json_line) = serde_json::to_string(&log_entry) {
                            let _ = writeln!(log_file, "{}", json_line);
                        }
                    }
                }
            }
        } 

        // ③ 最新状態のスナップショット (status.json) への上書き保存
        if let Ok(file) = File::create("status.json") {
            if let Err(e) = serde_json::to_writer_pretty(file, &histories) {
                eprintln!("JSONの保存に失敗しました: {}", e);
            }
        }

    } 
} 