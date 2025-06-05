//! Peer Discovery Module
//!
//! Handles automatic discovery and management of peers in the Solace Protocol network.
//! Implements various discovery mechanisms including DHT, gossip, and bootstrap nodes.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use tokio::time::interval;
use tracing::{info, warn, debug, error};

/// Peer information structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PeerInfo {
    pub id: String,
    pub address: SocketAddr,
    pub public_key: String,
    pub capabilities: Vec<String>,
    pub reputation: f64,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub protocol_version: String,
    pub node_type: NodeType,
}

/// Types of nodes in the network
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    Agent,
    Validator,
    Relay,
    Bootstrap,
    Client,
}

/// Discovery method enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    Bootstrap,
    DHT,
    Gossip,
    Manual,
    MDNS,
    DNS,
}

/// Discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub bootstrap_nodes: Vec<SocketAddr>,
    pub max_peers: usize,
    pub discovery_interval: Duration,
    pub peer_timeout: Duration,
    pub enable_dht: bool,
    pub enable_gossip: bool,
    pub enable_mdns: bool,
    pub reputation_threshold: f64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            bootstrap_nodes: vec![
                "bootstrap1.solace.network:8080".parse().unwrap(),
                "bootstrap2.solace.network:8080".parse().unwrap(),
            ],
            max_peers: 50,
            discovery_interval: Duration::from_secs(30),
            peer_timeout: Duration::from_secs(300),
            enable_dht: true,
            enable_gossip: true,
            enable_mdns: false,
            reputation_threshold: 0.3,
        }
    }
}

/// Discovery statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DiscoveryStats {
    pub total_discovered: u64,
    pub active_peers: usize,
    pub bootstrap_attempts: u64,
    pub dht_queries: u64,
    pub gossip_messages: u64,
    pub failed_connections: u64,
    pub peer_disconnections: u64,
}

/// Discovery event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryEvent {
    PeerDiscovered(PeerInfo),
    PeerConnected(String),
    PeerDisconnected(String),
    PeerTimeout(String),
    BootstrapCompleted,
    DiscoveryFailed(String),
}

/// Peer discovery service
pub struct PeerDiscovery {
    config: DiscoveryConfig,
    known_peers: HashMap<String, PeerInfo>,
    connected_peers: HashSet<String>,
    blacklisted_peers: HashSet<String>,
    stats: DiscoveryStats,
    last_discovery: Instant,
    event_callbacks: Vec<Box<dyn Fn(DiscoveryEvent) + Send + Sync>>,
}

impl PeerDiscovery {
    /// Create a new peer discovery service
    pub fn new(config: DiscoveryConfig) -> Self {
        Self {
            config,
            known_peers: HashMap::new(),
            connected_peers: HashSet::new(),
            blacklisted_peers: HashSet::new(),
            stats: DiscoveryStats::default(),
            last_discovery: Instant::now(),
            event_callbacks: Vec::new(),
        }
    }

    /// Start the discovery service
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting peer discovery service");
        
        // Bootstrap from known nodes
        self.bootstrap().await?;
        
        // Start periodic discovery
        self.start_periodic_discovery().await;
        
