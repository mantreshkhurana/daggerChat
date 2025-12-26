use crate::blockchain::{events::EventListener, BlockchainProvider, DaggerChatContract, Wallet};
use crate::chat::{ConversationManager, Message};
use crate::config::Config;
use crate::crypto::{EncryptionService, KeyManager};
use crate::errors::{DaggerError, Result};
use crate::storage::{LocalStorage, SyncManager};
use crate::tui::event::InputMode;
use alloy::primitives::Address;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Current view in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Welcome,
    Chat,
    NewConversation,
    WalletInfo,
}

/// Current focus area
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    Sidebar,
    Chat,
}

/// Application state
pub struct AppState {
    pub current_view: View,
    pub focus: Focus,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub is_syncing: bool,
    pub is_connected: bool,
    pub last_sync: Option<Instant>,
    pub error_message: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_view: View::Welcome,
            focus: Focus::Sidebar,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            is_syncing: false,
            is_connected: true,
            last_sync: None,
            error_message: None,
        }
    }
}

/// Main application struct
pub struct App {
    pub config: Config,
    pub wallet: Arc<Wallet>,
    pub provider: Arc<BlockchainProvider>,
    pub contract: Option<Arc<DaggerChatContract>>,
    pub encryption: Arc<EncryptionService>,
    pub conversations: Arc<RwLock<ConversationManager>>,
    pub storage: Arc<LocalStorage>,
    pub sync_manager: Option<SyncManager>,
    pub state: Arc<RwLock<AppState>>,
}

impl App {
    /// Create a new application instance
    pub async fn new(config: Config) -> Result<Self> {
        tracing::info!("Initializing daggerChat...");

        // Initialize wallet
        let wallet = Wallet::from_private_key(&config.private_key)?;
        tracing::info!("Wallet loaded: {}", wallet.short_address());

        // Initialize blockchain provider
        let provider = BlockchainProvider::new(&config.rpc_url, &wallet).await?;
        tracing::info!("Connected to {}", provider.network_name());

        // Initialize encryption (generate or load keys)
        let key_manager = if config.keys_path().exists() {
            // For now, use a default password - in production, prompt user
            match KeyManager::load_from_file(&config.keys_path(), "dagger-chat") {
                Ok(km) => km,
                Err(_) => {
                    tracing::warn!("Failed to load keys, generating new ones");
                    let km = KeyManager::generate();
                    km.save_to_file(&config.keys_path(), "dagger-chat")?;
                    km
                }
            }
        } else {
            let km = KeyManager::generate();
            std::fs::create_dir_all(config.keys_path().parent().unwrap())?;
            km.save_to_file(&config.keys_path(), "dagger-chat")?;
            km
        };
        let encryption = EncryptionService::new(key_manager);
        tracing::info!("Encryption initialized");

        // Initialize local storage
        let storage = LocalStorage::open(&config.db_path())?;
        tracing::info!("Database opened at {:?}", config.db_path());

        // Initialize conversation manager
        let mut conversations = ConversationManager::new(
            Arc::new(LocalStorage::open(&config.db_path())?),
            wallet.address(),
        );
        conversations.load_from_storage()?;

        // Initialize contract if address provided
        let (contract, sync_manager) = if let Some(addr) = &config.contract_address {
            let address: Address = addr
                .parse()
                .map_err(|_| DaggerError::InvalidAddress(addr.clone()))?;

            let contract = Arc::new(DaggerChatContract::new(address, provider.provider()));

            let event_listener = EventListener::new(provider.provider(), address);
            let sync_manager = SyncManager::new(
                Arc::new(LocalStorage::open(&config.db_path())?),
                Arc::clone(&contract),
                event_listener,
                wallet.address(),
            );

            (Some(contract), Some(sync_manager))
        } else {
            tracing::warn!("No contract address configured - running in offline mode");
            (None, None)
        };

        Ok(Self {
            config,
            wallet: Arc::new(wallet),
            provider: Arc::new(provider),
            contract,
            encryption: Arc::new(encryption),
            conversations: Arc::new(RwLock::new(conversations)),
            storage: Arc::new(storage),
            sync_manager,
            state: Arc::new(RwLock::new(AppState::default())),
        })
    }

