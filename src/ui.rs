use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, InputUnit, Modal, Screen};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    match app.screen {
        Screen::Conversations => draw_conversations(frame, app, chunks[0]),
        Screen::Messages => draw_messages(frame, app, chunks[0]),
    }
    frame.render_widget(
        Paragraph::new(app.status.as_str()).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );

    match &app.modal {
        Modal::None => {}
        Modal::ExportMenu { selected } => draw_export_menu(frame, *selected),
        Modal::NumberInput { unit, value } => draw_number_input(frame, *unit, value),
        Modal::PathInput { value, .. } => draw_path_input(frame, value),
        Modal::Notice(message) => draw_notice(frame, message),
    }
}

fn draw_conversations(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .visible_conversations()
        .map(|conversation| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    conversation.name.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    conversation
                        .last_date
                        .format("%b %-d, %-I:%M %p")
                        .to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();
    let title = match &app.conversation_search {
        Some(query) if !query.is_empty() => format!(" Conversations matching \"{query}\" "),
        Some(_) => " Search conversations ".to_string(),
        None => " Recent conversations ".to_string(),
    };
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("› ")
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));
    frame.render_stateful_widget(list, area, &mut app.conversation_state);
}

fn draw_messages(frame: &mut Frame, app: &mut App, area: Rect) {
    let title = app
        .current_conversation()
        .map(|conversation| format!(" {} ", conversation.name))
        .unwrap_or_else(|| " Messages ".to_string());
    let items: Vec<ListItem> = app
        .messages
        .iter()
        .map(|message| {
            let header = Line::from(vec![
                Span::styled(
                    message.sender.clone(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    message.date.format("%Y-%m-%d %-I:%M:%S %p").to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            let mut lines = vec![header];
            lines.extend(
                message
                    .display_body()
                    .lines()
                    .map(|line| Line::raw(format!("  {line}"))),
            );
            lines.push(Line::raw(""));
            ListItem::new(lines)
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_symbol("│ ")
        .highlight_style(Style::default().bg(Color::Rgb(35, 35, 45)));
    frame.render_stateful_widget(list, area, &mut app.message_state);
}

fn draw_export_menu(frame: &mut Frame, selected: usize) {
    let options = [
        "Last hour",
        "Last 24 hours",
        "Choose number of hours",
        "Choose number of days",
        "Everything",
    ];
    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let style = if index == selected {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            ListItem::new(*option).style(style)
        })
        .collect();
    let area = centered_rect(48, 11, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .title(" Export range ")
                .borders(Borders::ALL),
        ),
        area,
    );
}

fn draw_number_input(frame: &mut Frame, unit: InputUnit, value: &str) {
    let label = match unit {
        InputUnit::Hours => "Number of hours",
        InputUnit::Days => "Number of days",
    };
    draw_input(frame, label, value, 50);
}

fn draw_path_input(frame: &mut Frame, value: &str) {
    draw_input(
        frame,
        "Save Markdown file (current directory by default)",
        value,
        78,
    );
}

fn draw_input(frame: &mut Frame, title: &str, value: &str, width: u16) {
    let area = centered_rect(width, 5, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(value)
            .block(
                Block::default()
                    .title(format!(" {title} "))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_notice(frame: &mut Frame, message: &str) {
    let area = centered_rect(72, 9, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(message)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Export complete ")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width.saturating_sub(2));
    let height = height.min(area.height.saturating_sub(2));
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}
