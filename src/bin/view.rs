use crossterm::{
    event::{self, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout}, // ← 追加: 画面を分割するための機能
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use ruliadema::model::CheckHistory;
use std::{collections::BTreeMap, fs::File, io::stdout, time::Duration};

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    loop {
        let histories: BTreeMap<String, CheckHistory> =
            if let Ok(file) = File::open("status.json") {
                serde_json::from_reader(file).unwrap_or_default()
            } else {
                BTreeMap::new()
            };

        terminal.draw(|frame| {
            let size = frame.size();

            // ▼ 追加: 画面を「メイン部分」と「高さ1行のフッター部分」に縦分割する
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),    // 上部：残りの空間をすべて使う
                    Constraint::Length(1), // 下部：高さ1行分だけ絶対に確保する
                ])
                .split(size);

            let mut text = String::new();
            for (url, history) in &histories {
                let status_str = match history.results.back() {
                    Some(result) => {
                        let rt = result.response_time.map_or(0, |d| d.as_millis());
                        format!("{:?} ({}ms)", result.status, rt)
                    }
                    None => "No data".to_string(),
                };
                text.push_str(&format!("{:<30} => {}\n", url, status_str));
            }

            // メイン画面は chunks[0]（上の広い部分）に描画
            let main_paragraph = Paragraph::new(text)
                .block(Block::default().title(" Ruliadema Viewer ").borders(Borders::ALL));
            frame.render_widget(main_paragraph, chunks[0]);

            // ▼ 追加: フッターは chunks[1]（一番下の1行）に描画
            // 控えめなグレー色にして、右側に寄せるなどの装飾も可能です
            let footer = Paragraph::new(" q: quit ")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(footer, chunks[1]);
        })?;

        if event::poll(Duration::from_millis(1000))? {
            if let event::Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}