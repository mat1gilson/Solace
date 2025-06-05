//! Gossip Protocol Module
//!
//! Implements efficient information dissemination across the Solace Protocol network
//! using epidemiological gossip algorithms for scalable peer-to-peer communication.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tracing::{info, warn, debug, error};
use std::sync::Arc;

/// Gossip message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GossipMessageType {
    PeerAnnouncement,
    TransactionBroadcast,
    StateUpdate,
    HeartBeat,
    RoutingUpdate,
    ReputationUpdate,
    Custom(String),
}

/// Gossip message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub id: String,
    pub message_type: GossipMessageType,
    pub sender_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub ttl: u32,
    pub hop_count: u32,
    pub payload: serde_json::Value,
    pub signature: Option<String>,
    pub routing_path: Vec<String>,
}

impl GossipMessage {
    /// Create a new gossip message
    pub fn new(
        message_type: GossipMessageType,
        sender_id: String,
        payload: serde_json::Value,
        ttl: u32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message_type,
            sender_id,
            timestamp: chrono::Utc::now(),
            ttl,
            hop_count: 0,
            payload,
            signature: None,
            routing_path: Vec::new(),
        }
    }

    /// Check if message has expired
    pub fn is_expired(&self) -> bool {
        self.ttl == 0 || self.hop_count > 10 // Max hop limit
    }

    /// Decrement TTL and increment hop count
    pub fn forward(&mut self, node_id: &str) -> bool {
        if self.is_expired() {
            return false;
        }
        
        self.ttl = self.ttl.saturating_sub(1);
        self.hop_count += 1;
        self.routing_path.push(node_id.to_string());
        
        !self.is_expired()
    }
}

/// Gossip configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipConfig {
    pub fanout: usize,                    // Number of peers to gossip to
    pub gossip_interval: Duration,        // How often to gossip
    pub message_ttl: u32,                 // Default message TTL
    pub max_message_cache: usize,         // Max messages to cache
    pub duplicate_window: Duration,       // Window for duplicate detection
    pub heartbeat_interval: Duration,     // Heartbeat frequency
    pub enable_anti_entropy: bool,        // Enable anti-entropy protocol
    pub compression: bool,                // Enable message compression
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            fanout: 3,
            gossip_interval: Duration::from_secs(5),
            message_ttl: 10,
            max_message_cache: 1000,
            duplicate_window: Duration::from_secs(60),
            heartbeat_interval: Duration::from_secs(30),
            enable_anti_entropy: true,
            compression: false,
        }
    }
}

/// Gossip statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GossipStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_forwarded: u64,
    pub duplicates_filtered: u64,
    pub expired_messages: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub active_peers: usize,
}

/// Peer information for gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPeer {
    pub id: String,
    pub last_seen: Instant,
    pub message_count: u64,
    pub is_active: bool,
    pub latency: Duration,
}

/// Message cache entry
#[derive(Debug, Clone)]
struct CacheEntry {
    message: GossipMessage,
    received_at: Instant,
    forwarded_to: HashSet<String>,
}

/// Gossip protocol implementation
pub struct GossipProtocol {
    node_id: String,
    config: GossipConfig,
    peers: Arc<RwLock<HashMap<String, GossipPeer>>>,
    message_cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<GossipStats>>,
    message_handlers: HashMap<GossipMessageType, Box<dyn Fn(&GossipMessage) -> Result<()> + Send + Sync>>,
    outbound_tx: mpsc::UnboundedSender<(String, GossipMessage)>,
    outbound_rx: Option<mpsc::UnboundedReceiver<(String, GossipMessage)>>,
}

