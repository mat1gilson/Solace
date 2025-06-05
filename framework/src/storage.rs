//! Storage Module for Solace Protocol
//!
//! Provides persistent storage capabilities for agent data, transactions,
//! reputation scores, and blockchain state. Supports multiple storage backends
//! including RocksDB for high-performance local storage.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use anyhow::Result;
use tokio::sync::RwLock;
use tracing::{info, warn, debug, error};

use crate::{AgentId, TransactionId, error::SolaceError};

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base directory for storage files
    pub data_dir: PathBuf,
    /// Whether to enable compression
    pub enable_compression: bool,
    /// Maximum cache size in MB
    pub cache_size_mb: usize,
    /// Write buffer size in MB
    pub write_buffer_size_mb: usize,
    /// Number of background threads
    pub background_threads: usize,
    /// Enable write-ahead logging
    pub enable_wal: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./solace_data"),
            enable_compression: true,
            cache_size_mb: 256,
            write_buffer_size_mb: 64,
            background_threads: 4,
            enable_wal: true,
        }
    }
}

/// Storage key types for different data categories
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageKey {
    Agent(AgentId),
    Transaction(TransactionId),
    Reputation(AgentId),
    Block(u64),
    State(String),
    Config(String),
    Peer(String),
    Custom(String),
}

impl StorageKey {
    /// Convert storage key to byte representation
    pub fn as_bytes(&self) -> Vec<u8> {
        let prefix_and_key = match self {
            StorageKey::Agent(id) => format!("agent:{}", id),
            StorageKey::Transaction(id) => format!("tx:{}", id),
            StorageKey::Reputation(id) => format!("rep:{}", id),
            StorageKey::Block(height) => format!("block:{}", height),
            StorageKey::State(key) => format!("state:{}", key),
            StorageKey::Config(key) => format!("config:{}", key),
            StorageKey::Peer(id) => format!("peer:{}", id),
            StorageKey::Custom(key) => format!("custom:{}", key),
        };
        prefix_and_key.into_bytes()
    }
}

/// Storage operations trait
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    /// Store a value with the given key
    async fn put<T>(&self, key: StorageKey, value: &T) -> Result<()>
    where
        T: Serialize + Send + Sync;

    /// Retrieve a value by key
    async fn get<T>(&self, key: &StorageKey) -> Result<Option<T>>
    where
        T: DeserializeOwned + Send + Sync;

    /// Delete a key-value pair
    async fn delete(&self, key: &StorageKey) -> Result<()>;

    /// Check if a key exists
    async fn exists(&self, key: &StorageKey) -> Result<bool>;

    /// List all keys with a given prefix
    async fn list_keys(&self, prefix: &str) -> Result<Vec<StorageKey>>;

    /// Batch operations for efficiency
    async fn batch_put<T>(&self, operations: Vec<(StorageKey, T)>) -> Result<()>
    where
        T: Serialize + Send + Sync;

    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats>;

    /// Compact the storage (if supported)
    async fn compact(&self) -> Result<()>;
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_keys: usize,
    pub total_size_bytes: u64,
    pub cache_hit_rate: f64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub delete_ops: u64,
}

/// In-memory storage implementation for testing
pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    stats: Arc<RwLock<StorageStats>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(StorageStats {
                total_keys: 0,
                total_size_bytes: 0,
                cache_hit_rate: 1.0,
                read_ops: 0,
                write_ops: 0,
                delete_ops: 0,
            })),
        }
    }
}

