//! # Autonomous Commerce Protocol (ACP)
//!
//! The ACP is the core messaging and coordination layer for the Solace Protocol.
//! It defines the communication standards, message formats, and coordination
//! mechanisms for autonomous agent interactions.

pub mod messaging;
pub mod discovery;
pub mod gossip;
pub mod p2p;
pub mod protocol;
pub mod routing;
pub mod security;

pub use messaging::{ACPMessage, MessageType, MessageHandler};
pub use discovery::{PeerDiscovery, NodeInfo};
pub use gossip::{GossipProtocol, GossipMessage};
pub use p2p::{P2PNetwork, ConnectionManager};
pub use protocol::{ProtocolVersion, HandshakeManager};
pub use routing::{MessageRouter, RoutingTable};
pub use security::{SecurityManager, MessageAuthentication};

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// ACP Protocol version
pub const ACP_VERSION: &str = "1.0.0";

/// Default configuration constants
pub mod constants {
    use super::Duration;

    /// Maximum message size in bytes
    pub const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB

    /// Default heartbeat interval
    pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

    /// Maximum number of peers to maintain
    pub const MAX_PEERS: usize = 100;

    /// Message timeout duration
    pub const MESSAGE_TIMEOUT: Duration = Duration::from_secs(30);

    /// Discovery broadcast interval
    pub const DISCOVERY_INTERVAL: Duration = Duration::from_secs(60);

    /// Gossip propagation factor
    pub const GOSSIP_FACTOR: usize = 3;
}

/// ACP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACPConfig {
    /// Node identifier
    pub node_id: String,
    /// Listen address for incoming connections
    pub listen_address: String,
    /// Bootstrap peers for initial connection
    pub bootstrap_peers: Vec<String>,
    /// Maximum number of peers to maintain
    pub max_peers: usize,
    /// Enable gossip protocol
    pub enable_gossip: bool,
    /// Enable peer discovery
    pub enable_discovery: bool,
    /// Message timeout duration
    pub message_timeout: Duration,
}

impl Default for ACPConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            listen_address: "0.0.0.0:8080".to_string(),
            bootstrap_peers: Vec::new(),
            max_peers: constants::MAX_PEERS,
            enable_gossip: true,
            enable_discovery: true,
            message_timeout: constants::MESSAGE_TIMEOUT,
        }
    }
}

/// ACP Error types
#[derive(Error, Debug)]
pub enum ACPError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Message error: {0}")]
    Message(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Discovery error: {0}")]
    Discovery(String),
}

/// ACP Result type
pub type Result<T> = std::result::Result<T, ACPError>;

/// Main ACP coordinator
pub struct ACP {
    config: ACPConfig,
    network: P2PNetwork,
    discovery: PeerDiscovery,
    gossip: GossipProtocol,
    router: MessageRouter,
    security: SecurityManager,
}

impl ACP {
    /// Create a new ACP instance
    pub async fn new(config: ACPConfig) -> Result<Self> {
        let network = P2PNetwork::new(&config).await?;
        let discovery = PeerDiscovery::new(&config);
        let gossip = GossipProtocol::new(&config);
        let router = MessageRouter::new();
        let security = SecurityManager::new();

        Ok(Self {
            config,
            network,
            discovery,
            gossip,
            router,
            security,
        })
    }

    /// Start the ACP coordinator
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Starting ACP coordinator v{}", ACP_VERSION);

        // Start network layer
        self.network.start().await?;

        // Start discovery if enabled
        if self.config.enable_discovery {
            self.discovery.start().await?;
        }

        // Start gossip if enabled
        if self.config.enable_gossip {
            self.gossip.start().await?;
        }

        // Initialize routing
        self.router.start().await?;

        tracing::info!("ACP coordinator started successfully");
        Ok(())
    }

    /// Stop the ACP coordinator
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("Stopping ACP coordinator");

        self.gossip.stop().await?;
        self.discovery.stop().await?;
        self.network.stop().await?;

        tracing::info!("ACP coordinator stopped");
        Ok(())
    }

    /// Send a message to a specific peer
    pub async fn send_message(&self, peer_id: &str, message: ACPMessage) -> Result<()> {
        // Authenticate and sign the message
        let signed_message = self.security.sign_message(message)?;
        
        // Route the message
        self.router.route_message(peer_id, signed_message).await
    }

    /// Broadcast a message to all peers
    pub async fn broadcast_message(&self, message: ACPMessage) -> Result<()> {
        // Use gossip protocol for efficient broadcasting
        self.gossip.broadcast(message).await
    }

    /// Register a message handler
    pub fn register_handler<F>(&mut self, message_type: MessageType, handler: F)
    where
        F: Fn(ACPMessage) -> Result<()> + Send + Sync + 'static,
    {
        self.router.register_handler(message_type, Box::new(handler));
    }

    /// Get current peer count
    pub fn peer_count(&self) -> usize {
        self.network.peer_count()
    }

    /// Get ACP statistics
    pub fn get_stats(&self) -> ACPStats {
        ACPStats {
            peer_count: self.peer_count(),
            messages_sent: self.router.messages_sent(),
            messages_received: self.router.messages_received(),
            uptime: self.network.uptime(),
        }
    }
}

/// ACP statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACPStats {
    pub peer_count: usize,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub uptime: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acp_creation() {
        let config = ACPConfig::default();
        let result = ACP::new(config).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_default() {
        let config = ACPConfig::default();
        assert_eq!(config.max_peers, constants::MAX_PEERS);
        assert!(config.enable_gossip);
        assert!(config.enable_discovery);
    }

    #[test]
    fn test_constants() {
        assert_eq!(constants::MAX_MESSAGE_SIZE, 1024 * 1024);
        assert_eq!(constants::MAX_PEERS, 100);
        assert_eq!(constants::GOSSIP_FACTOR, 3);
    }
} 