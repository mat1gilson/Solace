//! Core messaging system for ACP
//!
//! This module defines the message formats, types, and handling mechanisms
//! used throughout the Autonomous Commerce Protocol.

use crate::{ACPError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Message types supported by ACP
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    /// Transaction request message
    TransactionRequest,
    /// Transaction proposal response
    TransactionProposal,
    /// Transaction acceptance/rejection
    TransactionResponse,
    /// Transaction completion notification
    TransactionComplete,
    /// Reputation update message
    ReputationUpdate,
    /// Heartbeat/keep-alive message
    Heartbeat,
    /// Peer discovery message
    PeerDiscovery,
    /// Gossip protocol message
    Gossip,
    /// Protocol handshake message
    Handshake,
    /// Custom message type
    Custom(String),
}

/// Core ACP message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACPMessage {
    /// Unique message identifier
    pub id: Uuid,
    /// Message type
    pub message_type: MessageType,
    /// Source node identifier
    pub from: String,
    /// Destination node identifier (optional for broadcasts)
    pub to: Option<String>,
    /// Message timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Protocol version
    pub version: String,
    /// Message payload
    pub payload: Vec<u8>,
    /// Message headers
    pub headers: HashMap<String, String>,
    /// Digital signature
    pub signature: Option<Vec<u8>>,
}

impl ACPMessage {
    /// Create a new ACP message
    pub fn new(
        message_type: MessageType,
        from: String,
        to: Option<String>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            message_type,
            from,
            to,
            timestamp: chrono::Utc::now(),
            version: crate::ACP_VERSION.to_string(),
            payload,
            headers: HashMap::new(),
            signature: None,
        }
    }

    /// Add a header to the message
    pub fn add_header<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.headers.insert(key.into(), value.into());
    }

    /// Get a header value
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Set the message signature
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = Some(signature);
    }

    /// Check if message is signed
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Serialize the message for transmission
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| ACPError::Message(format!("Serialization failed: {}", e)))
    }

    /// Deserialize a message from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data).map_err(|e| ACPError::Message(format!("Deserialization failed: {}", e)))
    }

    /// Get message size in bytes
    pub fn size(&self) -> usize {
        self.serialize().map(|data| data.len()).unwrap_or(0)
    }

    /// Check if message is expired based on TTL header
    pub fn is_expired(&self) -> bool {
        if let Some(ttl_str) = self.get_header("ttl") {
            if let Ok(ttl_seconds) = ttl_str.parse::<i64>() {
                let expiry = self.timestamp + chrono::Duration::seconds(ttl_seconds);
                return chrono::Utc::now() > expiry;
            }
        }
        false
    }

    /// Create a response message
    pub fn create_response(&self, response_type: MessageType, payload: Vec<u8>) -> ACPMessage {
        let mut response = ACPMessage::new(
            response_type,
            self.to.clone().unwrap_or_default(),
            Some(self.from.clone()),
            payload,
        );
        
        // Add correlation ID
        response.add_header("correlation_id", self.id.to_string());
        
        response
    }
}

/// Message handler trait
pub trait MessageHandler: Send + Sync {
    /// Handle an incoming message
    fn handle(&self, message: ACPMessage) -> Result<Option<ACPMessage>>;
    
    /// Get the message types this handler can process
    fn message_types(&self) -> Vec<MessageType>;
}

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

/// Priority message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityMessage {
    pub message: ACPMessage,
    pub priority: MessagePriority,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl PriorityMessage {
    /// Create a new priority message
    pub fn new(message: ACPMessage, priority: MessagePriority) -> Self {
        Self {
            message,
            priority,
            retry_count: 0,
            max_retries: 3,
        }
    }

    /// Check if message can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Message queue for handling prioritized messages
pub struct MessageQueue {
    messages: std::sync::RwLock<std::collections::BinaryHeap<PriorityMessage>>,
}

impl MessageQueue {
    /// Create a new message queue
    pub fn new() -> Self {
        Self {
            messages: std::sync::RwLock::new(std::collections::BinaryHeap::new()),
        }
    }

    /// Add a message to the queue
    pub fn push(&self, message: PriorityMessage) -> Result<()> {
        let mut queue = self.messages.write().unwrap();
        queue.push(message);
        Ok(())
    }

    /// Get the next highest priority message
    pub fn pop(&self) -> Option<PriorityMessage> {
        let mut queue = self.messages.write().unwrap();
        queue.pop()
    }

    /// Get queue size
    pub fn len(&self) -> usize {
        let queue = self.messages.read().unwrap();
        queue.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        let queue = self.messages.read().unwrap();
        queue.is_empty()
    }
}

impl Ord for PriorityMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for PriorityMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PriorityMessage {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.message.id == other.message.id
    }
}

impl Eq for PriorityMessage {}

/// Specialized message types for common operations
pub mod messages {
    use super::*;

