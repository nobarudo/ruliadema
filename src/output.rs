use crate::model::CheckResult;

pub fn print_log(url: &str, result: &CheckResult) {
    match result.status {
        crate::model::CheckStatus::Up => {
            let diff_str = match result.diff_from_prev {
                Some(d) => format!("{}ms", d),
                None => " -".to_string(),
            };

            println!(
                "[OK] {} response_time={}ms diff_from_prev={}",
                url,
                result
                    .response_time
                    .map(|d| d.as_millis())
                    .unwrap_or(0),
                diff_str
            );
        }
        crate::model::CheckStatus::Down => {
            println!("[NG] {} down", url);
        }
        crate::model::CheckStatus::Error => {
            println!("[ERR] {} error", url);
        }
    }
}
