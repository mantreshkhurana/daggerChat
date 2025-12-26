use crate::app::App;
use crate::chat::ConversationType;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

/// Render the conversation list sidebar
pub fn render_chat_list(frame: &mut Frame, app: &App, area: Rect) {
    let conversations = app.conversations.read().unwrap();
    let state = app.state.read().unwrap();

    let items: Vec<ListItem> = conversations
        .list()
        .iter()
        .enumerate()
        .map(|(i, conv)| {
            let is_selected = conversations.selected_index() == Some(i);

            // Icon based on conversation type
            let icon = match conv.conversation_type {
                ConversationType::DirectMessage => "👤",
                ConversationType::Group => "👥",
            };

            // Unread indicator
            let unread = if conv.unread_count > 0 {
                format!(" ({})", conv.unread_count)
            } else {
                String::new()
            };

            // Build the display line
            let name = conv.short_name(20);
            let content = format!("{} {}{}", icon, name, unread);

            let style = if is_selected && state.focus == crate::app::Focus::Sidebar {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![Span::styled(content, style)]))
        })
        .collect();

    let border_color = if state.focus == crate::app::Focus::Sidebar {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Conversations ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default());

    // Create list state for highlighting
    let mut list_state = ListState::default();
    list_state.select(conversations.selected_index());

    frame.render_stateful_widget(list, area, &mut list_state);
}
