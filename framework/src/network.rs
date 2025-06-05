//! Network layer for peer-to-peer communication

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub listen_port: u16,
    pub max_connections: usize,
    pub heartbeat_interval: u64,
}

#[derive(Debug)]
pub struct P2PNetwork;

#[derive(Debug)]
pub struct PeerManager;