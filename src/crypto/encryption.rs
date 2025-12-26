use crate::errors::{DaggerError, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;
use x25519_dalek::PublicKey;

use super::KeyManager;

/// Handles message encryption and decryption using XChaCha20-Poly1305
pub struct EncryptionService {
    key_manager: KeyManager,
}

impl EncryptionService {
    pub fn new(key_manager: KeyManager) -> Self {
        Self { key_manager }
    }

    /// Get our public key for sharing with others
    pub fn public_key(&self) -> &PublicKey {
        self.key_manager.public_key()
    }

    /// Get our public key as bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.key_manager.public_key_bytes()
    }

    /// Encrypt a message for a specific recipient using their public key
    /// Returns: nonce (24 bytes) || ciphertext
    pub fn encrypt_for_recipient(&self, plaintext: &[u8], recipient_public: &PublicKey) -> Result<Vec<u8>> {
        // Perform ECDH to get shared secret
        let shared_secret = self.key_manager.diffie_hellman(recipient_public);

        // Derive encryption key using HKDF
        let key = Self::derive_key(&shared_secret)?;

        // Encrypt with XChaCha20-Poly1305
        Self::encrypt_with_key(plaintext, &key)
    }

    /// Decrypt a message from a sender using their public key
    pub fn decrypt_from_sender(&self, ciphertext: &[u8], sender_public: &PublicKey) -> Result<Vec<u8>> {
        if ciphertext.len() < 24 {
            return Err(DaggerError::Decryption("Ciphertext too short".into()));
        }

        // Perform ECDH to get shared secret
        let shared_secret = self.key_manager.diffie_hellman(sender_public);

        // Derive decryption key using HKDF
        let key = Self::derive_key(&shared_secret)?;

        // Decrypt
        Self::decrypt_with_key(ciphertext, &key)
    }

    /// Encrypt with a symmetric key (for group chats)
    /// Returns: nonce (24 bytes) || ciphertext
    pub fn encrypt_symmetric(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
        Self::encrypt_with_key(plaintext, key)
    }

    /// Decrypt with a symmetric key (for group chats)
    pub fn decrypt_symmetric(ciphertext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
        Self::decrypt_with_key(ciphertext, key)
    }

    /// Generate a random symmetric key for group chats
    pub fn generate_group_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    }

    /// Encrypt the group key for a specific member
    pub fn encrypt_group_key_for_member(
        &self,
        group_key: &[u8; 32],
        member_public: &PublicKey,
    ) -> Result<Vec<u8>> {
        self.encrypt_for_recipient(group_key, member_public)
    }

    /// Decrypt the group key received from admin
    pub fn decrypt_group_key(&self, encrypted_key: &[u8], admin_public: &PublicKey) -> Result<[u8; 32]> {
        let decrypted = self.decrypt_from_sender(encrypted_key, admin_public)?;
        if decrypted.len() != 32 {
            return Err(DaggerError::Decryption("Invalid group key length".into()));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&decrypted);
        Ok(key)
    }

    // Internal helper: derive encryption key from shared secret
    fn derive_key(shared_secret: &[u8; 32]) -> Result<[u8; 32]> {
        let hk = Hkdf::<Sha256>::new(Some(b"dagger-chat-message"), shared_secret);
        let mut key = [0u8; 32];
        hk.expand(b"message-encryption-key", &mut key)
            .map_err(|_| DaggerError::Encryption("Key derivation failed".into()))?;
        Ok(key)
    }

    // Internal helper: encrypt with a key
    fn encrypt_with_key(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
        // Generate random nonce (24 bytes for XChaCha20)
        let mut nonce_bytes = [0u8; 24];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        // Create cipher and encrypt
        let cipher = XChaCha20Poly1305::new_from_slice(key)
            .map_err(|e| DaggerError::Encryption(e.to_string()))?;

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| DaggerError::Encryption(e.to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    // Internal helper: decrypt with a key
    fn decrypt_with_key(ciphertext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
        if ciphertext.len() < 24 {
            return Err(DaggerError::Decryption("Ciphertext too short".into()));
        }

        let nonce = XNonce::from_slice(&ciphertext[..24]);
        let encrypted = &ciphertext[24..];

        let cipher = XChaCha20Poly1305::new_from_slice(key)
            .map_err(|e| DaggerError::Decryption(e.to_string()))?;

        cipher
            .decrypt(nonce, encrypted)
            .map_err(|_| DaggerError::Decryption("Decryption failed - invalid key or corrupted data".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_dm() {
        let alice_keys = KeyManager::generate();
        let bob_keys = KeyManager::generate();

        let alice = EncryptionService::new(alice_keys);
        let bob = EncryptionService::new(bob_keys);

        let message = b"Hello, Bob! This is a secret message.";

        // Alice encrypts for Bob
        let encrypted = alice.encrypt_for_recipient(message, bob.public_key()).unwrap();

        // Bob decrypts from Alice
        let decrypted = bob.decrypt_from_sender(&encrypted, alice.public_key()).unwrap();

        assert_eq!(message.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_symmetric_encryption() {
        let key = EncryptionService::generate_group_key();
        let message = b"Group message for everyone!";

        let encrypted = EncryptionService::encrypt_symmetric(message, &key).unwrap();
        let decrypted = EncryptionService::decrypt_symmetric(&encrypted, &key).unwrap();

        assert_eq!(message.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = EncryptionService::generate_group_key();
        let key2 = EncryptionService::generate_group_key();

        let message = b"Secret message";
        let encrypted = EncryptionService::encrypt_symmetric(message, &key1).unwrap();

        let result = EncryptionService::decrypt_symmetric(&encrypted, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_group_key_exchange() {
        let admin_keys = KeyManager::generate();
        let member_keys = KeyManager::generate();

        let admin = EncryptionService::new(admin_keys);
        let member = EncryptionService::new(member_keys);

        // Admin creates group key
        let group_key = EncryptionService::generate_group_key();

        // Admin encrypts group key for member
        let encrypted_key = admin
            .encrypt_group_key_for_member(&group_key, member.public_key())
            .unwrap();

        // Member decrypts group key
        let decrypted_key = member
            .decrypt_group_key(&encrypted_key, admin.public_key())
            .unwrap();

        assert_eq!(group_key, decrypted_key);
    }
}
