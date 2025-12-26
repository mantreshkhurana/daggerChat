use crate::errors::{DaggerError, Result};
use alloy::primitives::{Address, FixedBytes, U256};
use alloy::sol;
use std::sync::Arc;

use super::provider::HttpProvider;

// Define the contract ABI using sol! macro
sol! {
    #[sol(rpc)]
    contract DaggerChat {
        // Constants
        uint256 public constant MAX_MESSAGE_SIZE = 4096;
        uint256 public constant MAX_BATCH_SIZE = 50;

        // Structs
        struct EncryptedMessage {
            bytes32 id;
            address sender;
            uint64 timestamp;
            bytes encryptedContent;
            bytes32 conversationId;
        }

        struct UserProfile {
            bytes32 publicKeyX25519;
            bytes encryptedMetadata;
            uint64 registeredAt;
            bool exists;
        }

        // Events
        event UserRegistered(
            address indexed user,
            bytes32 publicKeyX25519,
            uint64 timestamp
        );

        event MessageSent(
            bytes32 indexed conversationId,
            bytes32 indexed messageId,
            address indexed sender,
            uint64 timestamp
        );

        event GroupCreated(
            bytes32 indexed groupId,
            address indexed admin,
            uint64 timestamp
        );

        event GroupMemberAdded(
            bytes32 indexed groupId,
            address indexed member,
            uint64 timestamp
        );

        // View functions
        function users(address user) external view returns (UserProfile memory);
        function messageCount(bytes32 conversationId) external view returns (uint256);
        function isUserRegistered(address user) external view returns (bool);
        function getUserPublicKey(address user) external view returns (bytes32);
        function isGroupMember(bytes32 groupId, address user) external view returns (bool);
        function getDirectConversationId(address user1, address user2) external pure returns (bytes32);
        function getMessages(bytes32 conversationId, uint256 offset, uint256 limit) external view returns (EncryptedMessage[] memory);

        // Write functions
        function registerUser(bytes32 publicKeyX25519, bytes calldata encryptedMetadata) external;
        function sendMessage(bytes32 conversationId, bytes calldata encryptedContent) external returns (bytes32 messageId);
        function sendMessageBatch(bytes32[] calldata conversationIds, bytes[] calldata encryptedContents) external;
        function createGroup(bytes calldata encryptedName, bytes calldata encryptedSymKey, address[] calldata initialMembers) external returns (bytes32 groupId);
        function addGroupMember(bytes32 groupId, address member, bytes calldata encryptedSymKeyForMember) external;
    }
}

/// Wrapper for interacting with the DaggerChat smart contract
pub struct DaggerChatContract {
    contract: DaggerChat::DaggerChatInstance<Http<Client>, Arc<HttpProvider>>,
    address: Address,
}

use alloy::transports::http::{Client, Http};

impl DaggerChatContract {
    /// Create a new contract instance
    pub fn new(address: Address, provider: Arc<HttpProvider>) -> Self {
        let contract = DaggerChat::new(address, provider);
        Self { contract, address }
    }

    /// Get the contract address
    pub fn address(&self) -> Address {
        self.address
    }

    /// Register a new user with their X25519 public key
    pub async fn register_user(&self, public_key: [u8; 32], metadata: Vec<u8>) -> Result<FixedBytes<32>> {
        let tx = self
            .contract
            .registerUser(FixedBytes::from(public_key), metadata.into())
            .send()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to send tx: {}", e)))?;

        let receipt = tx
            .get_receipt()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to get receipt: {}", e)))?;

        Ok(receipt.transaction_hash)
    }

    /// Check if a user is registered
    pub async fn is_user_registered(&self, address: Address) -> Result<bool> {
        self.contract
            .isUserRegistered(address)
            .call()
            .await
            .map(|r| r._0)
            .map_err(|e| DaggerError::Blockchain(format!("Failed to check user: {}", e)))
    }

