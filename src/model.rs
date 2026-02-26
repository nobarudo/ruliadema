use std::collections::VecDeque;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

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
}

const MAX_HISTORY: usize = 50;

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckHistory {
    pub url: String,
    pub results: VecDeque<CheckResult>,
}

impl CheckHistory {
    pub fn new(url: String) -> Self {
        Self {
            url,
            results: VecDeque::with_capacity(MAX_HISTORY),
        }
    }

    pub fn push(&mut self, mut result: CheckResult) {
        if let Some(prev) = self.results.back() {
            if let (Some(prev_rt), Some(curr_rt)) = (prev.response_time, result.response_time)
            {
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
