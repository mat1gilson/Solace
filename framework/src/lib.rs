//! # Solace Protocol Framework
//!
//! A decentralized autonomous agent commerce framework built on Solana.
//! This library provides the core functionality for creating, managing, and
//! coordinating autonomous agents that can engage in commercial transactions.

pub mod agent;
pub mod acp;
pub mod crypto;
pub mod error;
pub mod network;
pub mod reputation;
pub mod transaction;
pub mod types;
pub mod utils;

// Re-export core types and functions
pub use agent::{Agent, AgentConfig, AgentCapability, AgentPreferences};
pub use acp::{ACPMessage, MessageType, NegotiationStrategy, ProtocolVersion};
pub use crypto::{KeyPair, Signature, SignatureError};
pub use error::{SolaceError, Result};
pub use network::{NetworkConfig, P2PNetwork, PeerManager};
pub use reputation::{ReputationScore, ReputationSystem, ReputationWeight};
pub use transaction::{
    Transaction, TransactionPhase, TransactionRequest, TransactionResult, TransactionStatus,
};
pub use types::{AgentId, Balance, Timestamp, TransactionId};

/// The current version of the Solace Protocol
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Default configuration constants
pub mod constants {
    use std::time::Duration;

    /// Maximum number of negotiation rounds
    pub const MAX_NEGOTIATION_ROUNDS: u32 = 10;

    /// Default transaction timeout
    pub const DEFAULT_TRANSACTION_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

    /// Minimum reputation score for participation
    pub const MIN_REPUTATION_SCORE: f64 = 0.1;

    /// Maximum transaction value for new agents
    pub const MAX_NEW_AGENT_TRANSACTION: u64 = 1000; // SOL lamports

    /// Network heartbeat interval
    pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

    /// Default RPC endpoint for devnet
    pub const DEFAULT_DEVNET_RPC: &str = "https://api.devnet.solana.com";

    /// Default RPC endpoint for mainnet
    pub const DEFAULT_MAINNET_RPC: &str = "https://api.mainnet-beta.solana.com";
}

/// Initialize the Solace Protocol with logging and configuration
pub fn init() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Solace Protocol v{} initialized", PROTOCOL_VERSION);
    Ok(())
}

/// Create a new agent with the specified configuration
pub async fn create_agent(config: AgentConfig) -> Result<Agent> {
    Agent::new(config).await
}

/// Validate protocol version compatibility
pub fn is_compatible_version(version: &str) -> bool {
    // Implement semantic versioning compatibility check
    let current_parts: Vec<u32> = PROTOCOL_VERSION
        .split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    let check_parts: Vec<u32> = version.split('.').map(|s| s.parse().unwrap_or(0)).collect();

    if current_parts.len() != 3 || check_parts.len() != 3 {
        return false;
    }

    // Major version must match
    current_parts[0] == check_parts[0] &&
    // Minor version must be >= current
    (current_parts[1] <= check_parts[1] || 
     (current_parts[1] == check_parts[1] && current_parts[2] <= check_parts[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        assert!(is_compatible_version("1.0.0"));
        assert!(is_compatible_version("1.0.1"));
        assert!(is_compatible_version("1.1.0"));
        assert!(!is_compatible_version("2.0.0"));
        assert!(!is_compatible_version("0.9.0"));
    }

    #[tokio::test]
    async fn test_protocol_initialization() {
        let result = init();
        assert!(result.is_ok());
    }
} 