        Ok(())
    }

    /// Bootstrap from configured bootstrap nodes
    async fn bootstrap(&mut self) -> Result<()> {
        info!("Bootstrapping from {} nodes", self.config.bootstrap_nodes.len());
        
        for bootstrap_addr in &self.config.bootstrap_nodes {
            self.stats.bootstrap_attempts += 1;
            
            match self.connect_to_bootstrap(*bootstrap_addr).await {
                Ok(peers) => {
                    info!("Successfully bootstrapped from {}, discovered {} peers", 
                        bootstrap_addr, peers.len());
                    
                    for peer in peers {
                        self.add_peer(peer, DiscoveryMethod::Bootstrap).await;
                    }
                },
                Err(e) => {
                    warn!("Failed to bootstrap from {}: {}", bootstrap_addr, e);
                    self.stats.failed_connections += 1;
                }
            }
        }
        
        self.emit_event(DiscoveryEvent::BootstrapCompleted);
        Ok(())
    }

    /// Connect to a bootstrap node and get peer list
    async fn connect_to_bootstrap(&self, addr: SocketAddr) -> Result<Vec<PeerInfo>> {
        // Simulate bootstrap connection and peer list retrieval
        debug!("Connecting to bootstrap node: {}", addr);
        
        // In a real implementation, this would make an actual network request
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let peers = vec![
            PeerInfo {
                id: format!("peer-{}", uuid::Uuid::new_v4()),
                address: "192.168.1.100:8080".parse()?,
                public_key: "bootstrap_peer_key_1".to_string(),
                capabilities: vec!["relay".to_string(), "validator".to_string()],
                reputation: 0.9,
                last_seen: chrono::Utc::now(),
                protocol_version: "1.0.0".to_string(),
                node_type: NodeType::Validator,
            },
            PeerInfo {
                id: format!("peer-{}", uuid::Uuid::new_v4()),
                address: "192.168.1.101:8080".parse()?,
                public_key: "bootstrap_peer_key_2".to_string(),
                capabilities: vec!["agent".to_string()],
                reputation: 0.8,
                last_seen: chrono::Utc::now(),
                protocol_version: "1.0.0".to_string(),
                node_type: NodeType::Agent,
            },
        ];
        
        Ok(peers)
    }

    /// Start periodic peer discovery
    async fn start_periodic_discovery(&mut self) {
        let mut discovery_interval = interval(self.config.discovery_interval);
        
        loop {
            discovery_interval.tick().await;
            
            if let Err(e) = self.discover_peers().await {
                error!("Periodic discovery failed: {}", e);
            }
            
            self.cleanup_inactive_peers().await;
        }
    }

    /// Discover new peers using various methods
    async fn discover_peers(&mut self) -> Result<()> {
        debug!("Starting peer discovery round");
        self.last_discovery = Instant::now();
        
        let mut new_peers = Vec::new();
        
        // DHT-based discovery
        if self.config.enable_dht {
            if let Ok(dht_peers) = self.dht_discovery().await {
                new_peers.extend(dht_peers);
            }
        }
        
        // Gossip-based discovery
        if self.config.enable_gossip {
            if let Ok(gossip_peers) = self.gossip_discovery().await {
                new_peers.extend(gossip_peers);
            }
        }
        
        // mDNS discovery (local network)
        if self.config.enable_mdns {
            if let Ok(mdns_peers) = self.mdns_discovery().await {
                new_peers.extend(mdns_peers);
            }
        }
        
        // Add discovered peers
        for peer in new_peers {
            self.add_peer(peer, DiscoveryMethod::DHT).await;
        }
        
        info!("Discovery round completed. Known peers: {}, Connected: {}", 
            self.known_peers.len(), self.connected_peers.len());
        
        Ok(())
    }

    /// DHT-based peer discovery
    async fn dht_discovery(&mut self) -> Result<Vec<PeerInfo>> {
        self.stats.dht_queries += 1;
        
        // Simulate DHT query
        debug!("Performing DHT peer discovery");
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        let peers = vec![
            PeerInfo {
                id: format!("dht-peer-{}", uuid::Uuid::new_v4()),
                address: "10.0.1.50:8080".parse()?,
                public_key: "dht_peer_key".to_string(),
                capabilities: vec!["agent".to_string(), "relay".to_string()],
                reputation: 0.7,
                last_seen: chrono::Utc::now(),
                protocol_version: "1.0.0".to_string(),
                node_type: NodeType::Agent,
            },
        ];
        
        Ok(peers)
    }

    /// Gossip-based peer discovery
    async fn gossip_discovery(&mut self) -> Result<Vec<PeerInfo>> {
        self.stats.gossip_messages += 1;
        
        // Simulate gossip protocol
        debug!("Performing gossip peer discovery");
        tokio::time::sleep(Duration::from_millis(30)).await;
        
        let peers = vec![
            PeerInfo {
                id: format!("gossip-peer-{}", uuid::Uuid::new_v4()),
                address: "172.16.0.25:8080".parse()?,
                public_key: "gossip_peer_key".to_string(),
                capabilities: vec!["validator".to_string()],
                reputation: 0.85,
                last_seen: chrono::Utc::now(),
                protocol_version: "1.0.0".to_string(),
                node_type: NodeType::Validator,
            },
        ];
        
        Ok(peers)
    }

    /// mDNS local network discovery
    async fn mdns_discovery(&self) -> Result<Vec<PeerInfo>> {
        debug!("Performing mDNS peer discovery");
        tokio::time::sleep(Duration::from_millis(20)).await;
        
        // Simulate local network discovery
        Ok(Vec::new())
    }

    /// Add a newly discovered peer
    async fn add_peer(&mut self, peer: PeerInfo, method: DiscoveryMethod) {
        // Check if peer is blacklisted
        if self.blacklisted_peers.contains(&peer.id) {
            debug!("Ignoring blacklisted peer: {}", peer.id);
            return;
        }
        
        // Check reputation threshold
        if peer.reputation < self.config.reputation_threshold {
            debug!("Ignoring peer with low reputation: {} ({})", peer.id, peer.reputation);
            return;
        }
        
        // Check if we've reached max peers
        if self.known_peers.len() >= self.config.max_peers {
            debug!("Max peers reached, not adding: {}", peer.id);
            return;
        }
        
        let is_new = !self.known_peers.contains_key(&peer.id);
        
        if is_new {
            self.stats.total_discovered += 1;
            info!("Discovered new peer: {} via {:?}", peer.id, method);
            self.emit_event(DiscoveryEvent::PeerDiscovered(peer.clone()));
        }
        
        self.known_peers.insert(peer.id.clone(), peer);
    }

    /// Remove inactive peers
    async fn cleanup_inactive_peers(&mut self) {
        let now = chrono::Utc::now();
        let timeout_threshold = now - chrono::Duration::from_std(self.config.peer_timeout).unwrap();
        
        let inactive_peers: Vec<String> = self.known_peers
            .iter()
            .filter(|(_, peer)| peer.last_seen < timeout_threshold)
            .map(|(id, _)| id.clone())
            .collect();
        
        for peer_id in inactive_peers {
            self.remove_peer(&peer_id).await;
        }
    }

    /// Remove a peer
    async fn remove_peer(&mut self, peer_id: &str) {
        if self.known_peers.remove(peer_id).is_some() {
            self.connected_peers.remove(peer_id);
            self.stats.peer_disconnections += 1;
            debug!("Removed inactive peer: {}", peer_id);
            self.emit_event(DiscoveryEvent::PeerTimeout(peer_id.to_string()));
        }
    }

    /// Blacklist a peer
    pub fn blacklist_peer(&mut self, peer_id: &str) {
        self.blacklisted_peers.insert(peer_id.to_string());
        self.remove_peer(peer_id);
        warn!("Blacklisted peer: {}", peer_id);
    }

    /// Get list of known peers
    pub fn get_known_peers(&self) -> Vec<&PeerInfo> {
        self.known_peers.values().collect()
    }

    /// Get connected peers
    pub fn get_connected_peers(&self) -> Vec<&PeerInfo> {
        self.connected_peers
            .iter()
            .filter_map(|id| self.known_peers.get(id))
            .collect()
    }

    /// Get peers by capability
    pub fn get_peers_by_capability(&self, capability: &str) -> Vec<&PeerInfo> {
        self.known_peers
            .values()
            .filter(|peer| peer.capabilities.contains(&capability.to_string()))
            .collect()
    }

    /// Get discovery statistics
    pub fn get_stats(&self) -> &DiscoveryStats {
        &self.stats
    }

    /// Register event callback
    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(DiscoveryEvent) + Send + Sync + 'static,
    {
        self.event_callbacks.push(Box::new(callback));
    }

    /// Emit discovery event
    fn emit_event(&self, event: DiscoveryEvent) {
        for callback in &self.event_callbacks {
            callback(event.clone());
        }
    }

    /// Connect to a specific peer
    pub async fn connect_peer(&mut self, peer_id: &str) -> Result<()> {
        if let Some(peer) = self.known_peers.get(peer_id) {
            // Simulate connection
            debug!("Connecting to peer: {}", peer_id);
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            self.connected_peers.insert(peer_id.to_string());
            self.emit_event(DiscoveryEvent::PeerConnected(peer_id.to_string()));
            
            Ok(())
        } else {
            Err(anyhow!("Peer not found: {}", peer_id))
        }
    }

    /// Disconnect from a peer
    pub async fn disconnect_peer(&mut self, peer_id: &str) -> Result<()> {
        if self.connected_peers.remove(peer_id) {
            self.emit_event(DiscoveryEvent::PeerDisconnected(peer_id.to_string()));
            debug!("Disconnected from peer: {}", peer_id);
            Ok(())
        } else {
            Err(anyhow!("Peer not connected: {}", peer_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_peer_discovery_creation() {
        let config = DiscoveryConfig::default();
        let discovery = PeerDiscovery::new(config);
        
        assert_eq!(discovery.known_peers.len(), 0);
        assert_eq!(discovery.connected_peers.len(), 0);
    }

    #[tokio::test]
    async fn test_add_peer() {
        let config = DiscoveryConfig::default();
        let mut discovery = PeerDiscovery::new(config);
        
        let peer = PeerInfo {
            id: "test_peer".to_string(),
            address: "127.0.0.1:8080".parse().unwrap(),
            public_key: "test_key".to_string(),
            capabilities: vec!["agent".to_string()],
            reputation: 0.8,
            last_seen: chrono::Utc::now(),
            protocol_version: "1.0.0".to_string(),
            node_type: NodeType::Agent,
        };
        
        discovery.add_peer(peer, DiscoveryMethod::Manual).await;
        
        assert_eq!(discovery.known_peers.len(), 1);
        assert!(discovery.known_peers.contains_key("test_peer"));
    }

    #[tokio::test]
    async fn test_blacklist_peer() {
        let config = DiscoveryConfig::default();
        let mut discovery = PeerDiscovery::new(config);
        
        discovery.blacklist_peer("bad_peer");
        
        assert!(discovery.blacklisted_peers.contains("bad_peer"));
    }
} 