    /// Send a message in the current conversation
    pub async fn send_message(&self, content: &str) -> Result<()> {
        let conv_id = {
            let conversations = self.conversations.read().unwrap();
            conversations.selected_id()
        };

        let conv_id = match conv_id {
            Some(id) => id,
            None => return Err(DaggerError::ConversationNotFound("No conversation selected".into())),
        };

        // Get peer's public key for encryption
        let peer_address = {
            let conversations = self.conversations.read().unwrap();
            conversations
                .get(&conv_id)
                .and_then(|c| c.peer_address)
        };

        let peer_address = match peer_address {
            Some(addr) => addr,
            None => return Err(DaggerError::UserNotRegistered("Peer address not found".into())),
        };

        // Create pending message for UI
        let pending_msg = Message::pending(self.wallet.address(), content.to_string());
        {
            let mut conversations = self.conversations.write().unwrap();
            if let Some(conv) = conversations.get_mut(&conv_id) {
                conv.add_message(pending_msg);
            }
        }

        // If we have a contract, send to blockchain
        if let Some(contract) = &self.contract {
            // Get peer's public key from contract
            let peer_public_key = contract.get_user_public_key(peer_address).await?;
            let peer_key = x25519_dalek::PublicKey::from(peer_public_key);

            // Encrypt the message
            let encrypted = self.encryption.encrypt_for_recipient(content.as_bytes(), &peer_key)?;

            // Send to blockchain
            let message_id = contract.send_message(conv_id, encrypted.clone()).await?;

            tracing::info!("Message sent: {}", hex::encode(message_id));

            // Update message with confirmed status
            // In a real implementation, you'd update the pending message
        } else {
            // Offline mode - just store locally
            tracing::warn!("Offline mode - message not sent to blockchain");
        }

        Ok(())
    }

    /// Start a new conversation with an address
    pub async fn start_conversation(&self, address_str: &str) -> Result<()> {
        // Parse address
        let peer_address: Address = address_str
            .parse()
            .map_err(|_| DaggerError::InvalidAddress(address_str.to_string()))?;

        // Generate conversation ID
        let conv_id = if let Some(contract) = &self.contract {
            contract
                .get_dm_conversation_id(self.wallet.address(), peer_address)
                .await?
        } else {
            // Generate locally if no contract
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            let (addr1, addr2) = if self.wallet.address() < peer_address {
                (self.wallet.address(), peer_address)
            } else {
                (peer_address, self.wallet.address())
            };
            hasher.update(b"dm");
            hasher.update(addr1.as_slice());
            hasher.update(addr2.as_slice());
            let result = hasher.finalize();
            let mut id = [0u8; 32];
            id.copy_from_slice(&result);
            id
        };

        // Create or get conversation
        {
            let mut conversations = self.conversations.write().unwrap();
            conversations.get_or_create_dm(conv_id, peer_address);
            // Select the new conversation
            let count = conversations.count();
            if count > 0 {
                conversations.select(0); // New conversations appear at top
            }
        }

        // Switch to chat view
        {
            let mut state = self.state.write().unwrap();
            state.current_view = View::Chat;
            state.focus = Focus::Chat;
        }

        tracing::info!("Started conversation with {}", address_str);
        Ok(())
    }

    /// Sync messages from blockchain
    pub async fn sync(&self) -> Result<()> {
        if let Some(sync_manager) = &self.sync_manager {
            {
                let mut state = self.state.write().unwrap();
                state.is_syncing = true;
            }

            match sync_manager.sync(&self.encryption).await {
                Ok(count) => {
                    tracing::info!("Synced {} messages", count);
                }
                Err(e) => {
                    tracing::error!("Sync failed: {}", e);
                }
            }

            // Also flush pending messages
            if let Ok(sent) = sync_manager.flush_pending().await {
                if sent > 0 {
                    tracing::info!("Sent {} pending messages", sent);
                }
            }

            {
                let mut state = self.state.write().unwrap();
                state.is_syncing = false;
                state.last_sync = Some(Instant::now());
            }
        }

        Ok(())
    }

    /// Periodic sync (called from main loop)
    pub async fn maybe_sync(&mut self) {
        let should_sync = {
            let state = self.state.read().unwrap();
            if state.is_syncing {
                false
            } else if let Some(last) = state.last_sync {
                last.elapsed() > Duration::from_secs(30)
            } else {
                true
            }
        };

        if should_sync {
            let _ = self.sync().await;
        }
    }

    /// Register user on blockchain
    pub async fn register(&self) -> Result<()> {
        if let Some(contract) = &self.contract {
            // Check if already registered
            if contract.is_user_registered(self.wallet.address()).await? {
                tracing::info!("User already registered");
                return Ok(());
            }

            // Register with our encryption public key
            let public_key = self.encryption.public_key_bytes();
            let tx_hash = contract.register_user(public_key, vec![]).await?;

            tracing::info!("Registered user, tx: {}", hex::encode(tx_hash.as_slice()));
        } else {
            return Err(DaggerError::Blockchain("No contract configured".into()));
        }

        Ok(())
    }
}
