use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the status bar
pub fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let state = app.state.read().unwrap();

    let (status_icon, status_text, status_color) = if state.is_syncing {
        ("⟳", "Syncing...", Color::Yellow)
    } else if state.is_connected {
        ("●", "Connected", Color::Green)
    } else {
        ("○", "Offline", Color::Red)
    };

    let network = app.provider.network_name();
    let short_network = match network {
        "Arbitrum One" => "Arb",
        "Arbitrum Sepolia" => "Arb-Sep",
        "Base" => "Base",
        "Base Sepolia" => "Base-Sep",
        "Ethereum Mainnet" => "ETH",
        "Sepolia" => "Sep",
        _ => "??",
    };

    let line = Line::from(vec![
        Span::styled(format!(" {} ", status_icon), Style::default().fg(status_color)),
        Span::styled(status_text, Style::default().fg(status_color)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(short_network, Style::default().fg(Color::Cyan)),
    ]);

    let paragraph = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(paragraph, area);
}
