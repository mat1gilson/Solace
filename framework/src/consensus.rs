//! Consensus Mechanism for Solace Protocol
//!
//! Implements a Proof-of-Reputation consensus algorithm specifically designed
//! for autonomous agent networks. This consensus mechanism considers agent
//! reputation, stake, and participation history to determine block producers.

use std::collections::{HashMap, BTreeMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::{info, warn, debug, error};

use crate::{AgentId, types::Hash, error::SolaceError, crypto::Signature};

/// Consensus configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Block time in seconds
    pub block_time: Duration,
    /// Number of validators per epoch
    pub validators_per_epoch: usize,
    /// Minimum stake required to be a validator
    pub min_validator_stake: u64,
    /// Reputation weight in validator selection
    pub reputation_weight: f64,
    /// Stake weight in validator selection
    pub stake_weight: f64,
    /// Maximum number of blocks a validator can produce consecutively
    pub max_consecutive_blocks: u32,
    /// Epoch duration in blocks
    pub epoch_duration: u32,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            block_time: Duration::from_secs(2),
            validators_per_epoch: 21,
            min_validator_stake: 1000,
            reputation_weight: 0.4,
            stake_weight: 0.6,
            max_consecutive_blocks: 3,
            epoch_duration: 1000,
        }
    }
}

/// Validator information for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub agent_id: AgentId,
    pub stake: u64,
    pub reputation: f64,
    pub blocks_produced: u32,
    pub consecutive_blocks: u32,
    pub last_block_time: SystemTime,
    pub is_active: bool,
    pub slashing_events: u32,
}

impl Validator {
    pub fn new(agent_id: AgentId, stake: u64, reputation: f64) -> Self {
        Self {
            agent_id,
            stake,
            reputation,
            blocks_produced: 0,
            consecutive_blocks: 0,
            last_block_time: UNIX_EPOCH,
            is_active: true,
            slashing_events: 0,
        }
    }

    /// Calculate validator weight for selection
    pub fn calculate_weight(&self, config: &ConsensusConfig) -> f64 {
        if !self.is_active || self.stake < config.min_validator_stake {
            return 0.0;
        }

        let stake_normalized = self.stake as f64 / 1_000_000.0; // Normalize to millions
        let reputation_component = self.reputation * config.reputation_weight;
        let stake_component = stake_normalized.ln() * config.stake_weight;
        
        // Apply penalties
        let consecutive_penalty = if self.consecutive_blocks >= config.max_consecutive_blocks {
            0.5
        } else {
            1.0
        };
        
        let slashing_penalty = 0.9_f64.powi(self.slashing_events as i32);
        
        (reputation_component + stake_component) * consecutive_penalty * slashing_penalty
    }
}

/// Block header for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub previous_hash: Hash,
    pub merkle_root: Hash,
    pub timestamp: SystemTime,
    pub producer: AgentId,
    pub epoch: u32,
    pub nonce: u64,
}

/// Consensus vote for block validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusVote {
    pub block_hash: Hash,
    pub block_height: u64,
    pub voter: AgentId,
    pub vote_type: VoteType,
    pub timestamp: SystemTime,
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VoteType {
    Approve,
    Reject,
    Abstain,
}

/// Epoch information for validator rotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Epoch {
    pub number: u32,
    pub start_block: u64,
    pub end_block: u64,
    pub validators: Vec<AgentId>,
    pub block_producers: BTreeMap<u64, AgentId>,
}

/// Consensus engine implementation
pub struct ConsensusEngine {
    config: ConsensusConfig,
    validators: HashMap<AgentId, Validator>,
    current_epoch: Epoch,
    pending_votes: HashMap<Hash, Vec<ConsensusVote>>,
    block_history: VecDeque<BlockHeader>,
    validator_performance: HashMap<AgentId, ValidatorPerformance>,
}

#[derive(Debug, Clone, Default)]
struct ValidatorPerformance {
    blocks_produced: u32,
    blocks_missed: u32,
    votes_cast: u32,
    votes_missed: u32,
    uptime_percentage: f64,
}

impl ConsensusEngine {
    /// Create a new consensus engine
    pub fn new(config: ConsensusConfig) -> Self {
        Self {
            config,
            validators: HashMap::new(),
            current_epoch: Epoch {
                number: 0,
                start_block: 0,
                end_block: config.epoch_duration as u64,
                validators: Vec::new(),
                block_producers: BTreeMap::new(),
            },
            pending_votes: HashMap::new(),
            block_history: VecDeque::new(),
            validator_performance: HashMap::new(),
        }
    }

