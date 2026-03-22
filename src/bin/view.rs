use crossterm::{
    ExecutableCommand,
    event::{self, KeyCode},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    symbols,
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, GraphType, List, ListItem, ListState,
        Paragraph, Row, Table,
    },
};
use ruliadema::model::{CheckHistory, CheckStatus};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, stdout},
    time::Duration,
};

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let mut show_config = false;
    let mut config_content = String::new();

    let mut show_breaches = false;
    let mut scroll_offset: usize = 0;

    loop {
        let histories: BTreeMap<String, CheckHistory> = if let Ok(file) = File::open("status.json")
        {
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
            let size = frame.area();

            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(size);

            if show_config {
                let config_para = Paragraph::new(config_content.as_str())
                    .block(Block::default().title(" config.toml ").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Yellow));

                frame.render_widget(config_para, main_chunks[0]);
            } else {
                let content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                    .split(main_chunks[0]);

                let left_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(10)])
                    .split(content_chunks[0]);

                let items: Vec<ListItem> = urls
                    .iter()
                    .map(|url| {
                        let style = match histories.get(url).and_then(|h| h.results.back()) {
                            Some(res) if matches!(res.status, CheckStatus::Up) => Style::default().fg(Color::Green),
                            Some(_) => Style::default().fg(Color::Red),
                            None => Style::default().fg(Color::DarkGray),
                        };

                        // オフセット分だけ文字をスキップ
                        let display_url = if url.chars().count() > scroll_offset {
                            url.chars().skip(scroll_offset).collect::<String>()
                        } else {
                            String::new()
                        };

                        ListItem::new(display_url).style(style)
                    })
                    .collect();

                let list_title = if scroll_offset > 0 {
                    format!(" URLs (offset: {}) ", scroll_offset)
                } else {
                    " URLs ".to_string()
                };

                let list = List::new(items)
                    .block(Block::default().title(list_title).borders(Borders::ALL))
                    .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
                    .highlight_symbol(">> ");

                frame.render_stateful_widget(list, left_chunks[0], &mut list_state);

                if let Some(selected_idx) = list_state.selected() {
                    if let Some(url) = urls.get(selected_idx) {
                        if let Some(history) = histories.get(url) {

                            // --- Detail パネルの描画 ---
                            let latest = history.results.back();
                            let status_str = latest.map_or("N/A".to_string(), |r| format!("{:?}", r.status));
                            let rt_str = latest.and_then(|r| r.response_time).map_or("N/A".to_string(), |d| format!("{} ms", d.as_millis()));
                            let diff_str = match latest.and_then(|r| r.diff_from_prev) {
                                Some(d) if d > 0 => format!("+{} ms 🔺", d),
                                Some(d) if d < 0 => format!("{} ms 🔽", d),
                                Some(_) => "±0 ms".to_string(),
                                None => "-".to_string(),
                            };
                            let diff_acc_str = match latest.and_then(|r| r.diff_from_acceptable) {
                                Some(d) if d > 0 => format!("+{} ms ⚠️ OVER", d),
                                Some(d) => format!("{} ms OK", d),
                                None => "-".to_string(),
                            };

                            let detail_text = format!(
                                " Target : {}\n\n Status        : {}\n Response time : {}\n\n Response diff : {}\n Limit         : {} ms\n Limit diff    : {}",
                                url, status_str, rt_str, diff_str, history.acceptable_latency_ms, diff_acc_str
                            );
                            let detail_para = Paragraph::new(detail_text)
                                .block(Block::default().title(" Detail ").borders(Borders::ALL));

                            frame.render_widget(detail_para, left_chunks[1]);

                            // --- 右側パネルの描画（モード切替） ---
                            if show_breaches {
                                let mut breach_rows = Vec::new();
                                if let Ok(file) = File::open("breaches.json") {
                                    let reader = BufReader::new(file);
                                    for line in reader.lines().filter_map(|l| l.ok()) {
                                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                                            if v["url"].as_str() == Some(url) {
                                                let rt = v["response_time_ms"].as_u64().unwrap_or(0);
                                                let limit = v["acceptable_latency_ms"].as_u64().unwrap_or(0);
                                                let diff = v["diff_ms"].as_u64().unwrap_or(0);

                                                let ts = v["result"]["timestamp"].as_str().unwrap_or("Unknown");

                                                let style = if v["is_error"].as_bool().unwrap_or(false) {
                                                    Style::default().fg(Color::Red)
                                                } else {
                                                    Style::default().fg(Color::Yellow)
                                                };

                                                breach_rows.push(Row::new(vec![
                                                    Cell::from(ts.to_string()),
                                                    Cell::from(if v["is_error"].as_bool().unwrap_or(false) { "ERROR".to_string() } else { format!("{}ms", rt) }),
                                                    Cell::from(format!("{}ms", limit)),
                                                    Cell::from(format!("+{}ms", diff)),
                                                ]).style(style));
                                            }
                                        }
                                    }
                                }

                                breach_rows.reverse();

                                let table = Table::new(
                                    breach_rows,
                                    [Constraint::Percentage(40), Constraint::Percentage(20), Constraint::Percentage(20), Constraint::Percentage(20)]
                                )
                                .header(Row::new(vec!["Timestamp", "Response", "Limit", "Over"]).style(Style::default().add_modifier(Modifier::BOLD)))
                                .block(Block::default().title(" SLA Breaches History (Press 'b' to back) ").borders(Borders::ALL));

                                frame.render_widget(table, content_chunks[1]);

                            } else {
                                let mut chart_data: Vec<(f64, f64)> = vec![];
                                let mut max_rt = 100.0;
                                let acceptable_rt = history.acceptable_latency_ms as f64;

                                for (i, res) in history.results.iter().enumerate() {
                                    let rt = res.response_time.map(|d| d.as_millis() as f64).unwrap_or(0.0);
                                    if rt > max_rt { max_rt = rt; }
                                    chart_data.push((i as f64, rt));
                                }

                                if acceptable_rt > max_rt { max_rt = acceptable_rt; }

                                let acceptable_data: Vec<(f64, f64)> = vec![
                                    (0.0, acceptable_rt),
                                    (history.results.len().saturating_sub(1) as f64, acceptable_rt),
                                ];

                                let datasets = vec![
                                    Dataset::default()
                                        .name("Response Time (ms)")
                                        .marker(symbols::Marker::Braille)
                                        .graph_type(GraphType::Line)
                                        .style(Style::default().fg(Color::Cyan))
                                        .data(&chart_data),
                                    Dataset::default()
                                        .name(format!("Limit ({} ms)", history.acceptable_latency_ms))
                                        .marker(symbols::Marker::Dot)
                                        .graph_type(GraphType::Line)
                                        .style(Style::default().fg(Color::Yellow))
                                        .data(&acceptable_data),
                                ];

                                let chart = Chart::new(datasets)
                                    .block(Block::default().title(" Latency History (Press 'b' for Breaches) ").borders(Borders::ALL))
                                    .x_axis(Axis::default().bounds([0.0, 50.0]).style(Style::default().fg(Color::Gray)))
                                    .y_axis(Axis::default().bounds([0.0, max_rt * 1.1]).labels(vec![
                                        Span::raw("0"), Span::raw(format!("{}", max_rt as u64)),
                                    ]).style(Style::default().fg(Color::Gray)));

                                frame.render_widget(chart, content_chunks[1]);
                            }
                        }
                    }
                }
            }

            let footer_text = if show_config {
                " c/Esc: Back to Main   q: Quit "
            } else if show_breaches {
                " j/k: Select   h/l: Scroll URL   b: Show Graph   q: Quit "
            } else {
                " j/k: Select   h/l: Scroll URL   b: Show Breaches   c: Config   q: Quit "
            };
            let footer = Paragraph::new(footer_text)
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(footer, main_chunks[1]);
        })?;

        // --- キー入力処理 ---
        if event::poll(Duration::from_millis(500))? {
            if let event::Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('c') => {
                        show_config = !show_config;
                        if show_config {
                            config_content =
                                std::fs::read_to_string("config.toml").unwrap_or_else(|_| {
                                    "Error: config.toml is missing or unreadable.".to_string()
                                });
                        }
                    }
                    KeyCode::Char('b') => {
                        if !show_config {
                            show_breaches = !show_breaches;
                        }
                    }
                    KeyCode::Esc => {
                        if show_config {
                            show_config = false;
                        }
                        if show_breaches {
                            show_breaches = false;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if !show_config {
                            let i = match list_state.selected() {
                                Some(i) => {
                                    if i >= urls.len().saturating_sub(1) {
                                        0
                                    } else {
                                        i + 1
                                    }
                                }
                                None => 0,
                            };
                            list_state.select(Some(i));
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if !show_config {
                            let i = match list_state.selected() {
                                Some(i) => {
                                    if i == 0 {
                                        urls.len().saturating_sub(1)
                                    } else {
                                        i - 1
                                    }
                                }
                                None => 0,
                            };
                            list_state.select(Some(i));
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        if !show_config {
                            scroll_offset = scroll_offset.saturating_add(1);
                        }
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        if !show_config {
                            scroll_offset = scroll_offset.saturating_sub(1);
                        }
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
