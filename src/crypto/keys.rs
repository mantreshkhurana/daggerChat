use crate::errors::{DaggerError, Result};
use rand::rngs::OsRng;
use std::path::Path;
use x25519_dalek::{PublicKey, StaticSecret};

/// Manages X25519 key pairs for end-to-end encryption
pub struct KeyManager {
    secret: StaticSecret,
    public: PublicKey,
}

impl KeyManager {
    /// Generate a new random key pair
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Create from existing secret key bytes
    pub fn from_secret_bytes(bytes: [u8; 32]) -> Self {
        let secret = StaticSecret::from(bytes);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public
    }

    /// Get the public key as bytes
    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }

    /// Get the secret key bytes (for storage)
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.secret.to_bytes()
    }

    /// Perform Diffie-Hellman key exchange
    pub fn diffie_hellman(&self, their_public: &PublicKey) -> [u8; 32] {
        *self.secret.diffie_hellman(their_public).as_bytes()
    }

    /// Save key pair to file (encrypted with password)
    pub fn save_to_file(&self, path: &Path, password: &str) -> Result<()> {
        use chacha20poly1305::{
            aead::{Aead, KeyInit},
            XChaCha20Poly1305,
        };
        use hkdf::Hkdf;
        use sha2::Sha256;

        // Derive encryption key from password
        let hk = Hkdf::<Sha256>::new(Some(b"dagger-chat-keystore"), password.as_bytes());
        let mut key = [0u8; 32];
        hk.expand(b"keystore-encryption", &mut key)
            .map_err(|_| DaggerError::Encryption("Key derivation failed".into()))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 24];
        rand::RngCore::fill_bytes(&mut OsRng, &mut nonce_bytes);

        // Encrypt secret key
        let cipher = XChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| DaggerError::Encryption(e.to_string()))?;
        let nonce = chacha20poly1305::XNonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, self.secret.to_bytes().as_ref())
            .map_err(|e| DaggerError::Encryption(e.to_string()))?;

        // Write: nonce (24 bytes) || ciphertext
        let mut data = nonce_bytes.to_vec();
        data.extend(ciphertext);

        std::fs::write(path, data)?;
        Ok(())
    }

    /// Load key pair from file (decrypt with password)
    pub fn load_from_file(path: &Path, password: &str) -> Result<Self> {
        use chacha20poly1305::{
            aead::{Aead, KeyInit},
            XChaCha20Poly1305,
        };
        use hkdf::Hkdf;
        use sha2::Sha256;

        let data = std::fs::read(path)?;
        if data.len() < 24 {
            return Err(DaggerError::Decryption("Invalid keystore file".into()));
        }

        // Derive decryption key from password
        let hk = Hkdf::<Sha256>::new(Some(b"dagger-chat-keystore"), password.as_bytes());
        let mut key = [0u8; 32];
        hk.expand(b"keystore-encryption", &mut key)
            .map_err(|_| DaggerError::Decryption("Key derivation failed".into()))?;

        // Decrypt
        let nonce = chacha20poly1305::XNonce::from_slice(&data[..24]);
        let cipher = XChaCha20Poly1305::new_from_slice(&key)
            .map_err(|e| DaggerError::Decryption(e.to_string()))?;
        let plaintext = cipher
            .decrypt(nonce, &data[24..])
            .map_err(|_| DaggerError::Decryption("Invalid password or corrupted keystore".into()))?;

        if plaintext.len() != 32 {
            return Err(DaggerError::Decryption("Invalid key length".into()));
        }

        let mut secret_bytes = [0u8; 32];
        secret_bytes.copy_from_slice(&plaintext);

        Ok(Self::from_secret_bytes(secret_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_key_generation() {
        let km = KeyManager::generate();
        assert_eq!(km.public_key_bytes().len(), 32);
    }

    #[test]
    fn test_diffie_hellman() {
        let alice = KeyManager::generate();
        let bob = KeyManager::generate();

        let alice_shared = alice.diffie_hellman(bob.public_key());
        let bob_shared = bob.diffie_hellman(alice.public_key());

        assert_eq!(alice_shared, bob_shared);
    }

    #[test]
    fn test_save_load_keystore() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.key");
        let password = "test-password-123";

        let original = KeyManager::generate();
        original.save_to_file(&path, password).unwrap();

        let loaded = KeyManager::load_from_file(&path, password).unwrap();
        assert_eq!(original.secret_bytes(), loaded.secret_bytes());
    }

    #[test]
    fn test_wrong_password() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.key");

        let km = KeyManager::generate();
        km.save_to_file(&path, "correct").unwrap();

        let result = KeyManager::load_from_file(&path, "wrong");
        assert!(result.is_err());
    }
}