    /// Transaction request message payload
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TransactionRequestPayload {
        pub transaction_id: Uuid,
        pub service_type: String,
        pub budget: f64,
        pub deadline: chrono::DateTime<chrono::Utc>,
        pub requirements: HashMap<String, serde_json::Value>,
    }

    /// Transaction proposal message payload
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TransactionProposalPayload {
        pub transaction_id: Uuid,
        pub proposal_id: Uuid,
        pub provider_id: String,
        pub proposed_price: f64,
        pub estimated_completion: chrono::DateTime<chrono::Utc>,
        pub terms: HashMap<String, serde_json::Value>,
    }

    /// Reputation update message payload
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ReputationUpdatePayload {
        pub agent_id: String,
        pub transaction_id: Uuid,
        pub rating: f64,
        pub feedback: String,
        pub metrics: HashMap<String, f64>,
    }

    /// Helper functions for creating common message types
    impl ACPMessage {
        /// Create a transaction request message
        pub fn transaction_request(
            from: String,
            to: Option<String>,
            payload: TransactionRequestPayload,
        ) -> Result<Self> {
            let serialized = serde_json::to_vec(&payload)
                .map_err(|e| ACPError::Message(format!("Failed to serialize payload: {}", e)))?;
            
            Ok(ACPMessage::new(
                MessageType::TransactionRequest,
                from,
                to,
                serialized,
            ))
        }

        /// Create a transaction proposal message
        pub fn transaction_proposal(
            from: String,
            to: String,
            payload: TransactionProposalPayload,
        ) -> Result<Self> {
            let serialized = serde_json::to_vec(&payload)
                .map_err(|e| ACPError::Message(format!("Failed to serialize payload: {}", e)))?;
            
            Ok(ACPMessage::new(
                MessageType::TransactionProposal,
                from,
                Some(to),
                serialized,
            ))
        }

        /// Create a reputation update message
        pub fn reputation_update(
            from: String,
            payload: ReputationUpdatePayload,
        ) -> Result<Self> {
            let serialized = serde_json::to_vec(&payload)
                .map_err(|e| ACPError::Message(format!("Failed to serialize payload: {}", e)))?;
            
            Ok(ACPMessage::new(
                MessageType::ReputationUpdate,
                from,
                None, // Broadcast
                serialized,
            ))
        }

        /// Create a heartbeat message
        pub fn heartbeat(from: String) -> Self {
            ACPMessage::new(
                MessageType::Heartbeat,
                from,
                None,
                Vec::new(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let message = ACPMessage::new(
            MessageType::Heartbeat,
            "node1".to_string(),
            None,
            vec![1, 2, 3],
        );

        assert_eq!(message.message_type, MessageType::Heartbeat);
        assert_eq!(message.from, "node1");
        assert_eq!(message.payload, vec![1, 2, 3]);
        assert!(!message.is_signed());
    }

    #[test]
    fn test_message_serialization() {
        let message = ACPMessage::new(
            MessageType::Heartbeat,
            "node1".to_string(),
            None,
            vec![1, 2, 3],
        );

        let serialized = message.serialize().unwrap();
        let deserialized = ACPMessage::deserialize(&serialized).unwrap();

        assert_eq!(message.id, deserialized.id);
        assert_eq!(message.message_type, deserialized.message_type);
        assert_eq!(message.payload, deserialized.payload);
    }

    #[test]
    fn test_message_headers() {
        let mut message = ACPMessage::new(
            MessageType::Custom("test".to_string()),
            "node1".to_string(),
            None,
            Vec::new(),
        );

        message.add_header("key1", "value1");
        message.add_header("key2", "value2");

        assert_eq!(message.get_header("key1"), Some(&"value1".to_string()));
        assert_eq!(message.get_header("key2"), Some(&"value2".to_string()));
        assert_eq!(message.get_header("key3"), None);
    }

    #[test]
    fn test_message_queue() {
        let queue = MessageQueue::new();
        
        let msg1 = PriorityMessage::new(
            ACPMessage::new(MessageType::Heartbeat, "node1".to_string(), None, Vec::new()),
            MessagePriority::Low,
        );
        
        let msg2 = PriorityMessage::new(
            ACPMessage::new(MessageType::TransactionRequest, "node2".to_string(), None, Vec::new()),
            MessagePriority::High,
        );

        queue.push(msg1).unwrap();
        queue.push(msg2).unwrap();

        assert_eq!(queue.len(), 2);

        // Should pop the high priority message first
        let popped = queue.pop().unwrap();
        assert_eq!(popped.priority, MessagePriority::High);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_message_expiry() {
        let mut message = ACPMessage::new(
            MessageType::Heartbeat,
            "node1".to_string(),
            None,
            Vec::new(),
        );

        // Message without TTL should not be expired
        assert!(!message.is_expired());

        // Add expired TTL
        message.add_header("ttl", "1");
        std::thread::sleep(std::time::Duration::from_secs(2));
        assert!(message.is_expired());
    }
}