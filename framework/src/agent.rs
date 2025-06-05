//! Agent implementation for autonomous commerce

use crate::{
    error::{AgentError, Result},
    reputation::ReputationScore,
    types::{AgentId, Balance, NetworkAddress, ServiceType, Timestamp, WalletInfo},
};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

/// Agent capabilities that define what services an agent can provide
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentCapability {
    DataAnalysis,
    ComputationalTask,
    MarketResearch,
    ContentCreation,
    TradingService,
    MachineLearning,
    CustomCapability(String),
}

impl AgentCapability {
    pub fn matches_service(&self, service: &ServiceType) -> bool {
        match (self, service) {
            (AgentCapability::DataAnalysis, ServiceType::DataAnalysis) => true,
            (AgentCapability::ComputationalTask, ServiceType::ComputationalTask) => true,
            (AgentCapability::MarketResearch, ServiceType::MarketResearch) => true,
            (AgentCapability::ContentCreation, ServiceType::ContentCreation) => true,
            (AgentCapability::TradingService, ServiceType::TradingService) => true,
            (AgentCapability::CustomCapability(cap), ServiceType::CustomService(srv)) => cap == srv,
            _ => false,
        }
    }
}

/// Agent preferences for autonomous decision-making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreferences {
    /// Risk tolerance (0.0 = risk-averse, 1.0 = risk-seeking)
    pub risk_tolerance: f64,
    /// Maximum transaction value the agent will accept
    pub max_transaction_value: Balance,
    /// Minimum reputation score required for counterparties
    pub min_counterparty_reputation: f64,
    /// Preferred payment methods
    pub preferred_payment_methods: Vec<String>,
    /// Automatic acceptance threshold for reputation scores
    pub auto_accept_threshold: f64,
    /// Geographic preferences (optional)
    pub geographic_preferences: Option<Vec<String>>,
}

impl Default for AgentPreferences {
    fn default() -> Self {
        Self {
            risk_tolerance: 0.5,
            max_transaction_value: Balance::from_sol(100.0),
            min_counterparty_reputation: 0.3,
            preferred_payment_methods: vec!["SOL".to_string()],
            auto_accept_threshold: 0.8,
            geographic_preferences: None,
        }
    }
}

/// Configuration for creating a new agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent's wallet keypair
    pub keypair: Option<Keypair>,
    /// Agent's display name
    pub name: String,
    /// Agent's description
    pub description: String,
    /// Agent's capabilities
    pub capabilities: Vec<AgentCapability>,
    /// Agent's preferences
    pub preferences: AgentPreferences,
    /// Network address for peer-to-peer communication
    pub network_address: Option<NetworkAddress>,
    /// Initial reputation score (for testing, normally starts at 0.5)
    pub initial_reputation: Option<f64>,
}

/// Agent state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Offline,
    Online,
    Busy,
    Maintenance,
}

/// Core agent structure
#[derive(Debug)]
pub struct Agent {
    /// Unique agent identifier
    pub id: AgentId,
    /// Agent configuration
    pub config: AgentConfig,
    /// Current agent state
    pub state: Arc<RwLock<AgentState>>,
    /// Agent's reputation score
    pub reputation: Arc<RwLock<ReputationScore>>,
    /// Wallet information
    pub wallet: Arc<RwLock<WalletInfo>>,
    /// Active transactions
    pub active_transactions: Arc<RwLock<HashMap<String, String>>>,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Last activity timestamp
    pub last_active: Arc<RwLock<Timestamp>>,
}

impl Agent {
    /// Create a new agent with the given configuration
    pub async fn new(mut config: AgentConfig) -> Result<Self> {
        // Generate keypair if not provided
        if config.keypair.is_none() {
            config.keypair = Some(Keypair::new());
        }

        let keypair = config.keypair.as_ref().unwrap();
        let pubkey = keypair.pubkey();

        // Validate configuration
        Self::validate_config(&config)?;

        let id = AgentId::new();
        let initial_reputation = config.initial_reputation.unwrap_or(0.5);
        
        let agent = Self {
            id,
            config,
            state: Arc::new(RwLock::new(AgentState::Offline)),
            reputation: Arc::new(RwLock::new(ReputationScore::new(initial_reputation))),
            wallet: Arc::new(RwLock::new(WalletInfo::new(pubkey, Balance::new(0)))),
            active_transactions: Arc::new(RwLock::new(HashMap::new())),
            created_at: Timestamp::now(),
            last_active: Arc::new(RwLock::new(Timestamp::now())),
        };

        tracing::info!("Created new agent {} ({})", agent.config.name, agent.id);
        Ok(agent)
    }

    /// Validate agent configuration
    fn validate_config(config: &AgentConfig) -> Result<()> {
        if config.name.trim().is_empty() {
            return Err(AgentError::InvalidConfig {
                reason: "Agent name cannot be empty".to_string(),
            }.into());
        }

        if config.capabilities.is_empty() {
            return Err(AgentError::InvalidConfig {
                reason: "Agent must have at least one capability".to_string(),
            }.into());
        }

        if config.preferences.risk_tolerance < 0.0 || config.preferences.risk_tolerance > 1.0 {
            return Err(AgentError::InvalidConfig {
                reason: "Risk tolerance must be between 0.0 and 1.0".to_string(),
            }.into());
        }

        if config.preferences.min_counterparty_reputation < 0.0 || config.preferences.min_counterparty_reputation > 1.0 {
            return Err(AgentError::InvalidConfig {
                reason: "Minimum counterparty reputation must be between 0.0 and 1.0".to_string(),
            }.into());
        }

        Ok(())
    }