    /// Register a new validator
    pub fn register_validator(&mut self, agent_id: AgentId, stake: u64, reputation: f64) -> Result<()> {
        if stake < self.config.min_validator_stake {
            return Err(SolaceError::InsufficientStake(stake, self.config.min_validator_stake).into());
        }

        let validator = Validator::new(agent_id.clone(), stake, reputation);
        self.validators.insert(agent_id.clone(), validator);
        self.validator_performance.insert(agent_id, ValidatorPerformance::default());

        info!("Registered validator {} with stake {} and reputation {}", 
            agent_id, stake, reputation);

        Ok(())
    }

    /// Remove a validator from the set
    pub fn remove_validator(&mut self, agent_id: &AgentId) -> Result<()> {
        if let Some(validator) = self.validators.remove(agent_id) {
            self.validator_performance.remove(agent_id);
            info!("Removed validator {} from consensus", agent_id);
        } else {
            warn!("Attempted to remove non-existent validator {}", agent_id);
        }

        Ok(())
    }

    /// Update validator stake
    pub fn update_validator_stake(&mut self, agent_id: &AgentId, new_stake: u64) -> Result<()> {
        if let Some(validator) = self.validators.get_mut(agent_id) {
            validator.stake = new_stake;
            validator.is_active = new_stake >= self.config.min_validator_stake;
            debug!("Updated validator {} stake to {}", agent_id, new_stake);
        } else {
            return Err(SolaceError::ValidatorNotFound(agent_id.clone()).into());
        }

        Ok(())
    }

    /// Update validator reputation
    pub fn update_validator_reputation(&mut self, agent_id: &AgentId, reputation: f64) -> Result<()> {
        if let Some(validator) = self.validators.get_mut(agent_id) {
            validator.reputation = reputation.clamp(0.0, 1.0);
            debug!("Updated validator {} reputation to {}", agent_id, reputation);
        } else {
            return Err(SolaceError::ValidatorNotFound(agent_id.clone()).into());
        }

        Ok(())
    }

    /// Select validators for the next epoch
    pub fn select_validators_for_epoch(&mut self, epoch_number: u32) -> Result<Vec<AgentId>> {
        let mut weighted_validators: Vec<_> = self.validators
            .iter()
            .filter(|(_, validator)| validator.is_active)
            .map(|(agent_id, validator)| {
                let weight = validator.calculate_weight(&self.config);
                (agent_id.clone(), weight)
            })
            .collect();

        // Sort by weight (descending)
        weighted_validators.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Select top validators
        let selected_count = std::cmp::min(self.config.validators_per_epoch, weighted_validators.len());
        let selected: Vec<AgentId> = weighted_validators
            .into_iter()
            .take(selected_count)
            .map(|(agent_id, _)| agent_id)
            .collect();

        info!("Selected {} validators for epoch {}", selected.len(), epoch_number);

        Ok(selected)
    }

    /// Get the next block producer for a given block height
    pub fn get_block_producer(&self, block_height: u64) -> Option<&AgentId> {
        if self.current_epoch.validators.is_empty() {
            return None;
        }

        let index = (block_height % self.current_epoch.validators.len() as u64) as usize;
        self.current_epoch.validators.get(index)
    }

