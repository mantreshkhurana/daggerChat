-- daggerChat SQLite Schema

-- Users table (cached from blockchain)
CREATE TABLE IF NOT EXISTS users (
    address TEXT PRIMARY KEY,
    public_key_x25519 BLOB NOT NULL,
    encrypted_metadata BLOB,
    registered_at INTEGER NOT NULL,
    last_synced_at INTEGER NOT NULL
);

-- Conversations
CREATE TABLE IF NOT EXISTS conversations (
    id BLOB PRIMARY KEY,
    conversation_type TEXT NOT NULL CHECK (conversation_type IN ('dm', 'group')),
    display_name TEXT,
    created_at INTEGER NOT NULL,
    last_message_at INTEGER,
    last_synced_block INTEGER DEFAULT 0
);

-- Conversation participants (for groups)
CREATE TABLE IF NOT EXISTS conversation_members (
    conversation_id BLOB NOT NULL,
    member_address TEXT NOT NULL,
    joined_at INTEGER NOT NULL,
    is_admin INTEGER DEFAULT 0,
    PRIMARY KEY (conversation_id, member_address),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);

-- Messages (decrypted locally)
CREATE TABLE IF NOT EXISTS messages (
    id BLOB PRIMARY KEY,
    conversation_id BLOB NOT NULL,
    sender_address TEXT NOT NULL,
    content TEXT NOT NULL,
    encrypted_content BLOB NOT NULL,
    timestamp INTEGER NOT NULL,
    tx_hash TEXT,
    block_number INTEGER,
    is_pending INTEGER DEFAULT 0,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);

-- Our keypairs
CREATE TABLE IF NOT EXISTS keypairs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    public_key BLOB NOT NULL UNIQUE,
    encrypted_private_key BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    is_active INTEGER DEFAULT 1
);

-- Pending outgoing messages (not yet on blockchain)
CREATE TABLE IF NOT EXISTS pending_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id BLOB NOT NULL,
    encrypted_content BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    retry_count INTEGER DEFAULT 0,
    last_error TEXT,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);

-- Sync state
CREATE TABLE IF NOT EXISTS sync_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_pending ON messages(is_pending) WHERE is_pending = 1;
CREATE INDEX IF NOT EXISTS idx_conversations_last_message ON conversations(last_message_at DESC);
CREATE INDEX IF NOT EXISTS idx_pending_created ON pending_messages(created_at);
