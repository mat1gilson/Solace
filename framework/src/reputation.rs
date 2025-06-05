//! Reputation system for agent trust scoring

use crate::{error::ReputationError, types::{AgentId, Timestamp}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reputation weight for different transaction types
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ReputationWeight {
    Low = 1,
    Medium = 3,
    High = 5,
    Critical = 10,
}

/// Individual reputation score for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    score: f64,
    total_transactions: u32,
    successful_transactions: u32,
    last_updated: Timestamp,
    history: Vec<ReputationEvent>,
}

impl ReputationScore {
    pub fn new(initial_score: f64) -> Self {
        Self {
            score: initial_score.clamp(0.0, 1.0),
            total_transactions: 0,
            successful_transactions: 0,
            last_updated: Timestamp::now(),
            history: Vec::new(),
        }
    }

    pub fn current_score(&self) -> f64 {
        self.score
    }

    pub fn update_score(&mut self, new_score: f64) {
        self.score = new_score.clamp(0.0, 1.0);
        self.last_updated = Timestamp::now();
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_transactions == 0 {
            0.0
        } else {
            self.successful_transactions as f64 / self.total_transactions as f64
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    pub timestamp: Timestamp,
    pub event_type: ReputationEventType,
    pub weight: ReputationWeight,
    pub delta: f64,
    pub counterparty: Option<AgentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationEventType {
    TransactionSuccess,
    TransactionFailure,
    TimeoutPenalty,
    QualityBonus,
    FraudPenalty,
}

/// Global reputation system
pub struct ReputationSystem {
    agent_scores: HashMap<AgentId, ReputationScore>,
}

impl ReputationSystem {
    pub fn new() -> Self {
        Self {
            agent_scores: HashMap::new(),
        }
    }

    pub fn get_score(&self, agent_id: &AgentId) -> Option<f64> {
        self.agent_scores.get(agent_id).map(|score| score.current_score())
    }

    pub fn update_reputation(&mut self, agent_id: AgentId, event: ReputationEvent) -> Result<f64, ReputationError> {
        let score = self.agent_scores.entry(agent_id).or_insert_with(|| ReputationScore::new(0.5));
        
        // Calculate new score based on event
        let weight_factor = match event.weight {
            ReputationWeight::Low => 0.01,
            ReputationWeight::Medium => 0.03,
            ReputationWeight::High => 0.05,
            ReputationWeight::Critical => 0.1,
        };

        let new_score = (score.score + event.delta * weight_factor).clamp(0.0, 1.0);
        score.update_score(new_score);
        score.history.push(event);

        Ok(new_score)
    }
} 