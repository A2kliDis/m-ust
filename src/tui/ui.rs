use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Gauge, List, ListItem, Tabs, Wrap},
    Frame,
};
use crate::tui::app::{App, Status};

fn friendly_mode(app: &App) -> String {
    match &app.mode {
        crate::audio::CaptureMode::Microphone(n) => if n.is_empty() {"Microphone".into()} else {n.clone()},
        crate::audio::CaptureMode::SystemLoopback(n) => if n=="Default" {"System Audio".into()} else {n.clone()},
        crate::audio::CaptureMode::AppLoopback(n) => n.clone(),
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Length(3),  // tabs
            Constraint::Length(3),  // status
            Constraint::Min(8),     // content
            Constraint::Length(1),  // footer
        ])
        .split(f.area());

    // Header — minimal, centered, no verbose help
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" M-ust ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("Song Recognizer", Style::default().fg(Color::White)),
        Span::styled("  v0.1  ", Style::default().fg(Color::DarkGray)),
        Span::styled("— 100% free • Rust", Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
    .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    // Tabs — clean, counts only
    let tabs = Tabs::new(vec![
        format!(" Input ({}) ", app.devices.len()),
        format!(" Output ({}) ", app.output_devices.len()),
        format!(" Apps ({}) ", app.apps.len()),
    ])
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
    .select(app.capture_tab)
    .style(Style::default().fg(Color::DarkGray))
    .highlight_style(Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[1]);

    // Status — clean gauge, no Debug
    let (status_label, color, gauge_label) = match app.status {
        Status::Idle => ("Idle — press r to record", Color::DarkGray, format!("{} • {}", friendly_mode(app), "Ready")),
        Status::Recording => ("Recording", Color::Red, format!("{} • {}/{}s", friendly_mode(app), app.elapsed, app.config.record_duration_secs)),
        Status::Identifying => ("Identifying", Color::Yellow, format!("{} • Analyzing...", friendly_mode(app))),
        Status::Done => ("Done", Color::Green, format!("{} • {}", friendly_mode(app), "Matched")),
        Status::Error => ("Error", Color::Red, format!("{} • {}", friendly_mode(app), "No match")),
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(format!(" {} ", status_label)))
        .gauge_style(Style::default().fg(color).bg(Color::Black))
        .percent((app.progress*100.0) as u16)
        .label(Span::styled(gauge_label, Style::default().fg(Color::White)));
    f.render_widget(gauge, chunks[2]);

    // Content
    let content = chunks[3];
    if let Some(song) = &app.result {
        let text_lines = vec![
            Line::from(vec![Span::styled(&song.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled(&song.artist, Style::default().fg(Color::Cyan))]),
            Line::from(vec![Span::styled(song.album.clone().unwrap_or_default(), Style::default().fg(Color::DarkGray))]),
            Line::from(vec![Span::styled(song.url.clone().unwrap_or_default(), Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED))]),
        ];
        match &song.cover {
            Some(cover) => {
                // Adaptive size: fit the content area (image pane <= half width).
                // Falls back to text-only on tiny terminals — never overflows.
                let ch = content.height as usize;
                let cw = content.width as usize;
                let rows = (ch.saturating_sub(2)).min((cw / 4).saturating_sub(1)).clamp(0, 20);
                if rows < 6 || cw < 60 {
                    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)).title(" Result (o=open song) ");
                    let p = Paragraph::new(text_lines).block(block).wrap(Wrap{trim:true});
                    f.render_widget(p, content);
                } else {
                    let img_w = (rows * 2) as u16;
                    // Cover (rows x 2*rows half-blocks) + text side by side
                    let cols = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Length(img_w + 2), Constraint::Min(0)])
                        .split(content);
                    let mut img_lines = Vec::with_capacity(rows);
                    for row in 0..rows {
                        let mut spans = Vec::with_capacity(rows * 2);
                        for col in 0..rows * 2 {
                            let sx = (col * cover.w as usize / (rows * 2)) as u32;
                            let sy = (row * 2 * cover.h as usize / (rows * 2)) as u32;
                            let (fr, fg, fb) = cover.pixel(sx, sy);
                            let (br, bg, bb) = cover.pixel(sx, sy + 1);
                            spans.push(Span::styled("▀", Style::default().fg(Color::Rgb(fr, fg, fb)).bg(Color::Rgb(br, bg, bb))));
                        }
                        img_lines.push(Line::from(spans));
                    }
                    let img = Paragraph::new(img_lines)
                        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)).title(" Cover "));
                    f.render_widget(img, cols[0]);
                    let p = Paragraph::new(text_lines)
                        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)).title(" Result (o=open song) "))
                        .wrap(Wrap{trim:true});
                    f.render_widget(p, cols[1]);
                }
            }
            None => {
                let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Green)).title(" Result (o=open song) ");
                let p = Paragraph::new(text_lines).block(block).wrap(Wrap{trim:true});
                f.render_widget(p, content);
            }
        }
    } else if let Some(err) = &app.error {
        let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Red)).title(" Result ");
        let short = if err.len()>120 { format!("{}…", &err[..120]) } else { err.clone() };
        let p = Paragraph::new(vec![
            Line::from(Span::styled(short, Style::default().fg(Color::Red))),
            Line::from(Span::raw("")),
            Line::from(Span::styled("Press r to try again • g to refresh", Style::default().fg(Color::DarkGray))),
        ]).block(block).wrap(Wrap{trim:true});
        f.render_widget(p, content);
    } else {
        let cols = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(42), Constraint::Percentage(58)]).split(content);
        // Device list — clean, star for default
        let (title, items) = match app.capture_tab {
            0 => {
                let title = " Input ";
                let items: Vec<ListItem> = app.devices.iter().enumerate().map(|(i,n)| {
                    let is_def = app.config.default_input.as_ref()==Some(n);
                    let sel = i==app.selected_device;
                    let style = if sel { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) };
                    let prefix = if sel {"▶ "} else {"  "};
                    let suffix = if is_def {"  ★"} else {""};
                    ListItem::new(format!("{}{}{}", prefix, n, suffix)).style(style)
                }).collect();
                (title, items)
            },
            1 => {
                let title = " Output ";
                let items: Vec<ListItem> = app.output_devices.iter().enumerate().map(|(i,n)| {
                    let is_def = app.config.default_output.as_ref()==Some(n);
                    let sel = i==app.selected_output;
                    let style = if sel { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) };
                    let prefix = if sel {"▶ "} else {"  "};
                    let suffix = if is_def {"  ★"} else {""};
                    ListItem::new(format!("{}{}{}", prefix, n, suffix)).style(style)
                }).collect();
                (title, items)
            },
            _ => {
                let title = " Apps — live ";
                let items: Vec<ListItem> = app.apps.iter().enumerate().map(|(i,n)| {
                    let sel = i==app.selected_app;
                    let style = if sel { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
                    let peak = app.app_peaks.get(i).copied().unwrap_or(0.0);
                    let wave = if peak>0.6 {" ▇▇▇"} else if peak>0.3 {" ▂▅▇"} else if peak>0.08 {" ▁▂▃"} else if peak>0.02 {" ▁"} else {""};
                    let prefix = if sel {"▶ "} else {"  "};
                    ListItem::new(format!("{}{}{}", prefix, n, wave)).style(style)
                }).collect();
                (title, items)
            },
        };
        let list = List::new(items).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(title));
        f.render_widget(list, cols[0]);
        let logs: Vec<ListItem> = app.log.iter().rev().take(10).rev().map(|l| {
            let style = if l.starts_with("Failed") || l.starts_with("No match") { Style::default().fg(Color::Red) }
                else if l.contains(" - ") && app.result.is_some() { Style::default().fg(Color::Green) }
                else { Style::default().fg(Color::DarkGray) };
            ListItem::new(l.as_str()).style(style)
        }).collect();
        let log_block = List::new(logs).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" Log "));
        f.render_widget(log_block, cols[1]);
    }

    // Footer — single minimal help line
    let loop_style = if app.continuous { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) };
    let loop_label = if app.continuous { " loop:ON  " } else { " loop  " };
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" r", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)), Span::styled(" record  ", Style::default().fg(Color::DarkGray)),
        Span::styled(" l", loop_style), Span::styled(loop_label, Style::default().fg(Color::DarkGray)),
        Span::styled("h", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::styled(" history  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::styled(" switch  ", Style::default().fg(Color::DarkGray)),
        Span::styled("↑↓", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)), Span::styled(" nav  ", Style::default().fg(Color::DarkGray)),
        Span::styled("g", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)), Span::styled(" refresh  ", Style::default().fg(Color::DarkGray)),
        Span::styled("d", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)), Span::styled(" default  ", Style::default().fg(Color::DarkGray)),
        Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)), Span::styled(" quit", Style::default().fg(Color::DarkGray)),
    ])).alignment(Alignment::Center);
    f.render_widget(footer, chunks[4]);
}
