use crate::app::App;
use crate::tui::components::{render_chat_list, render_input, render_messages, render_status_bar};
use crate::tui::event::InputMode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the main UI
pub fn render(frame: &mut Frame, app: &App) {
    // Main horizontal layout: sidebar + content
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(32), // Sidebar
            Constraint::Min(50),    // Content
        ])
        .split(frame.area());

    // Render sidebar
    render_sidebar(frame, app, main_chunks[0]);

    // Render content area
    render_content(frame, app, main_chunks[1]);
}

/// Render the sidebar (conversation list)
fn render_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Conversation list
            Constraint::Length(3), // Status bar
        ])
        .split(area);

    // Header
    let header = Paragraph::new(" daggerChat")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(header, chunks[0]);

    // Conversation list
    render_chat_list(frame, app, chunks[1]);

    // Status bar
    render_status_bar(frame, app, chunks[2]);
}

/// Render the main content area
fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    let state = app.state.read().unwrap();

    match state.current_view {
        crate::app::View::Chat => {
            drop(state);
            render_chat_view(frame, app, area);
        }
        crate::app::View::Welcome => {
            render_welcome(frame, area);
        }
        crate::app::View::NewConversation => {
            drop(state);
            render_new_conversation(frame, app, area);
        }
        crate::app::View::WalletInfo => {
            drop(state);
            render_wallet_info(frame, app, area);
        }
    }
}

/// Render the chat view with messages and input
fn render_chat_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Chat header
            Constraint::Min(10),   // Messages
            Constraint::Length(3), // Input
        ])
        .split(area);

    // Chat header
    let state = app.state.read().unwrap();
    let header_text = if let Some(conv) = app.conversations.read().unwrap().selected() {
        format!(" Chat: {}", conv.display_name)
    } else {
        " Select a conversation".to_string()
    };
    drop(state);

    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(header, chunks[0]);

    // Messages
    render_messages(frame, app, chunks[1]);

    // Input
    render_input(frame, app, chunks[2]);
}

/// Render welcome screen
fn render_welcome(frame: &mut Frame, area: Rect) {
    let text = r#"
    Welcome to daggerChat!

    A blockchain-based secure chat application.

    Commands:
    • n     - Start new conversation
    • g     - Create new group
    • Tab   - Switch panes
    • Enter - Select / Send
    • q     - Quit

    Select a conversation from the sidebar
    or press 'n' to start a new one.
    "#;

    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .title(" Welcome ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(paragraph, area);
}

/// Render new conversation dialog
fn render_new_conversation(frame: &mut Frame, app: &App, area: Rect) {
    let state = app.state.read().unwrap();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(5),
        ])
        .margin(2)
        .split(area);

    let instructions = Paragraph::new(
        "Enter the wallet address of the person you want to chat with.\nPress Enter to start the conversation, Esc to cancel.",
    )
    .style(Style::default().fg(Color::Gray))
    .block(Block::default().borders(Borders::NONE));
    frame.render_widget(instructions, chunks[0]);

    let input_style = if state.input_mode == InputMode::EnteringAddress {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let input = Paragraph::new(format!(" {}", state.input_buffer))
        .style(input_style)
        .block(
            Block::default()
                .title(" Address ")
                .borders(Borders::ALL)
                .border_style(input_style),
        );
    frame.render_widget(input, chunks[1]);

    // Show cursor position
    if state.input_mode == InputMode::EnteringAddress {
        frame.set_cursor_position((
            chunks[1].x + state.input_buffer.len() as u16 + 2,
            chunks[1].y + 1,
        ));
    }
}

/// Render wallet info screen
fn render_wallet_info(frame: &mut Frame, app: &App, area: Rect) {
    let wallet_addr = app.wallet.address_string();
    let short_addr = app.wallet.short_address();
    let network = app.provider.network_name();

    let text = format!(
        r#"
    Wallet Information
    ──────────────────

    Address: {}

    Short:   {}

    Network: {}

    Press Esc to go back.
    "#,
        wallet_addr, short_addr, network
    );

    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title(" Wallet Info ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );

    frame.render_widget(paragraph, area);
}
