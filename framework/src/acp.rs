//! Autonomous Commerce Protocol (ACP) implementation

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersion(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    TransactionRequest,
    TransactionProposal,
    TransactionAcceptance,
    TransactionCompletion,
    ReputationUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACPMessage {
    pub message_type: MessageType,
    pub version: ProtocolVersion,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NegotiationStrategy {
    Conservative { max_rounds: u32, reputation_weight: crate::reputation::ReputationWeight, price_flexibility: f64 },
    Aggressive { max_rounds: u32, price_flexibility: f64 },
    Balanced { max_rounds: u32, reputation_weight: crate::reputation::ReputationWeight },
} 