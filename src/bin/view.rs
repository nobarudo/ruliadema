use crossterm::{
    event::{self, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem, ListState, Paragraph},
};
use ruliadema::model::{CheckHistory, CheckStatus};
use std::{collections::BTreeMap, fs::File, io::stdout, time::Duration};

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    loop {
        let histories: BTreeMap<String, CheckHistory> =
            if let Ok(file) = File::open("status.json") {
                serde_json::from_reader(file).unwrap_or_default()
            } else {
                BTreeMap::new()
            };

        let urls: Vec<String> = histories.keys().cloned().collect();

        if let Some(selected) = list_state.selected() {
            if selected >= urls.len() && !urls.is_empty() {
                list_state.select(Some(urls.len() - 1));
            }
        }

        terminal.draw(|frame| {
            let size = frame.size();

            // 1. 全体を「メイン領域」と「フッター（1行）」に上下分割
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(size);

            // 2. メイン領域を「左（リストと詳細 30%）」と「右（グラフ 70%）」に左右分割
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(main_chunks[0]);

            // ▼ 追加: 左側をさらに「上（URL一覧 残り全部）」と「下（詳細情報 8行）」に上下分割
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(8)])
                .split(content_chunks[0]);

            // ==========================================
            // 左上ペイン：URLリストの描画
            // ==========================================
            let items: Vec<ListItem> = urls
                .iter()
                .map(|url| {
                    let style = match histories.get(url).and_then(|h| h.results.back()) {
                        Some(res) if matches!(res.status, CheckStatus::Up) => Style::default().fg(Color::Green),
                        Some(_) => Style::default().fg(Color::Red),
                        None => Style::default().fg(Color::DarkGray),
                    };
                    ListItem::new(url.clone()).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().title(" URLs ").borders(Borders::ALL))
                .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
                .highlight_symbol(">> ");
            
            frame.render_stateful_widget(list, left_chunks[0], &mut list_state);

            // ==========================================
            // 詳細とグラフの描画（選択されているURLのデータを取得）
            // ==========================================
            if let Some(selected_idx) = list_state.selected() {
                if let Some(url) = urls.get(selected_idx) {
                    if let Some(history) = histories.get(url) {
                        
                        // ==========================================
                        // 左下ペイン：詳細情報の描画
                        // ==========================================
                        let latest = history.results.back();
                        let status_str = latest.map_or("N/A".to_string(), |r| format!("{:?}", r.status));
                        let rt_str = latest.and_then(|r| r.response_time).map_or("N/A".to_string(), |d| format!("{} ms", d.as_millis()));
                        
                        // ▼ 差分（diff_from_prev）のフォーマット
                        let diff_str = match latest.and_then(|r| r.diff_from_prev) {
                            Some(d) if d > 0 => format!("+{} ms 🔺", d),
                            Some(d) if d < 0 => format!("{} ms 🔽", d),
                            Some(_) => "±0 ms".to_string(),
                            None => "-".to_string(),
                        };
                        
                        let detail_text = format!(
                            " Target :\n {}\n\n Status : {}\n Latency: {}\n Diff   : {}",
                            url, status_str, rt_str, diff_str
                        );
                        let detail_para = Paragraph::new(detail_text)
                            .block(Block::default().title(" Detail ").borders(Borders::ALL));
                        
                        // 左下の枠（left_chunks[1]）に詳細を描画
                        frame.render_widget(detail_para, left_chunks[1]);

                        // ==========================================
                        // 右ペイン：グラフの描画
                        // ==========================================
                        let mut chart_data: Vec<(f64, f64)> = vec![];
                        let mut max_rt = 100.0;

                        for (i, res) in history.results.iter().enumerate() {
                            let rt = res.response_time.map(|d| d.as_millis() as f64).unwrap_or(0.0);
                            if rt > max_rt {
                                max_rt = rt;
                            }
                            chart_data.push((i as f64, rt));
                        }

                        let datasets = vec![Dataset::default()
                            .name("Response Time (ms)")
                            .marker(symbols::Marker::Braille)
                            .graph_type(GraphType::Line)
                            .style(Style::default().fg(Color::Cyan))
                            .data(&chart_data)];

                        let chart = Chart::new(datasets)
                            .block(Block::default().title(" Latency History ").borders(Borders::ALL))
                            .x_axis(
                                Axis::default()
                                    .title("Time (older -> newer)")
                                    .bounds([0.0, 50.0])
                                    .style(Style::default().fg(Color::Gray)),
                            )
                            .y_axis(
                                Axis::default()
                                    .title("ms")
                                    .bounds([0.0, max_rt * 1.1])
                                    .labels(vec![
                                        Span::raw("0"),
                                        Span::raw(format!("{}", max_rt as u64)),
                                    ])
                                    .style(Style::default().fg(Color::Gray)),
                            );
                        
                        // 右側の大きな枠（content_chunks[1]）にグラフを描画
                        frame.render_widget(chart, content_chunks[1]);
                    }
                }
            }

            // フッター領域
            let footer = Paragraph::new(" ↑/↓: Select URL   q: Quit ")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(footer, main_chunks[1]);
        })?;

        if event::poll(Duration::from_millis(500))? {
            if let event::Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => {
                        let i = match list_state.selected() {
                            Some(i) => if i >= urls.len().saturating_sub(1) { 0 } else { i + 1 },
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                    KeyCode::Up => {
                        let i = match list_state.selected() {
                            Some(i) => if i == 0 { urls.len().saturating_sub(1) } else { i - 1 },
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}