use ruliadema::checker::HttpChecker;
use ruliadema::Config;
use ruliadema::model::CheckHistory;
use ruliadema::output::print_log;

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
    let mut histories: HashMap<String, CheckHistory> = HashMap::new();
    for target in &config.targets {
        histories.insert(
            target.url.clone(),
            CheckHistory::new(target.url.clone()),
        );
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

        // 結果を反映
        for handle in handles {
            let (url, result) = handle.await?;

            if let Some(history) = histories.get_mut(&url) {
                let mut result = result; // ★ ここで mut にする

                // ★★★ diff を計算する場所 ★★★
                if let (Some(prev), Some(curr_rt)) = (history.results.back(), result.response_time){
                    if let Some(prev_rt) = prev.response_time {
                        let diff = curr_rt.as_millis() as i128 - prev_rt.as_millis() as i128;
                        result.diff_from_prev = Some(diff);
                    }
                }

                // ★ 最後に push
                history.results.push_back(result.clone());

                // ★ ログ出力
                print_log(&url, &result);
            }


        }

    }
}
