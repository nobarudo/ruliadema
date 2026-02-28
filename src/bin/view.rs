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
        Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem, ListState, Paragraph,
    },
};
use ruliadema::model::{CheckHistory, CheckStatus};
use std::{collections::BTreeMap, fs::File, io::stdout, time::Duration};

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let mut show_config = false;
    let mut config_content = String::new();

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
            // ▼ 警告修正: size() ではなく area() を使う
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
                    .constraints([Constraint::Percentage(35), Constraint::Percentage(65)]) // 35:65のレイアウト
                    .split(main_chunks[0]);

                let left_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(10)]) // Detail用の15行
                    .split(content_chunks[0]);

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

                if let Some(selected_idx) = list_state.selected() {
                    if let Some(url) = urls.get(selected_idx) {
                        if let Some(history) = histories.get(url) {

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
                                .block(Block::default().title(" Latency History ").borders(Borders::ALL))
                                .x_axis(Axis::default().bounds([0.0, 50.0]).style(Style::default().fg(Color::Gray)))
                                .y_axis(Axis::default().bounds([0.0, max_rt * 1.1]).labels(vec![
                                    Span::raw("0"), Span::raw(format!("{}", max_rt as u64)),
                                ]).style(Style::default().fg(Color::Gray)));

                            frame.render_widget(chart, content_chunks[1]);
                        }
                    }
                }
            }

            let footer_text = if show_config {
                " c/Esc: Back to Main   q: Quit "
            } else {
                " j/k or ↓/↑: Select URL   c: Config   q: Quit "
            };
            let footer = Paragraph::new(footer_text)
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(footer, main_chunks[1]);
        })?;

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
                    KeyCode::Esc => {
                        if show_config {
                            show_config = false;
                        }
                    }
                    // ▼ 修正: ご自身で書かれた美しいパイプによる条件分岐！
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
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
