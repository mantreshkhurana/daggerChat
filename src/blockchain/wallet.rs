use crate::errors::{DaggerError, Result};
use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;

/// Manages the Ethereum wallet for blockchain operations
#[derive(Clone)]
pub struct Wallet {
    signer: PrivateKeySigner,
    address: Address,
}

impl Wallet {
    /// Create wallet from private key hex string
    pub fn from_private_key(private_key: &str) -> Result<Self> {
        // Remove 0x prefix if present
        let key = private_key.strip_prefix("0x").unwrap_or(private_key);

        let signer: PrivateKeySigner = key
            .parse()
            .map_err(|e| DaggerError::Wallet(format!("Invalid private key: {}", e)))?;

        let address = signer.address();

        Ok(Self { signer, address })
    }

    /// Get the wallet address
    pub fn address(&self) -> Address {
        self.address
    }

    /// Get the signer for transaction signing
    pub fn signer(&self) -> &PrivateKeySigner {
        &self.signer
    }

    /// Get address as hex string with 0x prefix
    pub fn address_string(&self) -> String {
        format!("{:?}", self.address)
    }

    /// Get shortened address for display (e.g., 0x1234...5678)
    pub fn short_address(&self) -> String {
        let addr = self.address_string();
        if addr.len() > 12 {
            format!("{}...{}", &addr[..6], &addr[addr.len() - 4..])
        } else {
            addr
        }
    }
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address_string())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        // Test private key (DO NOT use in production!)
        let test_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet = Wallet::from_private_key(test_key).unwrap();

        assert!(wallet.address_string().starts_with("0x"));
        assert_eq!(wallet.address_string().len(), 42);
    }

    #[test]
    fn test_short_address() {
        let test_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let wallet = Wallet::from_private_key(test_key).unwrap();

        let short = wallet.short_address();
        assert!(short.contains("..."));
        assert!(short.len() < 15);
    }

    #[test]
    fn test_invalid_key() {
        let result = Wallet::from_private_key("invalid");
        assert!(result.is_err());
    }
}
