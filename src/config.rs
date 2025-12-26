use crate::errors::{DaggerError, Result};
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub private_key: String,
    pub contract_address: Option<String>,
    pub data_dir: PathBuf,
    pub log_level: String,
}

impl Config {
    pub fn load(
        rpc_url_override: Option<String>,
        wallet_override: Option<String>,
        contract_override: Option<String>,
    ) -> Result<Self> {
        // Get RPC URL
        let rpc_url = rpc_url_override
            .or_else(|| std::env::var("RPC_URL").ok())
            .ok_or_else(|| {
                DaggerError::Config("RPC_URL not set. Use --rpc-url or set RPC_URL env var".into())
            })?;

        // Get private key
        let private_key = wallet_override
            .or_else(|| std::env::var("PRIVATE_KEY").ok())
            .ok_or_else(|| {
                DaggerError::Config(
                    "PRIVATE_KEY not set. Use --wallet or set PRIVATE_KEY env var".into(),
                )
            })?;

        // Get contract address (optional for initial setup)
        let contract_address = contract_override.or_else(|| std::env::var("CONTRACT_ADDRESS").ok());

        // Get data directory
        let data_dir = ProjectDirs::from("com", "daggerchat", "dagger-chat")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".dagger-chat"));

        // Ensure data directory exists
        std::fs::create_dir_all(&data_dir).map_err(|e| {
            DaggerError::Config(format!("Failed to create data directory: {}", e))
        })?;

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            rpc_url,
            private_key,
            contract_address,
            data_dir,
            log_level,
        })
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("dagger.db")
    }

    pub fn keys_path(&self) -> PathBuf {
        self.data_dir.join("keys")
    }
}
