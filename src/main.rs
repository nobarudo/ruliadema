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
                // ★ 修正: diff の計算と50件の制限はすべて model.rs の push メソッドにお任せ！
                history.push(result);

                // ★ 修正: 今 push されたばかりの最新のデータを取得してログに出力
                if let Some(latest_result) = history.results.back() {
                    print_log(&url, latest_result);
                }
            }
        }
    }
}
