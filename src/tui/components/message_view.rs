use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Render the message view
pub fn render_messages(frame: &mut Frame, app: &App, area: Rect) {
    let conversations = app.conversations.read().unwrap();
    let state = app.state.read().unwrap();

    let border_color = if state.focus == crate::app::Focus::Chat {
        Color::Blue
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" Messages ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    // Get messages from selected conversation
    let lines: Vec<Line> = if let Some(conv) = conversations.selected() {
        if conv.messages.is_empty() {
            vec![Line::from(Span::styled(
                "No messages yet. Send the first message!",
                Style::default().fg(Color::DarkGray),
            ))]
        } else {
            conv.messages
                .iter()
                .flat_map(|msg| {
                    let mut lines = Vec::new();

                    // Sender and timestamp
                    let sender_style = if msg.is_ours {
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    };

                    let sender_name = if msg.is_ours {
                        "You".to_string()
                    } else {
                        msg.sender_short()
                    };

                    let pending_indicator = if msg.is_pending { " (pending)" } else { "" };

                    lines.push(Line::from(vec![
                        Span::styled(sender_name, sender_style),
                        Span::styled(
                            format!(" [{}]{}", msg.time_short(), pending_indicator),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));

                    // Message content
                    let content_style = if msg.is_pending {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    // Wrap long messages
                    let max_width = area.width.saturating_sub(4) as usize;
                    for line in wrap_text(&msg.content, max_width) {
                        lines.push(Line::from(Span::styled(format!("  {}", line), content_style)));
                    }

                    // Add empty line between messages
                    lines.push(Line::from(""));

                    lines
                })
                .collect()
        }
    } else {
        vec![Line::from(Span::styled(
            "Select a conversation to view messages",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    // Calculate scroll to show latest messages
    let visible_height = area.height.saturating_sub(2) as usize;
    let scroll = if lines.len() > visible_height {
        (lines.len() - visible_height) as u16
    } else {
        0
    };

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}

/// Simple text wrapping
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}