#[async_trait::async_trait]
impl Storage for MemoryStorage {
    async fn put<T>(&self, key: StorageKey, value: &T) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| SolaceError::SerializationError(e.to_string()))?;
        
        let key_bytes = key.as_bytes();
        let mut data = self.data.write().await;
        let is_new_key = !data.contains_key(&key_bytes);
        
        data.insert(key_bytes, serialized.clone());
        
        // Update stats
        let mut stats = self.stats.write().await;
        if is_new_key {
            stats.total_keys += 1;
        }
        stats.total_size_bytes += serialized.len() as u64;
        stats.write_ops += 1;
        
        debug!("Stored value for key: {:?}", key);
        Ok(())
    }

    async fn get<T>(&self, key: &StorageKey) -> Result<Option<T>>
    where
        T: DeserializeOwned + Send + Sync,
    {
        let key_bytes = key.as_bytes();
        let data = self.data.read().await;
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.read_ops += 1;
        
        if let Some(value_bytes) = data.get(&key_bytes) {
            let value = serde_json::from_slice(value_bytes)
                .map_err(|e| SolaceError::DeserializationError(e.to_string()))?;
            debug!("Retrieved value for key: {:?}", key);
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, key: &StorageKey) -> Result<()> {
        let key_bytes = key.as_bytes();
        let mut data = self.data.write().await;
        
        if let Some(removed_value) = data.remove(&key_bytes) {
            // Update stats
            let mut stats = self.stats.write().await;
            stats.total_keys -= 1;
            stats.total_size_bytes -= removed_value.len() as u64;
            stats.delete_ops += 1;
            
            debug!("Deleted key: {:?}", key);
        }
        
        Ok(())
    }

    async fn exists(&self, key: &StorageKey) -> Result<bool> {
        let key_bytes = key.as_bytes();
        let data = self.data.read().await;
        Ok(data.contains_key(&key_bytes))
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<StorageKey>> {
        let data = self.data.read().await;
        let prefix_bytes = prefix.as_bytes();
        
        let matching_keys: Vec<StorageKey> = data
            .keys()
            .filter(|key| key.starts_with(prefix_bytes))
            .filter_map(|key_bytes| {
                if let Ok(key_str) = String::from_utf8(key_bytes.clone()) {
                    Self::parse_storage_key(&key_str)
                } else {
                    None
                }
            })
            .collect();
        
        Ok(matching_keys)
    }

    async fn batch_put<T>(&self, operations: Vec<(StorageKey, T)>) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        for (key, value) in operations {
            self.put(key, &value).await?;
        }
        Ok(())
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn compact(&self) -> Result<()> {
        // No-op for memory storage
        Ok(())
    }
}

impl MemoryStorage {
    fn parse_storage_key(key_str: &str) -> Option<StorageKey> {
        let parts: Vec<&str> = key_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        match parts[0] {
            "agent" => Some(StorageKey::Agent(AgentId::from_string(parts[1]))),
            "tx" => Some(StorageKey::Transaction(TransactionId::from_string(parts[1]))),
            "rep" => Some(StorageKey::Reputation(AgentId::from_string(parts[1]))),
            "block" => parts[1].parse::<u64>().ok().map(StorageKey::Block),
            "state" => Some(StorageKey::State(parts[1].to_string())),
            "config" => Some(StorageKey::Config(parts[1].to_string())),
            "peer" => Some(StorageKey::Peer(parts[1].to_string())),
            "custom" => Some(StorageKey::Custom(parts[1].to_string())),
            _ => None,
        }
    }
}

/// RocksDB storage implementation for production use
#[cfg(feature = "storage")]
pub struct RocksDbStorage {
    db: Arc<rocksdb::DB>,
    stats: Arc<RwLock<StorageStats>>,
}

#[cfg(feature = "storage")]
impl RocksDbStorage {
    pub fn new(config: &StorageConfig) -> Result<Self> {
        use rocksdb::{DB, Options};

        // Create data directory if it doesn't exist
        std::fs::create_dir_all(&config.data_dir)?;

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(if config.enable_compression {
            rocksdb::DBCompressionType::Lz4
        } else {
            rocksdb::DBCompressionType::None
        });
        opts.set_write_buffer_size(config.write_buffer_size_mb * 1024 * 1024);
        opts.set_max_background_jobs(config.background_threads as i32);
        opts.set_use_fsync(false);
        opts.set_disable_auto_compactions(false);

        let db_path = config.data_dir.join("rocksdb");
        let db = DB::open(&opts, db_path)?;

        Ok(Self {
            db: Arc::new(db),
            stats: Arc::new(RwLock::new(StorageStats {
                total_keys: 0,
                total_size_bytes: 0,
                cache_hit_rate: 0.95,
                read_ops: 0,
                write_ops: 0,
                delete_ops: 0,
            })),
        })
    }
}

#[cfg(feature = "storage")]
#[async_trait::async_trait]
impl Storage for RocksDbStorage {
    async fn put<T>(&self, key: StorageKey, value: &T) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| SolaceError::SerializationError(e.to_string()))?;
        
        let key_bytes = key.as_bytes();
        let is_new_key = !self.db.key_may_exist(&key_bytes);
        
        self.db.put(&key_bytes, &serialized)?;
        
        // Update stats
        let mut stats = self.stats.write().await;
        if is_new_key {
            stats.total_keys += 1;
        }
        stats.total_size_bytes += serialized.len() as u64;
        stats.write_ops += 1;
        
        debug!("Stored value for key: {:?}", key);
        Ok(())
    }

    async fn get<T>(&self, key: &StorageKey) -> Result<Option<T>>
    where
        T: DeserializeOwned + Send + Sync,
    {
        let key_bytes = key.as_bytes();
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.read_ops += 1;
        
        match self.db.get(&key_bytes)? {
            Some(value_bytes) => {
                let value = serde_json::from_slice(&value_bytes)
                    .map_err(|e| SolaceError::DeserializationError(e.to_string()))?;
                debug!("Retrieved value for key: {:?}", key);
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, key: &StorageKey) -> Result<()> {
        let key_bytes = key.as_bytes();
        
        if self.db.key_may_exist(&key_bytes) {
            self.db.delete(&key_bytes)?;
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.total_keys -= 1;
            stats.delete_ops += 1;
            
            debug!("Deleted key: {:?}", key);
        }
        
        Ok(())
    }

    async fn exists(&self, key: &StorageKey) -> Result<bool> {
        let key_bytes = key.as_bytes();
        Ok(self.db.get(&key_bytes)?.is_some())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<StorageKey>> {
        let prefix_bytes = prefix.as_bytes();
        let mut keys = Vec::new();
        
        let iter = self.db.prefix_iterator(prefix_bytes);
        for result in iter {
            let (key_bytes, _) = result?;
            if let Ok(key_str) = String::from_utf8(key_bytes.to_vec()) {
                if let Some(storage_key) = MemoryStorage::parse_storage_key(&key_str) {
                    keys.push(storage_key);
                }
            }
        }
        
        Ok(keys)
    }

    async fn batch_put<T>(&self, operations: Vec<(StorageKey, T)>) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        use rocksdb::WriteBatch;
        
        let mut batch = WriteBatch::default();
        
        for (key, value) in operations {
            let serialized = serde_json::to_vec(&value)
                .map_err(|e| SolaceError::SerializationError(e.to_string()))?;
            batch.put(key.as_bytes(), serialized);
        }
        
        self.db.write(batch)?;
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.write_ops += 1;
        
        Ok(())
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn compact(&self) -> Result<()> {
        self.db.compact_range::<&[u8], &[u8]>(None, None);
        info!("Completed storage compaction");
        Ok(())
    }
}

/// Storage manager that provides high-level operations
pub struct StorageManager {
    storage: Box<dyn Storage>,
}

impl StorageManager {
    pub fn new(storage: Box<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Create a new in-memory storage manager
    pub fn memory() -> Self {
        Self::new(Box::new(MemoryStorage::new()))
    }

    /// Create a new RocksDB storage manager
    #[cfg(feature = "storage")]
    pub fn rocksdb(config: &StorageConfig) -> Result<Self> {
        let storage = RocksDbStorage::new(config)?;
        Ok(Self::new(Box::new(storage)))
    }

    /// Store agent data
    pub async fn store_agent<T>(&self, agent_id: &AgentId, data: &T) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        self.storage.put(StorageKey::Agent(agent_id.clone()), data).await
    }

    /// Retrieve agent data
    pub async fn get_agent<T>(&self, agent_id: &AgentId) -> Result<Option<T>>
    where
        T: DeserializeOwned + Send + Sync,
    {
        self.storage.get(&StorageKey::Agent(agent_id.clone())).await
    }

    /// Store transaction data
    pub async fn store_transaction<T>(&self, tx_id: &TransactionId, data: &T) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        self.storage.put(StorageKey::Transaction(tx_id.clone()), data).await
    }

    /// Retrieve transaction data
    pub async fn get_transaction<T>(&self, tx_id: &TransactionId) -> Result<Option<T>>
    where
        T: DeserializeOwned + Send + Sync,
    {
        self.storage.get(&StorageKey::Transaction(tx_id.clone())).await
    }

    /// Store reputation data
    pub async fn store_reputation(&self, agent_id: &AgentId, reputation: f64) -> Result<()> {
        self.storage.put(StorageKey::Reputation(agent_id.clone()), &reputation).await
    }

    /// Get reputation data
    pub async fn get_reputation(&self, agent_id: &AgentId) -> Result<Option<f64>> {
        self.storage.get(&StorageKey::Reputation(agent_id.clone())).await
    }

    /// List all stored agents
    pub async fn list_agents(&self) -> Result<Vec<AgentId>> {
        let keys = self.storage.list_keys("agent:").await?;
        Ok(keys.into_iter().filter_map(|key| {
            if let StorageKey::Agent(agent_id) = key {
                Some(agent_id)
            } else {
                None
            }
        }).collect())
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> Result<StorageStats> {
        self.storage.get_stats().await
    }

    /// Perform storage maintenance
    pub async fn maintenance(&self) -> Result<()> {
        info!("Starting storage maintenance");
        self.storage.compact().await?;
        info!("Storage maintenance completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage() {
        let storage = MemoryStorage::new();
        let key = StorageKey::Agent(AgentId::new());
        let value = "test_data".to_string();

        // Test put
        storage.put(key.clone(), &value).await.unwrap();

        // Test get
        let retrieved: Option<String> = storage.get(&key).await.unwrap();
        assert_eq!(retrieved, Some(value));

        // Test exists
        assert!(storage.exists(&key).await.unwrap());

        // Test delete
        storage.delete(&key).await.unwrap();
        assert!(!storage.exists(&key).await.unwrap());
    }

    #[tokio::test]
    async fn test_storage_manager() {
        let manager = StorageManager::memory();
        let agent_id = AgentId::new();
        let reputation = 0.85;

        // Store and retrieve reputation
        manager.store_reputation(&agent_id, reputation).await.unwrap();
        let retrieved = manager.get_reputation(&agent_id).await.unwrap();
        assert_eq!(retrieved, Some(reputation));
    }

    #[test]
    fn test_storage_key_serialization() {
        let agent_id = AgentId::new();
        let key = StorageKey::Agent(agent_id.clone());
        let bytes = key.as_bytes();
        
        assert!(!bytes.is_empty());
        assert!(String::from_utf8(bytes).unwrap().starts_with("agent:"));
    }
} 