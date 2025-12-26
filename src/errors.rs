use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaggerError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Wallet error: {0}")]
    Wallet(String),

    #[error("Blockchain error: {0}")]
    Blockchain(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("User not registered: {0}")]
    UserNotRegistered(String),

    #[error("Conversation not found: {0}")]
    ConversationNotFound(String),

    #[error("Message too large: max {max} bytes, got {actual}")]
    MessageTooLarge { max: usize, actual: usize },

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DaggerError>;
