//! Core types used throughout the Solace Protocol

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    /// Generate a new random agent ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create an agent ID from a string
    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(pub Uuid);

impl TransactionId {
    /// Generate a new random transaction ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TransactionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Balance representation in lamports (1 SOL = 1,000,000,000 lamports)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Balance(pub u64);

impl Balance {
    /// Create a new balance
    pub fn new(lamports: u64) -> Self {
        Self(lamports)
    }

    /// Create balance from SOL amount
    pub fn from_sol(sol: f64) -> Self {
        Self((sol * 1_000_000_000.0) as u64)
    }

    /// Convert to SOL amount
    pub fn to_sol(&self) -> f64 {
        self.0 as f64 / 1_000_000_000.0
    }

    /// Check if balance is zero
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Add two balances
    pub fn add(&self, other: Balance) -> Option<Balance> {
        self.0.checked_add(other.0).map(Balance)
    }

    /// Subtract two balances
    pub fn sub(&self, other: Balance) -> Option<Balance> {
        self.0.checked_sub(other.0).map(Balance)
    }
}

impl fmt::Display for Balance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.6} SOL", self.to_sol())
    }
}

/// Timestamp type for consistent time handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(pub DateTime<Utc>);

impl Timestamp {
    /// Get current timestamp
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// Create timestamp from Unix timestamp
    pub fn from_unix(timestamp: i64) -> Option<Self> {
        DateTime::from_timestamp(timestamp, 0).map(Self)
    }

    /// Convert to Unix timestamp
    pub fn to_unix(&self) -> i64 {
        self.0.timestamp()
    }

    /// Check if timestamp is in the future
    pub fn is_future(&self) -> bool {
        self.0 > Utc::now()
    }

    /// Check if timestamp is in the past
    pub fn is_past(&self) -> bool {
        self.0 < Utc::now()
    }

    /// Duration since this timestamp
    pub fn elapsed(&self) -> chrono::Duration {
        Utc::now() - self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d %H:%M:%S UTC"))
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

/// Network address for peer communication
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkAddress {
    pub host: String,
    pub port: u16,
}

impl NetworkAddress {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }
}

impl fmt::Display for NetworkAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// Service type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    DataAnalysis,
    ComputationalTask,
    MarketResearch,
    ContentCreation,
    TradingService,
    CustomService(String),
}

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceType::DataAnalysis => write!(f, "Data Analysis"),
            ServiceType::ComputationalTask => write!(f, "Computational Task"),
            ServiceType::MarketResearch => write!(f, "Market Research"),
            ServiceType::ContentCreation => write!(f, "Content Creation"),
            ServiceType::TradingService => write!(f, "Trading Service"),
            ServiceType::CustomService(name) => write!(f, "Custom: {}", name),
        }
    }
}

/// Priority levels for transactions and messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Wallet information for blockchain operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub public_key: Pubkey,
    pub balance: Balance,
    pub last_updated: Timestamp,
}

impl WalletInfo {
    pub fn new(public_key: Pubkey, balance: Balance) -> Self {
        Self {
            public_key,
            balance,
            last_updated: Timestamp::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_creation() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_balance_operations() {
        let balance1 = Balance::from_sol(1.5);
        let balance2 = Balance::from_sol(0.5);
        
        assert_eq!(balance1.to_sol(), 1.5);
        
        let sum = balance1.add(balance2).unwrap();
        assert_eq!(sum.to_sol(), 2.0);
        
        let diff = balance1.sub(balance2).unwrap();
        assert_eq!(diff.to_sol(), 1.0);
    }

    #[test]
    fn test_timestamp_operations() {
        let ts = Timestamp::now();
        assert!(!ts.is_future());
        
        // Test past timestamp
        let past_ts = Timestamp::from_unix(1000000).unwrap();
        assert!(past_ts.is_past());
        assert!(past_ts.elapsed().num_seconds() > 0);
    }
} 