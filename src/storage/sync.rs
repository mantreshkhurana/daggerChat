use crate::blockchain::{contracts::DaggerChat, events::EventListener, DaggerChatContract};
use crate::crypto::EncryptionService;
use crate::errors::{DaggerError, Result};
use crate::storage::local::{LocalStorage, MessageRecord};
use alloy::primitives::Address;
use std::sync::Arc;
use x25519_dalek::PublicKey;

/// Manages synchronization between blockchain and local storage
pub struct SyncManager {
    storage: Arc<LocalStorage>,
    contract: Arc<DaggerChatContract>,
    event_listener: EventListener,
    our_address: Address,
}

impl SyncManager {
    pub fn new(
        storage: Arc<LocalStorage>,
        contract: Arc<DaggerChatContract>,
        event_listener: EventListener,
        our_address: Address,
    ) -> Self {
        Self {
            storage,
            contract,
            event_listener,
            our_address,
        }
    }

    /// Sync messages from blockchain to local storage
    pub async fn sync(&self, encryption: &EncryptionService) -> Result<u64> {
        let last_block = self.storage.get_last_synced_block()?;
        let current_block = self
            .contract
            .address()
            .to_string()
            .parse::<u64>()
            .unwrap_or(0);

        // This is a placeholder - in production, get actual block from provider
        // For now, we'll sync from events

        tracing::info!("Syncing from block {} to latest", last_block);

        // Get message events since last sync
        // Note: In production, you'd want to batch this for large ranges
        let events = self
            .event_listener
            .get_message_events(last_block + 1, current_block)
            .await?;

        let mut synced_count = 0;

        for event in events {
            // Check if this message is for a conversation we're part of
            if self.is_conversation_relevant(&event.conversation_id).await? {
                // Fetch the full message content from contract
                let messages = self
                    .contract
                    .get_messages(event.conversation_id, 0, 1000)
                    .await?;

                for msg in messages {
                    if msg.id == alloy::primitives::FixedBytes::from(event.message_id) {
                        // Try to decrypt the message
                        if let Ok(decrypted) = self
                            .decrypt_message(&msg, event.sender, encryption)
                            .await
                        {
                            let record = MessageRecord {
                                id: event.message_id,
                                conversation_id: event.conversation_id,
                                sender_address: format!("{:?}", event.sender),
                                content: decrypted,
                                encrypted_content: msg.encryptedContent.to_vec(),
                                timestamp: event.timestamp as i64,
                                tx_hash: Some(hex::encode(event.tx_hash)),
                                block_number: Some(event.block_number as i64),
                                is_pending: false,
                            };

                            self.storage.save_message(&record)?;
                            synced_count += 1;
                        }
                    }
                }
            }
        }

        // Update last synced block
        if current_block > last_block {
            self.storage.set_last_synced_block(current_block)?;
        }

        tracing::info!("Synced {} new messages", synced_count);
        Ok(synced_count)
    }

    /// Check if a conversation involves our address
    async fn is_conversation_relevant(&self, _conversation_id: &[u8; 32]) -> Result<bool> {
        // For DMs, check if we're one of the participants
        // For groups, check membership
        // This is simplified - in production, maintain a list of our conversations
        Ok(true)
    }

    /// Decrypt a message from the blockchain
    async fn decrypt_message(
        &self,
        msg: &DaggerChat::EncryptedMessage,
        sender: Address,
        encryption: &EncryptionService,
    ) -> Result<String> {
        // Get sender's public key
        let sender_public_key = self.contract.get_user_public_key(sender).await?;

        // Check local cache first
        let sender_addr = format!("{:?}", sender);
        if let Some(cached_key) = self.storage.get_user_public_key(&sender_addr)? {
            if cached_key != sender_public_key {
                // Update cached key if different
                self.storage.save_user(&sender_addr, &sender_public_key, 0)?;
            }
        } else {
            // Cache the key
            self.storage.save_user(&sender_addr, &sender_public_key, 0)?;
        }

        // Decrypt the message
        let public_key = PublicKey::from(sender_public_key);
        let decrypted = encryption.decrypt_from_sender(&msg.encryptedContent, &public_key)?;

        String::from_utf8(decrypted)
            .map_err(|_| DaggerError::Decryption("Invalid UTF-8 in message".into()))
    }

    /// Send pending messages to blockchain
    pub async fn flush_pending(&self) -> Result<u64> {
        let pending = self.storage.get_pending_messages()?;
        let mut sent_count = 0;

        for msg in pending {
            match self
                .contract
                .send_message(msg.conversation_id, msg.encrypted_content.clone())
                .await
            {
                Ok(_message_id) => {
                    self.storage.remove_pending_message(msg.id)?;
                    sent_count += 1;
                    tracing::info!("Sent pending message {}", msg.id);
                }
                Err(e) => {
                    tracing::warn!("Failed to send pending message {}: {}", msg.id, e);
                    // Could increment retry count here
                }
            }
        }

        Ok(sent_count)
    }
}
