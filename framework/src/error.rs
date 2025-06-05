//! Error types and handling for the Solace Protocol

use thiserror::Error;

/// Main result type for the Solace Protocol
pub type Result<T> = std::result::Result<T, SolaceError>;

/// Comprehensive error types for the Solace Protocol
#[derive(Error, Debug)]
pub enum SolaceError {
    /// Agent-related errors
    #[error("Agent error: {0}")]
    Agent(#[from] AgentError),

    /// Transaction-related errors
    #[error("Transaction error: {0}")]
    Transaction(#[from] TransactionError),

    /// Network-related errors
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// Cryptographic errors
    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),

    /// Reputation system errors
    #[error("Reputation error: {0}")]
    Reputation(#[from] ReputationError),

    /// Solana blockchain errors
    #[error("Solana error: {0}")]
    Solana(#[from] solana_client::client_error::ClientError),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Protocol version mismatch
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    /// Generic internal error
    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Agent-specific errors
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent not found: {id}")]
    NotFound { id: String },

    #[error("Agent already exists: {id}")]
    AlreadyExists { id: String },

    #[error("Agent not authorized for operation: {operation}")]
    NotAuthorized { operation: String },

    #[error("Agent configuration invalid: {reason}")]
    InvalidConfig { reason: String },

    #[error("Agent reputation too low: {current}, minimum required: {required}")]
    ReputationTooLow { current: f64, required: f64 },

    #[error("Agent capabilities insufficient for request")]
    InsufficientCapabilities,

    #[error("Agent wallet insufficient funds: {available}, required: {required}")]
    InsufficientFunds { available: u64, required: u64 },

    #[error("Agent is currently offline")]
    Offline,
}

/// Transaction-specific errors
#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("Transaction not found: {id}")]
    NotFound { id: String },

    #[error("Transaction already exists: {id}")]
    AlreadyExists { id: String },

    #[error("Transaction in invalid state: {current}, expected: {expected}")]
    InvalidState { current: String, expected: String },

    #[error("Transaction expired at {deadline}")]
    Expired { deadline: String },

    #[error("Transaction amount invalid: {amount}")]
    InvalidAmount { amount: u64 },

    #[error("Transaction signature invalid")]
    InvalidSignature,

    #[error("Transaction negotiation failed after {rounds} rounds")]
    NegotiationFailed { rounds: u32 },

    #[error("Transaction execution failed: {reason}")]
    ExecutionFailed { reason: String },

    #[error("Transaction timeout after {duration} seconds")]
    Timeout { duration: u64 },
}

/// Network-specific errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed to {address}: {reason}")]
    ConnectionFailed { address: String, reason: String },

    #[error("Connection timeout to {address}")]
    ConnectionTimeout { address: String },

    #[error("Peer not found: {peer_id}")]
    PeerNotFound { peer_id: String },

    #[error("Invalid message format")]
    InvalidMessage,

    #[error("Message too large: {size} bytes, maximum: {max}")]
    MessageTooLarge { size: usize, max: usize },

    #[error("Network partition detected")]
    NetworkPartition,

    #[error("Bandwidth limit exceeded")]
    BandwidthExceeded,

    #[error("Protocol handshake failed with {peer}")]
    HandshakeFailed { peer: String },
}

/// Cryptographic errors
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid key format")]
    InvalidKeyFormat,

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Key generation failed")]
    KeyGenerationFailed,

    #[error("Encryption failed")]
    EncryptionFailed,

    #[error("Decryption failed")]
    DecryptionFailed,

    #[error("Hash computation failed")]
    HashComputationFailed,

    #[error("Random number generation failed")]
    RandomGenerationFailed,
}

/// Reputation system errors
#[derive(Error, Debug)]
pub enum ReputationError {
    #[error("Reputation score out of range: {score}, valid range: 0.0-1.0")]
    ScoreOutOfRange { score: f64 },

    #[error("Insufficient reputation history for agent: {agent_id}")]
    InsufficientHistory { agent_id: String },

    #[error("Reputation calculation failed: {reason}")]
    CalculationFailed { reason: String },

    #[error("Reputation update denied: {reason}")]
    UpdateDenied { reason: String },

    #[error("Reputation system not initialized")]
    NotInitialized,
}

impl SolaceError {
    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            SolaceError::Network(NetworkError::ConnectionTimeout { .. }) => true,
            SolaceError::Network(NetworkError::ConnectionFailed { .. }) => true,
            SolaceError::Network(NetworkError::BandwidthExceeded) => true,
            SolaceError::Transaction(TransactionError::Timeout { .. }) => true,
            SolaceError::Solana(_) => true, // Blockchain issues might be temporary
            _ => false,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            SolaceError::Internal { .. } => ErrorSeverity::Critical,
            SolaceError::Crypto(_) => ErrorSeverity::High,
            SolaceError::Agent(AgentError::NotAuthorized { .. }) => ErrorSeverity::High,
            SolaceError::Transaction(TransactionError::InvalidSignature) => ErrorSeverity::High,
            SolaceError::Network(NetworkError::NetworkPartition) => ErrorSeverity::High,
            SolaceError::VersionMismatch { .. } => ErrorSeverity::Medium,
            SolaceError::Config { .. } => ErrorSeverity::Medium,
            _ => ErrorSeverity::Low,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        let timeout_error = SolaceError::Network(NetworkError::ConnectionTimeout {
            address: "test".to_string(),
        });
        assert!(timeout_error.is_retryable());

        let config_error = SolaceError::config("test error");
        assert!(!config_error.is_retryable());
    }

    #[test]
    fn test_error_severity() {
        let internal_error = SolaceError::internal("test");
        assert_eq!(internal_error.severity(), ErrorSeverity::Critical);

        let config_error = SolaceError::config("test");
        assert_eq!(config_error.severity(), ErrorSeverity::Medium);
    }

    #[test]
    fn test_agent_error_conversion() {
        let agent_error = AgentError::NotFound {
            id: "test".to_string(),
        };
        let solace_error: SolaceError = agent_error.into();
        
        match solace_error {
            SolaceError::Agent(AgentError::NotFound { .. }) => (),
            _ => panic!("Expected Agent error"),
        }
    }
} 