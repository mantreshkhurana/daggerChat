use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Input events from the terminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    /// Quit the application
    Quit,
    /// Move selection up
    Up,
    /// Move selection down
    Down,
    /// Select / Enter
    Enter,
    /// Go back / Cancel
    Back,
    /// Switch between panes
    Tab,
    /// Start new conversation
    NewConversation,
    /// Create new group
    NewGroup,
    /// Show wallet info
    WalletInfo,
    /// Delete character
    Backspace,
    /// Character input
    Char(char),
    /// Refresh / Sync
    Refresh,
    /// No event (timeout)
    None,
}

/// Poll for input events with a timeout
pub fn poll_event(timeout: Duration) -> InputEvent {
    if event::poll(timeout).unwrap_or(false) {
        if let Ok(Event::Key(key)) = event::read() {
            return map_key_event(key);
        }
    }
    InputEvent::None
}

/// Map a key event to an input event
fn map_key_event(key: KeyEvent) -> InputEvent {
    // Check for Ctrl combinations first
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => InputEvent::Quit,
            KeyCode::Char('n') => InputEvent::NewConversation,
            KeyCode::Char('g') => InputEvent::NewGroup,
            KeyCode::Char('w') => InputEvent::WalletInfo,
            KeyCode::Char('r') => InputEvent::Refresh,
            _ => InputEvent::None,
        };
    }

    match key.code {
        KeyCode::Char('q') => InputEvent::Quit,
        KeyCode::Up | KeyCode::Char('k') => InputEvent::Up,
        KeyCode::Down | KeyCode::Char('j') => InputEvent::Down,
        KeyCode::Enter => InputEvent::Enter,
        KeyCode::Esc => InputEvent::Back,
        KeyCode::Tab => InputEvent::Tab,
        KeyCode::Char('n') => InputEvent::NewConversation,
        KeyCode::Char('g') => InputEvent::NewGroup,
        KeyCode::Backspace => InputEvent::Backspace,
        KeyCode::Char(c) => InputEvent::Char(c),
        _ => InputEvent::None,
    }
}

/// Current input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal navigation mode
    #[default]
    Normal,
    /// Typing a message
    Typing,
    /// Entering an address for new conversation
    EnteringAddress,
    /// Entering group name
    EnteringGroupName,
}

impl InputMode {
    /// Check if we're in a text input mode
    pub fn is_editing(&self) -> bool {
        matches!(
            self,
            InputMode::Typing | InputMode::EnteringAddress | InputMode::EnteringGroupName
        )
    }
}
