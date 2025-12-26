use alloy::primitives::Address;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID (from blockchain)
    pub id: [u8; 32],
    /// Sender's wallet address
    pub sender: Address,
    /// Decrypted message content
    pub content: String,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Whether this message was sent by us
    pub is_ours: bool,
    /// Whether the message is pending (not yet on blockchain)
    pub is_pending: bool,
    /// Block number where this message was recorded
    pub block_number: Option<u64>,
    /// Transaction hash
    pub tx_hash: Option<[u8; 32]>,
}

impl Message {
    /// Create a new message
    pub fn new(
        id: [u8; 32],
        sender: Address,
        content: String,
        timestamp: i64,
        is_ours: bool,
    ) -> Self {
        Self {
            id,
            sender,
            content,
            timestamp: Utc.timestamp_opt(timestamp, 0).unwrap(),
            is_ours,
            is_pending: false,
            block_number: None,
            tx_hash: None,
        }
    }

    /// Create a pending outgoing message
    pub fn pending(sender: Address, content: String) -> Self {
        Self {
            id: [0u8; 32], // Will be set when confirmed
            sender,
            content,
            timestamp: Utc::now(),
            is_ours: true,
            is_pending: true,
            block_number: None,
            tx_hash: None,
        }
    }

    /// Get sender address as short string (0x1234...5678)
    pub fn sender_short(&self) -> String {
        let addr = format!("{:?}", self.sender);
        if addr.len() > 12 {
            format!("{}...{}", &addr[..6], &addr[addr.len() - 4..])
        } else {
            addr
        }
    }

    /// Get formatted time (HH:MM)
    pub fn time_short(&self) -> String {
        self.timestamp.format("%H:%M").to_string()
    }

    /// Get formatted date and time
    pub fn time_full(&self) -> String {
        self.timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// Check if message is from today
    pub fn is_today(&self) -> bool {
        let today = Utc::now().date_naive();
        self.timestamp.date_naive() == today
    }
}

/// Message content types for rich messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// Plain text message
    Text(String),
    /// System notification (e.g., "User joined")
    System(String),
    /// Group key update
    KeyUpdate { encrypted_key: Vec<u8> },
}

impl MessageContent {
    /// Serialize to bytes for encryption
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Try to parse as JSON first
        if let Ok(content) = serde_json::from_slice(bytes) {
            return Some(content);
        }
        // Fall back to plain text
        String::from_utf8(bytes.to_vec())
            .ok()
            .map(MessageContent::Text)
    }

    /// Get display text
    pub fn display(&self) -> &str {
        match self {
            MessageContent::Text(s) => s,
            MessageContent::System(s) => s,
            MessageContent::KeyUpdate { .. } => "[Key Update]",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_message_creation() {
        let sender = Address::from_str("0x0000000000000000000000000000000000000001").unwrap();
        let msg = Message::new([1u8; 32], sender, "Hello".to_string(), 1700000000, false);

        assert_eq!(msg.content, "Hello");
        assert!(!msg.is_ours);
        assert!(!msg.is_pending);
    }

    #[test]
    fn test_pending_message() {
        let sender = Address::from_str("0x0000000000000000000000000000000000000001").unwrap();
        let msg = Message::pending(sender, "Test".to_string());

        assert!(msg.is_pending);
        assert!(msg.is_ours);
    }

    #[test]
    fn test_message_content_serialization() {
        let content = MessageContent::Text("Hello world".to_string());
        let bytes = content.to_bytes();
        let parsed = MessageContent::from_bytes(&bytes).unwrap();

        match parsed {
            MessageContent::Text(s) => assert_eq!(s, "Hello world"),
            _ => panic!("Expected Text variant"),
        }
    }
}
