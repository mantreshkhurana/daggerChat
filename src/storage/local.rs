use crate::errors::{DaggerError, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;

/// Local SQLite storage for caching messages and user data
pub struct LocalStorage {
    conn: Mutex<Connection>,
}

impl LocalStorage {
    /// Open or create the database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| DaggerError::Storage(format!("Failed to open database: {}", e)))?;

        let storage = Self {
            conn: Mutex::new(conn),
        };

        storage.initialize()?;
        Ok(storage)
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(include_str!("../../migrations/001_initial.sql"))
            .map_err(|e| DaggerError::Storage(format!("Failed to initialize database: {}", e)))?;

        Ok(())
    }

    // ========== User Operations ==========

    /// Save or update a user's public key
    pub fn save_user(&self, address: &str, public_key: &[u8; 32], registered_at: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO users (address, public_key_x25519, registered_at, last_synced_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![address, public_key.as_slice(), registered_at, chrono::Utc::now().timestamp()],
        )
        .map_err(|e| DaggerError::Storage(format!("Failed to save user: {}", e)))?;

        Ok(())
    }

    /// Get a user's public key
    pub fn get_user_public_key(&self, address: &str) -> Result<Option<[u8; 32]>> {
        let conn = self.conn.lock().unwrap();

        let result: Option<Vec<u8>> = conn
            .query_row(
                "SELECT public_key_x25519 FROM users WHERE address = ?1",
                params![address],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| DaggerError::Storage(format!("Failed to get user: {}", e)))?;

        match result {
            Some(bytes) if bytes.len() == 32 => {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                Ok(Some(key))
            }
            _ => Ok(None),
        }
    }

    // ========== Conversation Operations ==========

    /// Save or update a conversation
    pub fn save_conversation(
        &self,
        id: &[u8; 32],
        conversation_type: &str,
        display_name: Option<&str>,
        created_at: i64,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR IGNORE INTO conversations (id, conversation_type, display_name, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id.as_slice(), conversation_type, display_name, created_at],
        )
        .map_err(|e| DaggerError::Storage(format!("Failed to save conversation: {}", e)))?;

        Ok(())
    }

    /// Get all conversations
    pub fn get_conversations(&self) -> Result<Vec<ConversationRecord>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, conversation_type, display_name, created_at, last_message_at
                 FROM conversations ORDER BY last_message_at DESC NULLS LAST",
            )
            .map_err(|e| DaggerError::Storage(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                let id_bytes: Vec<u8> = row.get(0)?;
                let mut id = [0u8; 32];
                if id_bytes.len() == 32 {
                    id.copy_from_slice(&id_bytes);
                }

                Ok(ConversationRecord {
                    id,
                    conversation_type: row.get(1)?,
                    display_name: row.get(2)?,
                    created_at: row.get(3)?,
                    last_message_at: row.get(4)?,
                })
            })
            .map_err(|e| DaggerError::Storage(format!("Failed to query conversations: {}", e)))?;

        let mut conversations = Vec::new();
        for row in rows {
            conversations.push(
                row.map_err(|e| DaggerError::Storage(format!("Failed to read row: {}", e)))?,
            );
        }

        Ok(conversations)
    }

    /// Update last message time for a conversation
    pub fn update_conversation_last_message(&self, id: &[u8; 32], timestamp: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE conversations SET last_message_at = ?1 WHERE id = ?2",
            params![timestamp, id.as_slice()],
        )
        .map_err(|e| DaggerError::Storage(format!("Failed to update conversation: {}", e)))?;

        Ok(())
    }

    // ========== Message Operations ==========

    /// Save a message
    pub fn save_message(&self, msg: &MessageRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO messages
             (id, conversation_id, sender_address, content, encrypted_content, timestamp, tx_hash, block_number, is_pending)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                msg.id.as_slice(),
                msg.conversation_id.as_slice(),
                msg.sender_address,
                msg.content,
                msg.encrypted_content,
                msg.timestamp,
                msg.tx_hash,
                msg.block_number,
                msg.is_pending as i32,
            ],
        )
        .map_err(|e| DaggerError::Storage(format!("Failed to save message: {}", e)))?;

        // Update conversation last message time
        self.update_conversation_last_message(&msg.conversation_id, msg.timestamp)?;

        Ok(())
    }

    /// Get messages for a conversation
    pub fn get_messages(&self, conversation_id: &[u8; 32], limit: u32) -> Result<Vec<MessageRecord>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, conversation_id, sender_address, content, encrypted_content,
                        timestamp, tx_hash, block_number, is_pending
                 FROM messages
                 WHERE conversation_id = ?1
                 ORDER BY timestamp DESC
                 LIMIT ?2",
            )
            .map_err(|e| DaggerError::Storage(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map(params![conversation_id.as_slice(), limit], |row| {
                let id_bytes: Vec<u8> = row.get(0)?;
                let conv_id_bytes: Vec<u8> = row.get(1)?;

                let mut id = [0u8; 32];
                let mut conv_id = [0u8; 32];
                if id_bytes.len() == 32 {
                    id.copy_from_slice(&id_bytes);
                }
                if conv_id_bytes.len() == 32 {
                    conv_id.copy_from_slice(&conv_id_bytes);
                }

                Ok(MessageRecord {
                    id,
                    conversation_id: conv_id,
                    sender_address: row.get(2)?,
                    content: row.get(3)?,
                    encrypted_content: row.get(4)?,
                    timestamp: row.get(5)?,
                    tx_hash: row.get(6)?,
                    block_number: row.get(7)?,
                    is_pending: row.get::<_, i32>(8)? != 0,
                })
            })
            .map_err(|e| DaggerError::Storage(format!("Failed to query messages: {}", e)))?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(
                row.map_err(|e| DaggerError::Storage(format!("Failed to read row: {}", e)))?,
            );
        }

        // Reverse to get chronological order
        messages.reverse();
        Ok(messages)
    }

    // ========== Sync State Operations ==========

    /// Get the last synced block number
    pub fn get_last_synced_block(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();

        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM sync_state WHERE key = 'last_block'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| DaggerError::Storage(format!("Failed to get sync state: {}", e)))?;

        match result {
            Some(s) => s
                .parse()
                .map_err(|_| DaggerError::Storage("Invalid block number".into())),
            None => Ok(0),
        }
    }

    /// Set the last synced block number
    pub fn set_last_synced_block(&self, block: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO sync_state (key, value) VALUES ('last_block', ?1)",
            params![block.to_string()],
        )
        .map_err(|e| DaggerError::Storage(format!("Failed to set sync state: {}", e)))?;

        Ok(())
    }

    // ========== Pending Messages ==========

    /// Add a pending outgoing message
    pub fn add_pending_message(&self, conversation_id: &[u8; 32], encrypted_content: &[u8]) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO pending_messages (conversation_id, encrypted_content, created_at)
             VALUES (?1, ?2, ?3)",
            params![
                conversation_id.as_slice(),
                encrypted_content,
                chrono::Utc::now().timestamp()
            ],
        )
        .map_err(|e| DaggerError::Storage(format!("Failed to add pending message: {}", e)))?;

        Ok(conn.last_insert_rowid())
    }

    /// Get all pending messages
    pub fn get_pending_messages(&self) -> Result<Vec<PendingMessageRecord>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, conversation_id, encrypted_content, created_at, retry_count
                 FROM pending_messages ORDER BY created_at",
            )
            .map_err(|e| DaggerError::Storage(format!("Failed to prepare query: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                let conv_id_bytes: Vec<u8> = row.get(1)?;
                let mut conv_id = [0u8; 32];
                if conv_id_bytes.len() == 32 {
                    conv_id.copy_from_slice(&conv_id_bytes);
                }

                Ok(PendingMessageRecord {
                    id: row.get(0)?,
                    conversation_id: conv_id,
                    encrypted_content: row.get(2)?,
                    created_at: row.get(3)?,
                    retry_count: row.get(4)?,
                })
            })
            .map_err(|e| DaggerError::Storage(format!("Failed to query pending: {}", e)))?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(
                row.map_err(|e| DaggerError::Storage(format!("Failed to read row: {}", e)))?,
            );
        }

        Ok(messages)
    }

    /// Remove a pending message (after successful send)
    pub fn remove_pending_message(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM pending_messages WHERE id = ?1", params![id])
            .map_err(|e| DaggerError::Storage(format!("Failed to remove pending: {}", e)))?;

        Ok(())
    }
}

/// Record for a conversation
#[derive(Debug, Clone)]
pub struct ConversationRecord {
    pub id: [u8; 32],
    pub conversation_type: String,
    pub display_name: Option<String>,
    pub created_at: i64,
    pub last_message_at: Option<i64>,
}

/// Record for a message
#[derive(Debug, Clone)]
pub struct MessageRecord {
    pub id: [u8; 32],
    pub conversation_id: [u8; 32],
    pub sender_address: String,
    pub content: String,
    pub encrypted_content: Vec<u8>,
    pub timestamp: i64,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub is_pending: bool,
}

/// Record for a pending outgoing message
#[derive(Debug, Clone)]
pub struct PendingMessageRecord {
    pub id: i64,
    pub conversation_id: [u8; 32],
    pub encrypted_content: Vec<u8>,
    pub created_at: i64,
    pub retry_count: i32,
}
