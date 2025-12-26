use crate::app::App;
use crate::tui::event::InputMode;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the message input box
pub fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let state = app.state.read().unwrap();

    let (border_color, placeholder) = match state.input_mode {
        InputMode::Typing => (Color::Yellow, ""),
        _ => (Color::DarkGray, "Press Enter to type a message..."),
    };

    let display_text = if state.input_buffer.is_empty() {
        placeholder.to_string()
    } else {
        state.input_buffer.clone()
    };

    let input_style = if state.input_mode == InputMode::Typing {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let input = Paragraph::new(format!(" {}", display_text))
        .style(input_style)
        .block(
            Block::default()
                .title(" Message ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );

    frame.render_widget(input, area);

    // Show cursor when typing
    if state.input_mode == InputMode::Typing {
        frame.set_cursor_position((
            area.x + state.input_buffer.len() as u16 + 2,
            area.y + 1,
        ));
    }
}
