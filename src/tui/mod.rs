pub mod app;
pub mod components;
pub mod event;
pub mod ui;

use crate::app::App;
use crate::errors::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, stdout};

/// Run the TUI application
pub async fn run(app: &mut App) -> Result<()> {
    // Setup terminal
    enable_raw_mode().map_err(|e| crate::errors::DaggerError::Io(e))?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .map_err(|e| crate::errors::DaggerError::Io(e))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| crate::errors::DaggerError::Io(e))?;

    // Run the app
    let result = app::run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode().map_err(|e| crate::errors::DaggerError::Io(e))?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )
    .map_err(|e| crate::errors::DaggerError::Io(e))?;
    terminal
        .show_cursor()
        .map_err(|e| crate::errors::DaggerError::Io(e))?;

    result
}
