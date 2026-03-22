use ruliadema::Config;
use ruliadema::checker::HttpChecker;
use ruliadema::model::CheckHistory;
use ruliadema::output::print_log;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{Duration, interval};

use std::fs::{File, OpenOptions};
use std::io::Write;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // config 読み込み
    let config = Config::from_file("config.toml")?;

    // checker 初期化
    let checker = Arc::new(HttpChecker::new(config.timeout_seconds)?);

    // 状態の復元
    let mut histories: HashMap<String, CheckHistory> = if let Ok(file) = File::open("status.json") {
        serde_json::from_reader(file).unwrap_or_default()
    } else {
        HashMap::new()
    };

    for target in &config.targets {
        let history = histories
            .entry(target.url.clone())
            .or_insert_with(|| CheckHistory::new(target.url.clone(), target.acceptable_latency_ms));
        history.acceptable_latency_ms = target.acceptable_latency_ms;
    }

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
                // 判定用に許容時間を取得しておく
                let acceptable_ms = history.acceptable_latency_ms;

                history.push(result);

                if let Some(latest_result) = history.results.back() {
                    // コンソールへの出力
                    print_log(&url, latest_result);

                    let rt_ms = latest_result
                        .response_time
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);

                    // 永久保存用ログ (ruliadema.log) への追記
                    if let Ok(mut log_file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("ruliadema.log")
                    {
                        let log_entry = serde_json::json!({
                            "url": url,
                            "response_time_ms": rt_ms,
                            "result": latest_result
                        });
                        if let Ok(json_line) = serde_json::to_string(&log_entry) {
                            let _ = writeln!(log_file, "{}", json_line);
                        }
                    }

                    // タイムアウト(取得失敗) または レスポンスタイムが許容時間を超えた場合
                    let is_error = latest_result.response_time.is_none();
                    if is_error || rt_ms > acceptable_ms {
                        if let Ok(mut breach_file) = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("breaches.json")
                        {
                            let breach_entry = serde_json::json!({
                                "url": url,
                                "response_time_ms": rt_ms,
                                "acceptable_latency_ms": acceptable_ms,
                                "diff_ms": if is_error { 0 } else { rt_ms.saturating_sub(acceptable_ms) },
                                "is_error": is_error,
                                "result": latest_result // タイムスタンプやステータスコードを含めるため
                            });

                            if let Ok(json_line) = serde_json::to_string(&breach_entry) {
                                let _ = writeln!(breach_file, "{}", json_line);
                            }
                        }
                    }
                    // ▲▲ ここまで ▲▲
                }
            }
        }

        // 最新状態のスナップショット保存
        if let Ok(file) = File::create("status.json") {
            if let Err(e) = serde_json::to_writer_pretty(file, &histories) {
                eprintln!("JSONの保存に失敗しました: {}", e);
            }
        }
    }
}
