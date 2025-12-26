use crate::app::App;
use crate::errors::Result;
use crate::tui::event::{poll_event, InputEvent, InputMode};
use crate::tui::ui;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;
use std::time::Duration;

/// Run the TUI application main loop
pub async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Draw the UI
        terminal
            .draw(|frame| ui::render(frame, app))
            .map_err(|e| crate::errors::DaggerError::Io(e))?;

        // Handle input
        let event = poll_event(Duration::from_millis(100));

        if handle_event(app, event).await? {
            break;
        }

        // Periodic sync (every 30 seconds)
        app.maybe_sync().await;
    }

    Ok(())
}

/// Handle an input event. Returns true if the app should quit.
async fn handle_event(app: &mut App, event: InputEvent) -> Result<bool> {
    let mut state = app.state.write().unwrap();

    // Handle events based on input mode
    if state.input_mode.is_editing() {
        match event {
            InputEvent::Quit => {
                // In editing mode, Ctrl+Q/C still quits
                return Ok(true);
            }
            InputEvent::Back => {
                // Cancel editing
                state.input_buffer.clear();
                state.input_mode = InputMode::Normal;
                if state.current_view == crate::app::View::NewConversation {
                    state.current_view = crate::app::View::Welcome;
                }
            }
            InputEvent::Enter => {
                let input = state.input_buffer.clone();
                let mode = state.input_mode;
                state.input_buffer.clear();
                state.input_mode = InputMode::Normal;
                drop(state);

                match mode {
                    InputMode::Typing => {
                        if !input.is_empty() {
                            app.send_message(&input).await?;
                        }
                    }
                    InputMode::EnteringAddress => {
                        if !input.is_empty() {
                            app.start_conversation(&input).await?;
                        }
                    }
                    _ => {}
                }
            }
            InputEvent::Backspace => {
                state.input_buffer.pop();
            }
            InputEvent::Char(c) => {
                state.input_buffer.push(c);
            }
            _ => {}
        }
    } else {
        // Normal mode
        match event {
            InputEvent::Quit => {
                return Ok(true);
            }
            InputEvent::Up => {
                drop(state);
                app.conversations.write().unwrap().select_previous();
            }
            InputEvent::Down => {
                drop(state);
                app.conversations.write().unwrap().select_next();
            }
            InputEvent::Enter => {
                if state.focus == crate::app::Focus::Sidebar {
                    // Select conversation and switch to chat
                    if app.conversations.read().unwrap().selected().is_some() {
                        state.current_view = crate::app::View::Chat;
                        state.focus = crate::app::Focus::Chat;
                    }
                } else if state.focus == crate::app::Focus::Chat {
                    // Start typing
                    state.input_mode = InputMode::Typing;
                }
            }
            InputEvent::Tab => {
                state.focus = match state.focus {
                    crate::app::Focus::Sidebar => crate::app::Focus::Chat,
                    crate::app::Focus::Chat => crate::app::Focus::Sidebar,
                };
            }
            InputEvent::Back => {
                match state.current_view {
                    crate::app::View::Chat => {
                        state.current_view = crate::app::View::Welcome;
                        state.focus = crate::app::Focus::Sidebar;
                    }
                    crate::app::View::NewConversation | crate::app::View::WalletInfo => {
                        state.current_view = crate::app::View::Welcome;
                    }
                    _ => {}
                }
            }
            InputEvent::NewConversation => {
                state.current_view = crate::app::View::NewConversation;
                state.input_mode = InputMode::EnteringAddress;
            }
            InputEvent::WalletInfo => {
                state.current_view = crate::app::View::WalletInfo;
            }
            InputEvent::Refresh => {
                drop(state);
                app.sync().await?;
            }
            _ => {}
        }
    }

    Ok(false)
}
