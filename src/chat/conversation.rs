use crate::chat::Message;
use crate::errors::Result;
use crate::storage::local::{ConversationRecord, LocalStorage, MessageRecord};
use alloy::primitives::Address;
use chrono::{TimeZone, Utc};
use std::collections::HashMap;
use std::sync::Arc;

/// Type of conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationType {
    DirectMessage,
    Group,
}

impl ConversationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversationType::DirectMessage => "dm",
            ConversationType::Group => "group",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "group" => ConversationType::Group,
            _ => ConversationType::DirectMessage,
        }
    }
}

/// A conversation (DM or group)
#[derive(Debug, Clone)]
pub struct Conversation {
    /// Unique conversation ID (derived from participants or group ID)
    pub id: [u8; 32],
    /// Type of conversation
    pub conversation_type: ConversationType,
    /// Display name (other user's address or group name)
    pub display_name: String,
    /// Messages in this conversation
    pub messages: Vec<Message>,
    /// For DMs: the other participant's address
    pub peer_address: Option<Address>,
    /// Creation timestamp
    pub created_at: i64,
    /// Last message timestamp
    pub last_message_at: Option<i64>,
    /// Unread message count
    pub unread_count: u32,
}

impl Conversation {
    /// Create a new DM conversation
    pub fn new_dm(id: [u8; 32], peer_address: Address, display_name: String) -> Self {
        Self {
            id,
            conversation_type: ConversationType::DirectMessage,
            display_name,
            messages: Vec::new(),
            peer_address: Some(peer_address),
            created_at: Utc::now().timestamp(),
            last_message_at: None,
            unread_count: 0,
        }
    }

    /// Create a new group conversation
    pub fn new_group(id: [u8; 32], name: String) -> Self {
        Self {
            id,
            conversation_type: ConversationType::Group,
            display_name: name,
            messages: Vec::new(),
            peer_address: None,
            created_at: Utc::now().timestamp(),
            last_message_at: None,
            unread_count: 0,
        }
    }

    /// Get conversation ID as hex string
    pub fn id_hex(&self) -> String {
        hex::encode(self.id)
    }

    /// Get short display name (truncated if too long)
    pub fn short_name(&self, max_len: usize) -> String {
        if self.display_name.len() <= max_len {
            self.display_name.clone()
        } else {
            format!("{}...", &self.display_name[..max_len - 3])
        }
    }

    /// Add a message to this conversation
    pub fn add_message(&mut self, message: Message) {
        let timestamp = message.timestamp.timestamp();
        self.messages.push(message);
        self.last_message_at = Some(timestamp);
    }

    /// Get the last message preview
    pub fn last_message_preview(&self) -> Option<String> {
        self.messages.last().map(|m| {
            let content = if m.content.len() > 30 {
                format!("{}...", &m.content[..27])
            } else {
                m.content.clone()
            };
            if m.is_ours {
                format!("You: {}", content)
            } else {
                content
            }
        })
    }
}

/// Manages all conversations
pub struct ConversationManager {
    conversations: HashMap<[u8; 32], Conversation>,
    storage: Arc<LocalStorage>,
    our_address: Address,
    selected_index: Option<usize>,
}

impl ConversationManager {
    pub fn new(storage: Arc<LocalStorage>, our_address: Address) -> Self {
        Self {
            conversations: HashMap::new(),
            storage,
            our_address,
            selected_index: None,
        }
    }

    /// Load conversations from local storage
    pub fn load_from_storage(&mut self) -> Result<()> {
        let records = self.storage.get_conversations()?;

        for record in records {
            let conversation = self.record_to_conversation(record);
            self.conversations.insert(conversation.id, conversation);
        }

        // Load messages for each conversation
        let conv_ids: Vec<[u8; 32]> = self.conversations.keys().cloned().collect();
        for conv_id in conv_ids {
            let messages = self.storage.get_messages(&conv_id, 100)?;
            for msg_record in messages {
                let is_ours = msg_record.sender_address == format!("{:?}", self.our_address);
                let message = self.record_to_message(msg_record, is_ours);
                if let Some(conv) = self.conversations.get_mut(&conv_id) {
                    conv.messages.push(message);
                }
            }
        }

        Ok(())
    }

    /// Get or create a DM conversation with an address
    pub fn get_or_create_dm(&mut self, id: [u8; 32], peer: Address) -> &mut Conversation {
        self.conversations.entry(id).or_insert_with(|| {
            let display_name = format!("{:?}", peer);
            let short = if display_name.len() > 12 {
                format!(
                    "{}...{}",
                    &display_name[..6],
                    &display_name[display_name.len() - 4..]
                )
            } else {
                display_name
            };

            // Save to storage
            let _ = self
                .storage
                .save_conversation(&id, "dm", Some(&short), Utc::now().timestamp());

            Conversation::new_dm(id, peer, short)
        })
    }

    /// Get a conversation by ID
    pub fn get(&self, id: &[u8; 32]) -> Option<&Conversation> {
        self.conversations.get(id)
    }

    /// Get a mutable conversation by ID
    pub fn get_mut(&mut self, id: &[u8; 32]) -> Option<&mut Conversation> {
        self.conversations.get_mut(id)
    }

    /// Get all conversations sorted by last message time
    pub fn list(&self) -> Vec<&Conversation> {
        let mut convs: Vec<_> = self.conversations.values().collect();
        convs.sort_by(|a, b| b.last_message_at.cmp(&a.last_message_at));
        convs
    }

    /// Get conversation count
    pub fn count(&self) -> usize {
        self.conversations.len()
    }

    /// Select a conversation by index
    pub fn select(&mut self, index: usize) {
        if index < self.count() {
            self.selected_index = Some(index);
        }
    }

    /// Get selected conversation
    pub fn selected(&self) -> Option<&Conversation> {
        self.selected_index.and_then(|i| self.list().get(i).copied())
    }

    /// Get selected conversation ID
    pub fn selected_id(&self) -> Option<[u8; 32]> {
        self.selected().map(|c| c.id)
    }

    /// Get selected index
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if let Some(idx) = self.selected_index {
            if idx > 0 {
                self.selected_index = Some(idx - 1);
            }
        } else if self.count() > 0 {
            self.selected_index = Some(0);
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        let count = self.count();
        if let Some(idx) = self.selected_index {
            if idx + 1 < count {
                self.selected_index = Some(idx + 1);
            }
        } else if count > 0 {
            self.selected_index = Some(0);
        }
    }

    fn record_to_conversation(&self, record: ConversationRecord) -> Conversation {
        Conversation {
            id: record.id,
            conversation_type: ConversationType::from_str(&record.conversation_type),
            display_name: record.display_name.unwrap_or_else(|| "Unknown".to_string()),
            messages: Vec::new(),
            peer_address: None,
            created_at: record.created_at,
            last_message_at: record.last_message_at,
            unread_count: 0,
        }
    }

    fn record_to_message(&self, record: MessageRecord, is_ours: bool) -> Message {
        Message {
            id: record.id,
            sender: record
                .sender_address
                .parse()
                .unwrap_or(Address::ZERO),
            content: record.content,
            timestamp: Utc.timestamp_opt(record.timestamp, 0).unwrap(),
            is_ours,
            is_pending: record.is_pending,
            block_number: record.block_number.map(|b| b as u64),
            tx_hash: record
                .tx_hash
                .and_then(|h| {
                    let bytes = hex::decode(h).ok()?;
                    if bytes.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&bytes);
                        Some(arr)
                    } else {
                        None
                    }
                }),
        }
    }
}