    /// Get agent's public key
    pub fn public_key(&self) -> Pubkey {
        self.config.keypair.as_ref().unwrap().pubkey()
    }

    /// Get current agent state
    pub async fn get_state(&self) -> AgentState {
        *self.state.read().await
    }

    /// Set agent state
    pub async fn set_state(&self, new_state: AgentState) -> Result<()> {
        let mut state = self.state.write().await;
        *state = new_state;
        *self.last_active.write().await = Timestamp::now();
        
        tracing::debug!("Agent {} state changed to {:?}", self.id, new_state);
        Ok(())
    }

    /// Start the agent (set to online state)
    pub async fn start(&self) -> Result<()> {
        self.set_state(AgentState::Online).await?;
        tracing::info!("Agent {} ({}) started", self.config.name, self.id);
        Ok(())
    }

    /// Stop the agent (set to offline state)
    pub async fn stop(&self) -> Result<()> {
        self.set_state(AgentState::Offline).await?;
        tracing::info!("Agent {} ({}) stopped", self.config.name, self.id);
        Ok(())
    }

    /// Check if agent can handle a specific service type
    pub fn can_handle_service(&self, service_type: &ServiceType) -> bool {
        self.config
            .capabilities
            .iter()
            .any(|cap| cap.matches_service(service_type))
    }

    /// Get current reputation score
    pub async fn get_reputation(&self) -> f64 {
        self.reputation.read().await.current_score()
    }

    /// Update reputation score
    pub async fn update_reputation(&self, new_score: f64) -> Result<()> {
        if new_score < 0.0 || new_score > 1.0 {
            return Err(AgentError::InvalidConfig {
                reason: "Reputation score must be between 0.0 and 1.0".to_string(),
            }.into());
        }

        let mut reputation = self.reputation.write().await;
        reputation.update_score(new_score);
        *self.last_active.write().await = Timestamp::now();
        
        tracing::debug!("Agent {} reputation updated to {}", self.id, new_score);
        Ok(())
    }

    /// Get wallet balance
    pub async fn get_balance(&self) -> Balance {
        self.wallet.read().await.balance
    }

    /// Update wallet balance
    pub async fn update_balance(&self, new_balance: Balance) -> Result<()> {
        let mut wallet = self.wallet.write().await;
        wallet.balance = new_balance;
        wallet.last_updated = Timestamp::now();
        *self.last_active.write().await = Timestamp::now();
        
        tracing::debug!("Agent {} balance updated to {}", self.id, new_balance);
        Ok(())
    }

    /// Check if agent is online and available
    pub async fn is_available(&self) -> bool {
        matches!(self.get_state().await, AgentState::Online)
    }

    /// Check if agent meets minimum requirements for a transaction
    pub async fn meets_requirements(&self, min_reputation: f64, required_balance: Balance) -> bool {
        let current_reputation = self.get_reputation().await;
        let current_balance = self.get_balance().await;
        
        current_reputation >= min_reputation && current_balance.0 >= required_balance.0
    }

    /// Get agent summary for display
    pub async fn get_summary(&self) -> AgentSummary {
        AgentSummary {
            id: self.id,
            name: self.config.name.clone(),
            description: self.config.description.clone(),
            capabilities: self.config.capabilities.clone(),
            state: self.get_state().await,
            reputation: self.get_reputation().await,
            balance: self.get_balance().await,
            created_at: self.created_at,
            last_active: *self.last_active.read().await,
        }
    }
}

/// Agent summary for display and serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSummary {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<AgentCapability>,
    pub state: AgentState,
    pub reputation: f64,
    pub balance: Balance,
    pub created_at: Timestamp,
    pub last_active: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            keypair: None,
            name: "Test Agent".to_string(),
            description: "A test agent".to_string(),
            capabilities: vec![AgentCapability::DataAnalysis],
            preferences: AgentPreferences::default(),
            network_address: None,
            initial_reputation: Some(0.7),
        }
    }

    #[tokio::test]
    async fn test_agent_creation() {
        let config = create_test_config();
        let agent = Agent::new(config).await.unwrap();
        
        assert_eq!(agent.config.name, "Test Agent");
        assert_eq!(agent.get_reputation().await, 0.7);
        assert_eq!(agent.get_state().await, AgentState::Offline);
    }

    #[tokio::test]
    async fn test_agent_state_management() {
        let config = create_test_config();
        let agent = Agent::new(config).await.unwrap();
        
        // Test starting agent
        agent.start().await.unwrap();
        assert_eq!(agent.get_state().await, AgentState::Online);
        assert!(agent.is_available().await);
        
        // Test stopping agent
        agent.stop().await.unwrap();
        assert_eq!(agent.get_state().await, AgentState::Offline);
        assert!(!agent.is_available().await);
    }

    #[tokio::test]
    async fn test_service_capability_matching() {
        let config = create_test_config();
        let agent = Agent::new(config).await.unwrap();
        
        assert!(agent.can_handle_service(&ServiceType::DataAnalysis));
        assert!(!agent.can_handle_service(&ServiceType::TradingService));
    }

    #[test]
    fn test_config_validation() {
        let mut config = create_test_config();
        
        // Test empty name
        config.name = "".to_string();
        assert!(Agent::validate_config(&config).is_err());
        
        // Test empty capabilities
        config.name = "Test".to_string();
        config.capabilities = vec![];
        assert!(Agent::validate_config(&config).is_err());
        
        // Test invalid risk tolerance
        config.capabilities = vec![AgentCapability::DataAnalysis];
        config.preferences.risk_tolerance = 1.5;
        assert!(Agent::validate_config(&config).is_err());
    }
}