    /// Validate a proposed block
    pub fn validate_block(&self, header: &BlockHeader) -> Result<bool> {
        // Check if producer is authorized for this block
        if let Some(expected_producer) = self.get_block_producer(header.height) {
            if &header.producer != expected_producer {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }

        // Check timestamp
        let now = SystemTime::now();
        if header.timestamp > now {
            return Ok(false);
        }

        // Check block height sequence
        if let Some(last_block) = self.block_history.back() {
            if header.height != last_block.height + 1 {
                return Ok(false);
            }
            if header.previous_hash != self.calculate_block_hash(last_block) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Process a consensus vote
    pub fn process_vote(&mut self, vote: ConsensusVote) -> Result<()> {
        // Verify the voter is a validator
        if !self.validators.contains_key(&vote.voter) {
            return Err(SolaceError::ValidatorNotFound(vote.voter).into());
        }

        // Add vote to pending votes
        self.pending_votes
            .entry(vote.block_hash.clone())
            .or_insert_with(Vec::new)
            .push(vote.clone());

        // Update validator performance
        if let Some(performance) = self.validator_performance.get_mut(&vote.voter) {
            performance.votes_cast += 1;
        }

        debug!("Processed vote from validator {} for block {}", 
            vote.voter, vote.block_hash);

        Ok(())
    }

    /// Check if a block has enough votes to be finalized
    pub fn check_finalization(&self, block_hash: &Hash) -> bool {
        if let Some(votes) = self.pending_votes.get(block_hash) {
            let approve_votes = votes.iter()
                .filter(|vote| matches!(vote.vote_type, VoteType::Approve))
                .count();

            let required_votes = (self.current_epoch.validators.len() * 2) / 3 + 1;
            approve_votes >= required_votes
        } else {
            false
        }
    }

    /// Finalize a block and update validator state
    pub fn finalize_block(&mut self, header: BlockHeader) -> Result<()> {
        // Update block producer statistics
        if let Some(validator) = self.validators.get_mut(&header.producer) {
            validator.blocks_produced += 1;
            validator.last_block_time = header.timestamp;

            // Check for consecutive blocks
            if let Some(last_block) = self.block_history.back() {
                if last_block.producer == header.producer {
                    validator.consecutive_blocks += 1;
                } else {
                    validator.consecutive_blocks = 1;
                }
            }
        }

        // Update performance metrics
        if let Some(performance) = self.validator_performance.get_mut(&header.producer) {
            performance.blocks_produced += 1;
        }

        // Add to block history
        self.block_history.push_back(header.clone());
        if self.block_history.len() > 1000 {
            self.block_history.pop_front();
        }

        // Check if we need to start a new epoch
        if header.height >= self.current_epoch.end_block {
            self.start_new_epoch(header.height + 1)?;
        }

        // Clean up old votes
        let block_hash = self.calculate_block_hash(&header);
        self.pending_votes.remove(&block_hash);

        info!("Finalized block {} produced by {}", header.height, header.producer);

        Ok(())
    }

    /// Start a new epoch with validator rotation
    fn start_new_epoch(&mut self, start_block: u64) -> Result<()> {
        let new_epoch_number = self.current_epoch.number + 1;
        let selected_validators = self.select_validators_for_epoch(new_epoch_number)?;

        self.current_epoch = Epoch {
            number: new_epoch_number,
            start_block,
            end_block: start_block + self.config.epoch_duration as u64,
            validators: selected_validators,
            block_producers: BTreeMap::new(),
        };

        // Reset consecutive block counts
        for validator in self.validators.values_mut() {
            validator.consecutive_blocks = 0;
        }

        info!("Started epoch {} with {} validators", 
            new_epoch_number, self.current_epoch.validators.len());

        Ok(())
    }

    /// Apply slashing to a validator for misbehavior
    pub fn slash_validator(&mut self, agent_id: &AgentId, reason: &str) -> Result<()> {
        if let Some(validator) = self.validators.get_mut(agent_id) {
            validator.slashing_events += 1;
            validator.stake = (validator.stake as f64 * 0.9) as u64; // 10% slash
            
            if validator.stake < self.config.min_validator_stake {
                validator.is_active = false;
            }

            warn!("Slashed validator {} for: {}", agent_id, reason);
        } else {
            return Err(SolaceError::ValidatorNotFound(agent_id.clone()).into());
        }

        Ok(())
    }

    /// Get consensus statistics
    pub fn get_consensus_stats(&self) -> ConsensusStats {
        let total_validators = self.validators.len();
        let active_validators = self.validators.values().filter(|v| v.is_active).count();
        
        let total_stake: u64 = self.validators.values().map(|v| v.stake).sum();
        let average_reputation: f64 = if total_validators > 0 {
            self.validators.values().map(|v| v.reputation).sum::<f64>() / total_validators as f64
        } else {
            0.0
        };

        ConsensusStats {
            current_epoch: self.current_epoch.number,
            total_validators,
            active_validators,
            total_stake,
            average_reputation,
            blocks_finalized: self.block_history.len(),
        }
    }

    /// Calculate block hash (simplified for demo)
    fn calculate_block_hash(&self, header: &BlockHeader) -> Hash {
        use sha2::{Sha256, Digest};
        
        let serialized = serde_json::to_vec(header).unwrap_or_default();
        let hash = Sha256::digest(&serialized);
        format!("{:x}", hash)
    }
}

/// Consensus statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStats {
    pub current_epoch: u32,
    pub total_validators: usize,
    pub active_validators: usize,
    pub total_stake: u64,
    pub average_reputation: f64,
    pub blocks_finalized: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_weight_calculation() {
        let config = ConsensusConfig::default();
        let validator = Validator::new(AgentId::new(), 10000, 0.8);
        
        let weight = validator.calculate_weight(&config);
        assert!(weight > 0.0);
    }

    #[test]
    fn test_consensus_engine_creation() {
        let config = ConsensusConfig::default();
        let engine = ConsensusEngine::new(config);
        
        assert_eq!(engine.validators.len(), 0);
        assert_eq!(engine.current_epoch.number, 0);
    }

    #[tokio::test]
    async fn test_validator_registration() {
        let mut engine = ConsensusEngine::new(ConsensusConfig::default());
        let agent_id = AgentId::new();
        
        let result = engine.register_validator(agent_id.clone(), 5000, 0.7);
        assert!(result.is_ok());
        assert!(engine.validators.contains_key(&agent_id));
    }

    #[test]
    fn test_insufficient_stake_rejection() {
        let mut engine = ConsensusEngine::new(ConsensusConfig::default());
        let agent_id = AgentId::new();
        
        let result = engine.register_validator(agent_id, 500, 0.8); // Below minimum
        assert!(result.is_err());
    }
} 