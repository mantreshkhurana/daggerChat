use crate::errors::{DaggerError, Result};
use alloy::primitives::{Address, FixedBytes};
use alloy::providers::Provider;
use alloy::rpc::types::{Filter, Log};
use alloy::sol_types::SolEvent;
use std::sync::Arc;

use super::contracts::DaggerChat;
use super::provider::HttpProvider;

/// Parsed message event from the blockchain
#[derive(Debug, Clone)]
pub struct MessageEvent {
    pub conversation_id: [u8; 32],
    pub message_id: [u8; 32],
    pub sender: Address,
    pub timestamp: u64,
    pub block_number: u64,
    pub tx_hash: [u8; 32],
}

/// Parsed user registration event
#[derive(Debug, Clone)]
pub struct UserRegisteredEvent {
    pub user: Address,
    pub public_key: [u8; 32],
    pub timestamp: u64,
    pub block_number: u64,
}

/// Parsed group created event
#[derive(Debug, Clone)]
pub struct GroupCreatedEvent {
    pub group_id: [u8; 32],
    pub admin: Address,
    pub timestamp: u64,
    pub block_number: u64,
}

/// Event listener for blockchain events
pub struct EventListener {
    provider: Arc<HttpProvider>,
    contract_address: Address,
}

impl EventListener {
    pub fn new(provider: Arc<HttpProvider>, contract_address: Address) -> Self {
        Self {
            provider,
            contract_address,
        }
    }

    /// Get message events from a range of blocks
    pub async fn get_message_events(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<MessageEvent>> {
        let filter = Filter::new()
            .address(self.contract_address)
            .event_signature(DaggerChat::MessageSent::SIGNATURE_HASH)
            .from_block(from_block)
            .to_block(to_block);

        let logs = self
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| DaggerError::Network(format!("Failed to get logs: {}", e)))?;

        let mut events = Vec::new();
        for log in logs {
            if let Some(event) = self.parse_message_event(&log) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Get message events for a specific conversation
    pub async fn get_conversation_events(
        &self,
        conversation_id: [u8; 32],
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<MessageEvent>> {
        let filter = Filter::new()
            .address(self.contract_address)
            .event_signature(DaggerChat::MessageSent::SIGNATURE_HASH)
            .topic1(FixedBytes::from(conversation_id))
            .from_block(from_block)
            .to_block(to_block);

        let logs = self
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| DaggerError::Network(format!("Failed to get logs: {}", e)))?;

        let mut events = Vec::new();
        for log in logs {
            if let Some(event) = self.parse_message_event(&log) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Get user registration events
    pub async fn get_user_registered_events(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<UserRegisteredEvent>> {
        let filter = Filter::new()
            .address(self.contract_address)
            .event_signature(DaggerChat::UserRegistered::SIGNATURE_HASH)
            .from_block(from_block)
            .to_block(to_block);

        let logs = self
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| DaggerError::Network(format!("Failed to get logs: {}", e)))?;

        let mut events = Vec::new();
        for log in logs {
            if let Some(event) = self.parse_user_registered_event(&log) {
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Get group created events
    pub async fn get_group_created_events(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<GroupCreatedEvent>> {
        let filter = Filter::new()
            .address(self.contract_address)
            .event_signature(DaggerChat::GroupCreated::SIGNATURE_HASH)
            .from_block(from_block)
            .to_block(to_block);

        let logs = self
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| DaggerError::Network(format!("Failed to get logs: {}", e)))?;

        let mut events = Vec::new();
        for log in logs {
            if let Some(event) = self.parse_group_created_event(&log) {
                events.push(event);
            }
        }

        Ok(events)
    }

    fn parse_message_event(&self, log: &Log) -> Option<MessageEvent> {
        let decoded = log.log_decode::<DaggerChat::MessageSent>().ok()?;
        let inner = decoded.inner;

        Some(MessageEvent {
            conversation_id: inner.conversationId.into(),
            message_id: inner.messageId.into(),
            sender: inner.sender,
            timestamp: inner.timestamp,
            block_number: log.block_number?,
            tx_hash: log.transaction_hash?.into(),
        })
    }

    fn parse_user_registered_event(&self, log: &Log) -> Option<UserRegisteredEvent> {
        let decoded = log.log_decode::<DaggerChat::UserRegistered>().ok()?;
        let inner = decoded.inner;

        Some(UserRegisteredEvent {
            user: inner.user,
            public_key: inner.publicKeyX25519.into(),
            timestamp: inner.timestamp,
            block_number: log.block_number?,
        })
    }

    fn parse_group_created_event(&self, log: &Log) -> Option<GroupCreatedEvent> {
        let decoded = log.log_decode::<DaggerChat::GroupCreated>().ok()?;
        let inner = decoded.inner;

        Some(GroupCreatedEvent {
            group_id: inner.groupId.into(),
            admin: inner.admin,
            timestamp: inner.timestamp,
            block_number: log.block_number?,
        })
    }
}