    /// Get a user's X25519 public key
    pub async fn get_user_public_key(&self, address: Address) -> Result<[u8; 32]> {
        let result = self
            .contract
            .getUserPublicKey(address)
            .call()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to get public key: {}", e)))?;

        Ok(result._0.into())
    }

    /// Get conversation ID for direct messages between two users
    pub async fn get_dm_conversation_id(&self, user1: Address, user2: Address) -> Result<[u8; 32]> {
        let result = self
            .contract
            .getDirectConversationId(user1, user2)
            .call()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to get conversation ID: {}", e)))?;

        Ok(result._0.into())
    }

    /// Send an encrypted message
    pub async fn send_message(
        &self,
        conversation_id: [u8; 32],
        encrypted_content: Vec<u8>,
    ) -> Result<[u8; 32]> {
        let tx = self
            .contract
            .sendMessage(FixedBytes::from(conversation_id), encrypted_content.into())
            .send()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to send message: {}", e)))?;

        let receipt = tx
            .get_receipt()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to get receipt: {}", e)))?;

        // Parse message ID from logs
        for log in receipt.inner.logs() {
            if let Ok(event) = log.log_decode::<DaggerChat::MessageSent>() {
                return Ok(event.inner.messageId.into());
            }
        }

        Err(DaggerError::Blockchain("Message ID not found in logs".into()))
    }

    /// Send multiple messages in a batch
    pub async fn send_message_batch(
        &self,
        conversation_ids: Vec<[u8; 32]>,
        encrypted_contents: Vec<Vec<u8>>,
    ) -> Result<FixedBytes<32>> {
        let ids: Vec<FixedBytes<32>> = conversation_ids.into_iter().map(FixedBytes::from).collect();
        let contents: Vec<alloy::primitives::Bytes> = encrypted_contents.into_iter().map(Into::into).collect();

        let tx = self
            .contract
            .sendMessageBatch(ids, contents)
            .send()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to send batch: {}", e)))?;

        let receipt = tx
            .get_receipt()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to get receipt: {}", e)))?;

        Ok(receipt.transaction_hash)
    }

    /// Get messages from a conversation with pagination
    pub async fn get_messages(
        &self,
        conversation_id: [u8; 32],
        offset: u64,
        limit: u64,
    ) -> Result<Vec<DaggerChat::EncryptedMessage>> {
        let result = self
            .contract
            .getMessages(
                FixedBytes::from(conversation_id),
                U256::from(offset),
                U256::from(limit),
            )
            .call()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to get messages: {}", e)))?;

        Ok(result._0)
    }

    /// Get message count for a conversation
    pub async fn message_count(&self, conversation_id: [u8; 32]) -> Result<u64> {
        let result = self
            .contract
            .messageCount(FixedBytes::from(conversation_id))
            .call()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to get message count: {}", e)))?;

        Ok(result._0.try_into().unwrap_or(u64::MAX))
    }

    /// Create a new group chat
    pub async fn create_group(
        &self,
        encrypted_name: Vec<u8>,
        encrypted_sym_key: Vec<u8>,
        initial_members: Vec<Address>,
    ) -> Result<[u8; 32]> {
        let tx = self
            .contract
            .createGroup(encrypted_name.into(), encrypted_sym_key.into(), initial_members)
            .send()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to create group: {}", e)))?;

        let receipt = tx
            .get_receipt()
            .await
            .map_err(|e| DaggerError::Blockchain(format!("Failed to get receipt: {}", e)))?;

        // Parse group ID from logs
        for log in receipt.inner.logs() {
            if let Ok(event) = log.log_decode::<DaggerChat::GroupCreated>() {
                return Ok(event.inner.groupId.into());
            }
        }

        Err(DaggerError::Blockchain("Group ID not found in logs".into()))
    }

    /// Check if an address is a group member
    pub async fn is_group_member(&self, group_id: [u8; 32], user: Address) -> Result<bool> {
        self.contract
            .isGroupMember(FixedBytes::from(group_id), user)
            .call()
            .await
            .map(|r| r._0)
            .map_err(|e| DaggerError::Blockchain(format!("Failed to check membership: {}", e)))
    }
}
