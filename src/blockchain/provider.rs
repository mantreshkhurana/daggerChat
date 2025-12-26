use crate::errors::{DaggerError, Result};
use alloy::network::EthereumWallet;
use alloy::providers::{Provider, ProviderBuilder, ReqwestProvider};
use std::sync::Arc;

use super::Wallet;

/// Type alias for the provider
pub type HttpProvider = ReqwestProvider;

/// Manages the blockchain connection and provider
pub struct BlockchainProvider {
    provider: Arc<HttpProvider>,
    wallet: EthereumWallet,
    chain_id: u64,
    rpc_url: String,
}

impl BlockchainProvider {
    /// Create a new provider connected to the RPC endpoint
    pub async fn new(rpc_url: &str, wallet: &Wallet) -> Result<Self> {
        let eth_wallet = EthereumWallet::from(wallet.signer().clone());

        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(eth_wallet.clone())
            .on_http(
                rpc_url
                    .parse()
                    .map_err(|e| DaggerError::Network(format!("Invalid RPC URL: {}", e)))?,
            );

        let chain_id = provider
            .get_chain_id()
            .await
            .map_err(|e| DaggerError::Network(format!("Failed to get chain ID: {}", e)))?;

        tracing::info!("Connected to chain ID: {}", chain_id);

        // Create a simple provider for contract calls
        let simple_provider = ProviderBuilder::new()
            .on_http(rpc_url.parse().unwrap());

        Ok(Self {
            provider: Arc::new(simple_provider),
            wallet: eth_wallet,
            chain_id,
            rpc_url: rpc_url.to_string(),
        })
    }

    /// Get the provider for contract interactions
    pub fn provider(&self) -> Arc<HttpProvider> {
        Arc::clone(&self.provider)
    }

    /// Get the wallet
    pub fn wallet(&self) -> &EthereumWallet {
        &self.wallet
    }

    /// Get the chain ID
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Get the latest block number
    pub async fn block_number(&self) -> Result<u64> {
        self.provider
            .get_block_number()
            .await
            .map_err(|e| DaggerError::Network(format!("Failed to get block number: {}", e)))
    }

    /// Check if connected to the network
    pub async fn is_connected(&self) -> bool {
        self.provider.get_chain_id().await.is_ok()
    }

    /// Get the network name based on chain ID
    pub fn network_name(&self) -> &'static str {
        match self.chain_id {
            1 => "Ethereum Mainnet",
            42161 => "Arbitrum One",
            421614 => "Arbitrum Sepolia",
            8453 => "Base",
            84532 => "Base Sepolia",
            11155111 => "Sepolia",
            _ => "Unknown Network",
        }
    }
}
