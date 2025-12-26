use crate::crypto::EncryptionService;
use crate::errors::Result;
use alloy::primitives::Address;
use std::collections::HashSet;

/// Represents a group chat with its encryption key and members
#[derive(Debug, Clone)]
pub struct GroupChat {
    /// Group ID (from blockchain)
    pub id: [u8; 32],
    /// Encrypted group name (stored on chain)
    pub encrypted_name: Vec<u8>,
    /// Decrypted group name (local)
    pub name: String,
    /// Group admin address
    pub admin: Address,
    /// Group members
    pub members: HashSet<Address>,
    /// Symmetric encryption key for this group
    pub symmetric_key: [u8; 32],
    /// Creation timestamp
    pub created_at: i64,
}

impl GroupChat {
    /// Create a new group chat (as admin)
    pub fn create(name: &str, admin: Address, members: Vec<Address>) -> Self {
        let symmetric_key = EncryptionService::generate_group_key();

        let mut member_set = HashSet::new();
        member_set.insert(admin);
        for member in members {
            member_set.insert(member);
        }

        Self {
            id: [0u8; 32], // Will be set after blockchain confirmation
            encrypted_name: Vec::new(), // Will be set when encrypting
            name: name.to_string(),
            admin,
            members: member_set,
            symmetric_key,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Encrypt the group name with the symmetric key
    pub fn encrypt_name(&mut self) -> Result<()> {
        self.encrypted_name =
            EncryptionService::encrypt_symmetric(self.name.as_bytes(), &self.symmetric_key)?;
        Ok(())
    }

    /// Decrypt the group name with the symmetric key
    pub fn decrypt_name(encrypted: &[u8], key: &[u8; 32]) -> Result<String> {
        let decrypted = EncryptionService::decrypt_symmetric(encrypted, key)?;
        String::from_utf8(decrypted)
            .map_err(|_| crate::errors::DaggerError::Decryption("Invalid UTF-8 in group name".into()))
    }

    /// Encrypt the symmetric key for a specific member
    pub fn encrypt_key_for_member(
        &self,
        encryption: &EncryptionService,
        member_public: &x25519_dalek::PublicKey,
    ) -> Result<Vec<u8>> {
        encryption.encrypt_group_key_for_member(&self.symmetric_key, member_public)
    }

    /// Encrypt a message for the group
    pub fn encrypt_message(&self, content: &str) -> Result<Vec<u8>> {
        EncryptionService::encrypt_symmetric(content.as_bytes(), &self.symmetric_key)
    }

    /// Decrypt a message from the group
    pub fn decrypt_message(&self, encrypted: &[u8]) -> Result<String> {
        let decrypted = EncryptionService::decrypt_symmetric(encrypted, &self.symmetric_key)?;
        String::from_utf8(decrypted)
            .map_err(|_| crate::errors::DaggerError::Decryption("Invalid UTF-8 in message".into()))
    }

    /// Add a member to the group
    pub fn add_member(&mut self, member: Address) {
        self.members.insert(member);
    }

    /// Remove a member from the group
    pub fn remove_member(&mut self, member: &Address) {
        self.members.remove(member);
    }

    /// Check if an address is a member
    pub fn is_member(&self, address: &Address) -> bool {
        self.members.contains(address)
    }

    /// Check if an address is the admin
    pub fn is_admin(&self, address: &Address) -> bool {
        self.admin == *address
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get group ID as hex string
    pub fn id_hex(&self) -> String {
        hex::encode(self.id)
    }
}

/// Manages multiple group chats
pub struct GroupManager {
    groups: std::collections::HashMap<[u8; 32], GroupChat>,
}

impl GroupManager {
    pub fn new() -> Self {
        Self {
            groups: std::collections::HashMap::new(),
        }
    }

    /// Add a group
    pub fn add(&mut self, group: GroupChat) {
        self.groups.insert(group.id, group);
    }

    /// Get a group by ID
    pub fn get(&self, id: &[u8; 32]) -> Option<&GroupChat> {
        self.groups.get(id)
    }

    /// Get a mutable group by ID
    pub fn get_mut(&mut self, id: &[u8; 32]) -> Option<&mut GroupChat> {
        self.groups.get_mut(id)
    }

    /// List all groups
    pub fn list(&self) -> Vec<&GroupChat> {
        self.groups.values().collect()
    }
}

impl Default for GroupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_group_creation() {
        let admin = Address::from_str("0x0000000000000000000000000000000000000001").unwrap();
        let member = Address::from_str("0x0000000000000000000000000000000000000002").unwrap();

        let group = GroupChat::create("Test Group", admin, vec![member]);

        assert_eq!(group.name, "Test Group");
        assert!(group.is_admin(&admin));
        assert!(group.is_member(&member));
        assert!(group.is_member(&admin));
        assert_eq!(group.member_count(), 2);
    }

    #[test]
    fn test_group_encryption() {
        let admin = Address::from_str("0x0000000000000000000000000000000000000001").unwrap();
        let mut group = GroupChat::create("Secret Group", admin, vec![]);

        // Encrypt name
        group.encrypt_name().unwrap();
        assert!(!group.encrypted_name.is_empty());

        // Decrypt name
        let decrypted = GroupChat::decrypt_name(&group.encrypted_name, &group.symmetric_key).unwrap();
        assert_eq!(decrypted, "Secret Group");
    }

    #[test]
    fn test_message_encryption() {
        let admin = Address::from_str("0x0000000000000000000000000000000000000001").unwrap();
        let group = GroupChat::create("Test", admin, vec![]);

        let message = "Hello, group!";
        let encrypted = group.encrypt_message(message).unwrap();
        let decrypted = group.decrypt_message(&encrypted).unwrap();

        assert_eq!(decrypted, message);
    }
}
