//! Transaction handling for autonomous commerce

use crate::{
    crypto::Signature,
    error::{Result, TransactionError},
    types::{AgentId, Balance, ServiceType, Timestamp, TransactionId},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transaction phases in the commerce lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionPhase {
    Request,
    Negotiation,
    Execution,
    Evaluation,
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Expired,
}

/// Transaction request from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequest {
    pub id: TransactionId,
    pub requester: AgentId,
    pub service_type: ServiceType,
    pub description: String,
    pub budget: Balance,
    pub deadline: Timestamp,
    pub requirements: HashMap<String, String>,
    pub created_at: Timestamp,
}

impl TransactionRequest {
    pub fn new(
        requester: AgentId,
        service_type: ServiceType,
        description: String,
        budget: Balance,
        deadline: Timestamp,
    ) -> Self {
        Self {
            id: TransactionId::new(),
            requester,
            service_type,
            description,
            budget,
            deadline,
            requirements: HashMap::new(),
            created_at: Timestamp::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.deadline.is_past()
    }
}

/// Transaction proposal from a service provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionProposal {
    pub id: TransactionId,
    pub request_id: TransactionId,
    pub provider: AgentId,
    pub proposed_price: Balance,
    pub estimated_completion: Timestamp,
    pub proposal_details: String,
    pub terms: HashMap<String, String>,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

/// Core transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub request: TransactionRequest,
    pub provider: Option<AgentId>,
    pub agreed_price: Option<Balance>,
    pub phase: TransactionPhase,
    pub status: TransactionStatus,
    pub proposals: Vec<TransactionProposal>,
    pub negotiation_rounds: u32,
    pub signatures: HashMap<AgentId, Signature>,
    pub execution_data: Option<ExecutionData>,
    pub evaluation: Option<TransactionEvaluation>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Transaction {
    pub fn new(request: TransactionRequest) -> Self {
        Self {
            id: request.id,
            request,
            provider: None,
            agreed_price: None,
            phase: TransactionPhase::Request,
            status: TransactionStatus::Pending,
            proposals: Vec::new(),
            negotiation_rounds: 0,
            signatures: HashMap::new(),
            execution_data: None,
            evaluation: None,
            created_at: Timestamp::now(),
            updated_at: Timestamp::now(),
        }
    }

    pub fn add_proposal(&mut self, proposal: TransactionProposal) -> Result<()> {
        if self.phase != TransactionPhase::Request && self.phase != TransactionPhase::Negotiation {
            return Err(TransactionError::InvalidState {
                current: format!("{:?}", self.phase),
                expected: "Request or Negotiation".to_string(),
            }.into());
        }

        self.proposals.push(proposal);
        self.phase = TransactionPhase::Negotiation;
        self.updated_at = Timestamp::now();
        Ok(())
    }

    pub fn accept_proposal(&mut self, provider_id: AgentId, price: Balance) -> Result<()> {
        if self.phase != TransactionPhase::Negotiation {
            return Err(TransactionError::InvalidState {
                current: format!("{:?}", self.phase),
                expected: "Negotiation".to_string(),
            }.into());
        }

        self.provider = Some(provider_id);
        self.agreed_price = Some(price);
        self.phase = TransactionPhase::Execution;
        self.status = TransactionStatus::InProgress;
        self.updated_at = Timestamp::now();
        Ok(())
    }

    pub fn complete_execution(&mut self, execution_data: ExecutionData) -> Result<()> {
        if self.phase != TransactionPhase::Execution {
            return Err(TransactionError::InvalidState {
                current: format!("{:?}", self.phase),
                expected: "Execution".to_string(),
            }.into());
        }

        self.execution_data = Some(execution_data);
        self.phase = TransactionPhase::Evaluation;
        self.updated_at = Timestamp::now();
        Ok(())
    }

    pub fn add_evaluation(&mut self, evaluation: TransactionEvaluation) -> Result<()> {
        if self.phase != TransactionPhase::Evaluation {
            return Err(TransactionError::InvalidState {
                current: format!("{:?}", self.phase),
                expected: "Evaluation".to_string(),
            }.into());
        }

        self.evaluation = Some(evaluation);
        self.status = TransactionStatus::Completed;
        self.updated_at = Timestamp::now();
        Ok(())
    }
}

/// Execution data containing results and proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionData {
    pub result: String,
    pub artifacts: Vec<String>,
    pub completion_time: Timestamp,
    pub quality_metrics: HashMap<String, f64>,
}

/// Transaction evaluation from both parties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEvaluation {
    pub requester_rating: f64,
    pub provider_rating: f64,
    pub requester_feedback: String,
    pub provider_feedback: String,
    pub quality_score: f64,
    pub timeliness_score: f64,
    pub overall_satisfaction: f64,
}

/// Transaction result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub transaction_id: TransactionId,
    pub status: TransactionStatus,
    pub final_price: Option<Balance>,
    pub completion_time: Option<Timestamp>,
    pub quality_score: Option<f64>,
    pub reputation_delta: HashMap<AgentId, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_lifecycle() {
        let requester = AgentId::new();
        let provider = AgentId::new();
        
        let request = TransactionRequest::new(
            requester,
            ServiceType::DataAnalysis,
            "Test request".to_string(),
            Balance::from_sol(10.0),
            Timestamp::now(),
        );

        let mut transaction = Transaction::new(request);
        assert_eq!(transaction.phase, TransactionPhase::Request);
        assert_eq!(transaction.status, TransactionStatus::Pending);

        // Add proposal
        let proposal = TransactionProposal {
            id: TransactionId::new(),
            request_id: transaction.id,
            provider,
            proposed_price: Balance::from_sol(8.0),
            estimated_completion: Timestamp::now(),
            proposal_details: "Test proposal".to_string(),
            terms: HashMap::new(),
            created_at: Timestamp::now(),
            expires_at: Timestamp::now(),
        };

        transaction.add_proposal(proposal).unwrap();
        assert_eq!(transaction.phase, TransactionPhase::Negotiation);

        // Accept proposal
        transaction.accept_proposal(provider, Balance::from_sol(8.0)).unwrap();
        assert_eq!(transaction.phase, TransactionPhase::Execution);
        assert_eq!(transaction.status, TransactionStatus::InProgress);
    }
} 