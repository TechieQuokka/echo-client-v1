use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, LineKind};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 상태바
            Constraint::Min(0),    // 메시지창
            Constraint::Length(3), // 입력창
        ])
        .split(frame.area());

    // ── 상태바 ──────────────────────────────────────────────────────────────
    let status_text = if !app.connected {
        " connecting...".to_string()
    } else if let Some(ref room) = app.current_room {
        let names: Vec<&str> = app.members.iter().map(|m| m.nickname.as_str()).collect();
        let members_str = if names.is_empty() {
            "(none)".to_string()
        } else {
            names.join(", ")
        };
        format!(" room: #{} | members: {}", room, members_str)
    } else {
        format!(" {} | no room (/join <room>)", app.nickname)
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD));
    frame.render_widget(status, chunks[0]);

    // ── 메시지창 ─────────────────────────────────────────────────────────────
    let lines: Vec<Line> = app.messages.iter().map(|line| match &line.kind {
        LineKind::Message { from } => Line::from(vec![
            Span::styled(
                format!("[{}] ", line.time),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{}: ", from),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw(line.text.clone()),
        ]),
        LineKind::System => Line::from(Span::styled(
            format!("[{}] {}", line.time, line.text),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
        )),
        LineKind::Error => Line::from(Span::styled(
            format!("[{}] ERROR: {}", line.time, line.text),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
    }).collect();

    let total = lines.len();
    let scroll = if total == 0 { 0 } else { app.scroll.min(total - 1) } as u16;

    let messages = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(messages, chunks[1]);

    // ── 입력창 ───────────────────────────────────────────────────────────────
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(" input ");
    let inner = input_block.inner(chunks[2]);
    frame.render_widget(input_block, chunks[2]);

    let display = format!("> {}", app.input);
    frame.render_widget(Paragraph::new(display.as_str()), inner);

    // 커서 위치: "> " (2자) + input 내 문자 수
    let visual_offset = app.input[..app.cursor].chars().count() as u16;
    let cx = (inner.x + 2 + visual_offset).min(inner.x + inner.width.saturating_sub(1));
    let cy = inner.y;
    frame.set_cursor_position((cx, cy));
}
