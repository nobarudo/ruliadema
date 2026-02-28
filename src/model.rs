use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CheckStatus {
    Up,
    Down,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub timestamp: DateTime<Utc>,
    pub status: CheckStatus,
    pub response_time: Option<Duration>,
    pub diff_from_prev: Option<i128>,
    #[serde(default)] // ← 古いstatus.json対策
    pub diff_from_acceptable: Option<i128>, // 許容時間との差分
}

const MAX_HISTORY: usize = 50;

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckHistory {
    pub url: String,
    #[serde(default)] // ← 古いstatus.json対策
    pub acceptable_latency_ms: u64, // このURLの許容時間
    pub results: VecDeque<CheckResult>,
}

impl CheckHistory {
    // 引数に acceptable_latency_ms を追加
    pub fn new(url: String, acceptable_latency_ms: u64) -> Self {
        Self {
            url,
            acceptable_latency_ms,
            results: VecDeque::with_capacity(MAX_HISTORY),
        }
    }

    pub fn push(&mut self, mut result: CheckResult) {
        // 許容時間との差分を計算（プラスなら超過、マイナスなら余裕あり）
        if let Some(curr_rt) = result.response_time {
            let diff = curr_rt.as_millis() as i128 - self.acceptable_latency_ms as i128;
            result.diff_from_acceptable = Some(diff);
        }

        // 前回との差分を計算
        if let Some(prev) = self.results.back() {
            if let (Some(prev_rt), Some(curr_rt)) = (prev.response_time, result.response_time) {
                let diff = curr_rt.as_millis() as i128 - prev_rt.as_millis() as i128;
                result.diff_from_prev = Some(diff);
            }
        }

        if self.results.len() == MAX_HISTORY {
            self.results.pop_front();
        }
        self.results.push_back(result);
    }
}