impl GossipProtocol {
    /// Create a new gossip protocol instance
    pub fn new(node_id: String, config: GossipConfig) -> Self {
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        
        Self {
            node_id,
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            message_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(GossipStats::default())),
            message_handlers: HashMap::new(),
            outbound_tx,
            outbound_rx: Some(outbound_rx),
        }
    }

    /// Start the gossip protocol
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting gossip protocol for node: {}", self.node_id);
        
        // Start periodic gossip
        self.start_periodic_gossip().await;
        
        // Start message processing
        if let Some(rx) = self.outbound_rx.take() {
            self.start_message_processor(rx).await;
        }
        
        // Start maintenance tasks
        self.start_maintenance_tasks().await;
        
        Ok(())
    }

    /// Add a peer to the gossip network
    pub async fn add_peer(&self, peer_id: String) {
        let peer = GossipPeer {
            id: peer_id.clone(),
            last_seen: Instant::now(),
            message_count: 0,
            is_active: true,
            latency: Duration::from_millis(50), // Default latency
        };
        
        let mut peers = self.peers.write().await;
        peers.insert(peer_id.clone(), peer);
        
        let mut stats = self.stats.write().await;
        stats.active_peers = peers.len();
        
        debug!("Added gossip peer: {}", peer_id);
    }

    /// Remove a peer from the gossip network
    pub async fn remove_peer(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        if peers.remove(peer_id).is_some() {
            let mut stats = self.stats.write().await;
            stats.active_peers = peers.len();
            debug!("Removed gossip peer: {}", peer_id);
        }
    }

    /// Broadcast a message to the network
    pub async fn broadcast(&self, message_type: GossipMessageType, payload: serde_json::Value) -> Result<()> {
        let message = GossipMessage::new(
            message_type,
            self.node_id.clone(),
            payload,
            self.config.message_ttl,
        );
        
        self.gossip_message(message).await
    }

    /// Gossip a specific message
    pub async fn gossip_message(&self, message: GossipMessage) -> Result<()> {
        // Cache the message
        self.cache_message(message.clone()).await;
        
        // Select peers to gossip to
        let target_peers = self.select_gossip_targets().await;
        
        // Send to selected peers
        for peer_id in target_peers {
            if let Err(e) = self.outbound_tx.send((peer_id.clone(), message.clone())) {
                error!("Failed to queue message for peer {}: {}", peer_id, e);
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.messages_sent += 1;
        
        Ok(())
    }

    /// Process incoming gossip message
    pub async fn handle_incoming_message(&self, message: GossipMessage) -> Result<()> {
        let mut stats = self.stats.write().await;
        stats.messages_received += 1;
        stats.bytes_received += serde_json::to_vec(&message)?.len() as u64;
        drop(stats);
        
        // Check for duplicates
        if self.is_duplicate(&message).await {
            let mut stats = self.stats.write().await;
            stats.duplicates_filtered += 1;
            return Ok(());
        }
        
        // Check if expired
        if message.is_expired() {
            let mut stats = self.stats.write().await;
            stats.expired_messages += 1;
            return Ok(());
        }
        
        // Process the message
        self.process_message(&message).await?;
        
        // Forward the message if appropriate
        if self.should_forward(&message).await {
            self.forward_message(message).await?;
        }
        
        Ok(())
    }

    /// Register a message handler
    pub fn register_handler<F>(&mut self, message_type: GossipMessageType, handler: F)
    where
        F: Fn(&GossipMessage) -> Result<()> + Send + Sync + 'static,
    {
        self.message_handlers.insert(message_type, Box::new(handler));
    }

    /// Process a message using registered handlers
    async fn process_message(&self, message: &GossipMessage) -> Result<()> {
        if let Some(handler) = self.message_handlers.get(&message.message_type) {
            handler(message)?;
        } else {
            debug!("No handler registered for message type: {:?}", message.message_type);
        }
        
        // Update peer information
        self.update_peer_info(&message.sender_id).await;
        
        Ok(())
    }

    /// Check if message is a duplicate
    async fn is_duplicate(&self, message: &GossipMessage) -> bool {
        let cache = self.message_cache.read().await;
        cache.contains_key(&message.id)
    }

    /// Cache a message
    async fn cache_message(&self, message: GossipMessage) {
        let mut cache = self.message_cache.write().await;
        
        let entry = CacheEntry {
            message: message.clone(),
            received_at: Instant::now(),
            forwarded_to: HashSet::new(),
        };
        
        cache.insert(message.id.clone(), entry);
        
        // Cleanup old entries if cache is full
        if cache.len() > self.config.max_message_cache {
            self.cleanup_cache(&mut cache);
        }
    }

    /// Clean up old cache entries
    fn cleanup_cache(&self, cache: &mut HashMap<String, CacheEntry>) {
        let cutoff = Instant::now() - self.config.duplicate_window;
        
        cache.retain(|_, entry| entry.received_at > cutoff);
        
        // If still too many, remove oldest entries
        if cache.len() > self.config.max_message_cache {
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by_key(|(_, entry)| entry.received_at);
            
            let to_remove = cache.len() - self.config.max_message_cache + 100; // Remove extra
            for (id, _) in entries.iter().take(to_remove) {
                cache.remove(*id);
            }
        }
    }

    /// Determine if message should be forwarded
    async fn should_forward(&self, message: &GossipMessage) -> bool {
        // Don't forward our own messages
        if message.sender_id == self.node_id {
            return false;
        }
        
        // Don't forward expired messages
        if message.is_expired() {
            return false;
        }
        
        // Check if we've already forwarded to enough peers
        let cache = self.message_cache.read().await;
        if let Some(entry) = cache.get(&message.id) {
            return entry.forwarded_to.len() < self.config.fanout;
        }
        
        true
    }

    /// Forward a message
    async fn forward_message(&self, mut message: GossipMessage) -> Result<()> {
        // Update message for forwarding
        if !message.forward(&self.node_id) {
            return Ok(()); // Message expired during forwarding
        }
        
        // Select peers to forward to (excluding sender and previous forwarders)
        let target_peers = self.select_forward_targets(&message).await;
        
        // Send to selected peers
        for peer_id in &target_peers {
            if let Err(e) = self.outbound_tx.send((peer_id.clone(), message.clone())) {
                error!("Failed to queue forwarded message for peer {}: {}", peer_id, e);
            }
        }
        
        // Update cache with forwarding info
        let mut cache = self.message_cache.write().await;
        if let Some(entry) = cache.get_mut(&message.id) {
            for peer_id in target_peers {
                entry.forwarded_to.insert(peer_id);
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.messages_forwarded += 1;
        
        Ok(())
    }

    /// Select peers for gossiping
    async fn select_gossip_targets(&self) -> Vec<String> {
        let peers = self.peers.read().await;
        let active_peers: Vec<_> = peers
            .values()
            .filter(|peer| peer.is_active)
            .collect();
        
        if active_peers.is_empty() {
            return Vec::new();
        }
        
        let target_count = std::cmp::min(self.config.fanout, active_peers.len());
        
        // Simple random selection for now
        // In production, this could use more sophisticated selection algorithms
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        active_peers
            .choose_multiple(&mut rng, target_count)
            .map(|peer| peer.id.clone())
            .collect()
    }

    /// Select peers for forwarding (excluding sender and routing path)
    async fn select_forward_targets(&self, message: &GossipMessage) -> Vec<String> {
        let peers = self.peers.read().await;
        let excluded: HashSet<_> = message.routing_path.iter().cloned().collect();
        
        let available_peers: Vec<_> = peers
            .values()
            .filter(|peer| {
                peer.is_active && 
                peer.id != message.sender_id && 
                !excluded.contains(&peer.id)
            })
            .collect();
        
        if available_peers.is_empty() {
            return Vec::new();
        }
        
        let target_count = std::cmp::min(self.config.fanout, available_peers.len());
        
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        available_peers
            .choose_multiple(&mut rng, target_count)
            .map(|peer| peer.id.clone())
            .collect()
    }

    /// Update peer information
    async fn update_peer_info(&self, peer_id: &str) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.last_seen = Instant::now();
            peer.message_count += 1;
            peer.is_active = true;
        }
    }

    /// Start periodic gossip task
    async fn start_periodic_gossip(&self) {
        let config = self.config.clone();
        let peers = self.peers.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(config.gossip_interval);
            
            loop {
                interval.tick().await;
                
                // Send heartbeat messages
                let heartbeat = GossipMessage::new(
                    GossipMessageType::HeartBeat,
                    "node_id".to_string(), // Would use actual node ID
                    serde_json::json!({"timestamp": chrono::Utc::now()}),
                    5, // Short TTL for heartbeats
                );
                
                // This would trigger gossip of heartbeat message
                debug!("Periodic heartbeat gossip");
            }
        });
    }

    /// Start message processor task
    async fn start_message_processor(&self, mut rx: mpsc::UnboundedReceiver<(String, GossipMessage)>) {
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            while let Some((peer_id, message)) = rx.recv().await {
                // Simulate sending message to peer
                debug!("Sending message {} to peer {}", message.id, peer_id);
                
                // Update stats
                let mut stats = stats.write().await;
                stats.bytes_sent += serde_json::to_vec(&message).unwrap_or_default().len() as u64;
                
                // In a real implementation, this would send over the network
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    }

    /// Start maintenance tasks
    async fn start_maintenance_tasks(&self) {
        let peers = self.peers.clone();
        let cache = self.message_cache.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut cleanup_interval = interval(Duration::from_secs(60));
            
            loop {
                cleanup_interval.tick().await;
                
                // Clean up inactive peers
                let now = Instant::now();
                let mut peers = peers.write().await;
                peers.retain(|_, peer| {
                    let is_active = now.duration_since(peer.last_seen) < Duration::from_secs(300);
                    if !is_active {
                        debug!("Marking peer as inactive: {}", peer.id);
                    }
                    peer.is_active = is_active;
                    true // Keep peer but mark as inactive
                });
                drop(peers);
                
                // Clean up message cache
                let mut cache = cache.write().await;
                let cutoff = now - config.duplicate_window;
                cache.retain(|_, entry| entry.received_at > cutoff);
            }
        });
    }

    /// Get gossip statistics
    pub async fn get_stats(&self) -> GossipStats {
        self.stats.read().await.clone()
    }

    /// Get active peer count
    pub async fn get_peer_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.values().filter(|peer| peer.is_active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gossip_message_creation() {
        let message = GossipMessage::new(
            GossipMessageType::PeerAnnouncement,
            "test_node".to_string(),
            serde_json::json!({"test": "data"}),
            10,
        );
        
        assert_eq!(message.sender_id, "test_node");
        assert_eq!(message.ttl, 10);
        assert_eq!(message.hop_count, 0);
        assert!(!message.is_expired());
    }

    #[tokio::test]
    async fn test_message_forwarding() {
        let mut message = GossipMessage::new(
            GossipMessageType::PeerAnnouncement,
            "sender".to_string(),
            serde_json::json!({}),
            3,
        );
        
        assert!(message.forward("node1"));
        assert_eq!(message.ttl, 2);
        assert_eq!(message.hop_count, 1);
        assert_eq!(message.routing_path, vec!["node1"]);
        
        assert!(message.forward("node2"));
        assert!(message.forward("node3"));
        assert!(!message.forward("node4")); // Should be expired now
    }

    #[tokio::test]
    async fn test_gossip_protocol() {
        let config = GossipConfig::default();
        let mut protocol = GossipProtocol::new("test_node".to_string(), config);
        
        protocol.add_peer("peer1".to_string()).await;
        protocol.add_peer("peer2".to_string()).await;
        
        assert_eq!(protocol.get_peer_count().await, 2);
        
        let stats = protocol.get_stats().await;
        assert_eq!(stats.active_peers, 2);
    }
} 