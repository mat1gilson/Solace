//! Integration tests for Solace Protocol
//!
//! This module contains comprehensive integration tests that verify the
//! end-to-end functionality of the Solace Protocol system.

use solace_protocol::{
    Agent, AgentConfig, AgentCapability, AgentPreferences,
    Transaction, TransactionRequest, TransactionPhase, TransactionStatus,
    ReputationScore, Balance, ServiceType, Timestamp,
};
use acp::{ACP, ACPConfig, messaging::ACPMessage};
use tokio_test;
use std::time::Duration;
use uuid::Uuid;

/// Test fixture for creating test agents
pub struct TestAgentFactory;

impl TestAgentFactory {
    /// Create a basic test agent configuration
    pub fn create_basic_config(name: &str) -> AgentConfig {
        AgentConfig {
            keypair: None,
            name: name.to_string(),
            description: format!("Test agent: {}", name),
            capabilities: vec![
                AgentCapability::DataAnalysis,
                AgentCapability::ComputationalTask,
            ],
            preferences: AgentPreferences {
                risk_tolerance: 0.5,
                max_transaction_value: Balance::from_sol(100.0),
                min_counterparty_reputation: 0.3,
                preferred_payment_methods: vec!["SOL".to_string()],
                auto_accept_threshold: 0.8,
                geographic_preferences: None,
            },
            network_address: None,
            initial_reputation: Some(0.7),
        }
    }

    /// Create a specialized trading agent
    pub fn create_trading_agent(name: &str) -> AgentConfig {
        let mut config = Self::create_basic_config(name);
        config.capabilities = vec![
            AgentCapability::TradingService,
            AgentCapability::MarketResearch,
        ];
        config.preferences.risk_tolerance = 0.8;
        config.preferences.max_transaction_value = Balance::from_sol(1000.0);
        config
    }

    /// Create a conservative data analysis agent
    pub fn create_analysis_agent(name: &str) -> AgentConfig {
        let mut config = Self::create_basic_config(name);
        config.capabilities = vec![AgentCapability::DataAnalysis];
        config.preferences.risk_tolerance = 0.3;
        config.preferences.min_counterparty_reputation = 0.6;
        config
    }
}

/// Test environment for running multi-agent scenarios
pub struct TestEnvironment {
    agents: Vec<Agent>,
    acp_nodes: Vec<ACP>,
}

impl TestEnvironment {
    /// Create a new test environment
    pub async fn new() -> Self {
        Self {
            agents: Vec::new(),
            acp_nodes: Vec::new(),
        }
    }

    /// Add an agent to the test environment
    pub async fn add_agent(&mut self, config: AgentConfig) -> Result<(), Box<dyn std::error::Error>> {
        let agent = Agent::new(config).await?;
        agent.start().await?;
        self.agents.push(agent);
        Ok(())
    }

    /// Add an ACP node to the test environment
    pub async fn add_acp_node(&mut self, config: ACPConfig) -> Result<(), Box<dyn std::error::Error>> {
        let mut acp = ACP::new(config).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        acp.start().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        self.acp_nodes.push(acp);
        Ok(())
    }

    /// Get agent by index
    pub fn get_agent(&self, index: usize) -> Option<&Agent> {
        self.agents.get(index)
    }

    /// Get number of agents
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// Cleanup the test environment
    pub async fn cleanup(&mut self) {
        // Stop all agents
        for agent in &self.agents {
            let _ = agent.stop().await;
        }

        // Stop all ACP nodes
        for acp in &mut self.acp_nodes {
            let _ = acp.stop().await;
        }
    }
}

#[tokio::test]
async fn test_basic_agent_lifecycle() {
    let config = TestAgentFactory::create_basic_config("test-agent-1");
    let agent = Agent::new(config).await.unwrap();

    // Test initial state
    assert_eq!(agent.get_state().await, solace_protocol::AgentState::Offline);
    
    // Test starting agent
    agent.start().await.unwrap();
    assert_eq!(agent.get_state().await, solace_protocol::AgentState::Online);
    assert!(agent.is_available().await);

    // Test agent capabilities
    assert!(agent.can_handle_service(&ServiceType::DataAnalysis));
    assert!(agent.can_handle_service(&ServiceType::ComputationalTask));
    assert!(!agent.can_handle_service(&ServiceType::TradingService));

    // Test reputation
    assert_eq!(agent.get_reputation().await, 0.7);

    // Test stopping agent
    agent.stop().await.unwrap();
    assert_eq!(agent.get_state().await, solace_protocol::AgentState::Offline);
    assert!(!agent.is_available().await);
}

#[tokio::test]
async fn test_transaction_lifecycle() {
    // Create test environment
    let mut env = TestEnvironment::new().await;
    
    // Add requester and provider agents
    let requester_config = TestAgentFactory::create_basic_config("requester");
    let provider_config = TestAgentFactory::create_analysis_agent("provider");
    
    env.add_agent(requester_config).await.unwrap();
    env.add_agent(provider_config).await.unwrap();

    let requester = env.get_agent(0).unwrap();
    let provider = env.get_agent(1).unwrap();

    // Create a transaction request
    let transaction_request = TransactionRequest::new(
        requester.id,
        ServiceType::DataAnalysis,
        "Analyze market data for Q4 trends".to_string(),
        Balance::from_sol(50.0),
        Timestamp::now(),
    );

    // Create transaction
    let mut transaction = Transaction::new(transaction_request);
    
    // Verify initial state
    assert_eq!(transaction.phase, TransactionPhase::Request);
    assert_eq!(transaction.status, TransactionStatus::Pending);

    // Simulate proposal from provider
    let proposal = solace_protocol::TransactionProposal {
        id: solace_protocol::TransactionId::new(),
        request_id: transaction.id,
        provider: provider.id,
        proposed_price: Balance::from_sol(40.0),
        estimated_completion: Timestamp::now(),
        proposal_details: "Comprehensive market analysis with ML insights".to_string(),
        terms: std::collections::HashMap::new(),
        created_at: Timestamp::now(),
        expires_at: Timestamp::now(),
    };

    transaction.add_proposal(proposal).unwrap();
    assert_eq!(transaction.phase, TransactionPhase::Negotiation);
    assert_eq!(transaction.proposals.len(), 1);

    // Accept proposal
    transaction.accept_proposal(provider.id, Balance::from_sol(40.0)).unwrap();
    assert_eq!(transaction.phase, TransactionPhase::Execution);
    assert_eq!(transaction.status, TransactionStatus::InProgress);
    assert_eq!(transaction.provider, Some(provider.id));

    // Complete execution
    let execution_data = solace_protocol::ExecutionData {
        result: "Analysis complete with 94% confidence".to_string(),
        artifacts: vec!["report.pdf".to_string(), "data.csv".to_string()],
        completion_time: Timestamp::now(),
        quality_metrics: {
            let mut metrics = std::collections::HashMap::new();
            metrics.insert("accuracy".to_string(), 0.94);
            metrics.insert("completeness".to_string(), 0.98);
            metrics
        },
    };

    transaction.complete_execution(execution_data).unwrap();
    assert_eq!(transaction.phase, TransactionPhase::Evaluation);

    // Add evaluation
    let evaluation = solace_protocol::TransactionEvaluation {
        requester_rating: 4.8,
        provider_rating: 4.9,
        requester_feedback: "Excellent analysis, very detailed".to_string(),
        provider_feedback: "Professional client, clear requirements".to_string(),
        quality_score: 0.94,
        timeliness_score: 0.95,
        overall_satisfaction: 0.94,
    };

    transaction.add_evaluation(evaluation).unwrap();
    assert_eq!(transaction.status, TransactionStatus::Completed);

    // Cleanup
    env.cleanup().await;
}

#[tokio::test]
async fn test_reputation_system() {
    let config = TestAgentFactory::create_basic_config("reputation-test");
    let agent = Agent::new(config).await.unwrap();

    // Test initial reputation
    let initial_reputation = agent.get_reputation().await;
    assert_eq!(initial_reputation, 0.7);

    // Test reputation update
    agent.update_reputation(0.8).await.unwrap();
    assert_eq!(agent.get_reputation().await, 0.8);

    // Test reputation bounds
    agent.update_reputation(1.5).await.unwrap_err(); // Should fail - out of bounds
    agent.update_reputation(-0.1).await.unwrap_err(); // Should fail - out of bounds

    // Test valid bounds
    agent.update_reputation(1.0).await.unwrap();
    assert_eq!(agent.get_reputation().await, 1.0);

    agent.update_reputation(0.0).await.unwrap();
    assert_eq!(agent.get_reputation().await, 0.0);
}

#[tokio::test]
async fn test_multi_agent_coordination() {
    let mut env = TestEnvironment::new().await;

    // Create multiple agents with different capabilities
    let configs = vec![
        TestAgentFactory::create_analysis_agent("analyst-1"),
        TestAgentFactory::create_analysis_agent("analyst-2"),
        TestAgentFactory::create_trading_agent("trader-1"),
        TestAgentFactory::create_basic_config("generalist-1"),
    ];

    for config in configs {
        env.add_agent(config).await.unwrap();
    }

    assert_eq!(env.agent_count(), 4);

    // Test that all agents are available
    for i in 0..env.agent_count() {
        let agent = env.get_agent(i).unwrap();
        assert!(agent.is_available().await);
    }

    // Test capability matching
    let analyst1 = env.get_agent(0).unwrap();
    let trader1 = env.get_agent(2).unwrap();

    assert!(analyst1.can_handle_service(&ServiceType::DataAnalysis));
    assert!(!analyst1.can_handle_service(&ServiceType::TradingService));

    assert!(trader1.can_handle_service(&ServiceType::TradingService));
    assert!(trader1.can_handle_service(&ServiceType::MarketResearch));

    env.cleanup().await;
}

#[tokio::test]
async fn test_acp_messaging() {
    use acp::messaging::{ACPMessage, MessageType};

    // Create ACP nodes
    let mut node1_config = ACPConfig::default();
    node1_config.node_id = "node-1".to_string();
    node1_config.listen_address = "127.0.0.1:8081".to_string();

    let mut node2_config = ACPConfig::default();
    node2_config.node_id = "node-2".to_string();
    node2_config.listen_address = "127.0.0.1:8082".to_string();
    node2_config.bootstrap_peers = vec!["127.0.0.1:8081".to_string()];

    // Test message creation
    let message = ACPMessage::new(
        MessageType::Heartbeat,
        "node-1".to_string(),
        Some("node-2".to_string()),
        b"heartbeat data".to_vec(),
    );

    assert_eq!(message.message_type, MessageType::Heartbeat);
    assert_eq!(message.from, "node-1");
    assert_eq!(message.to, Some("node-2".to_string()));
    assert!(!message.is_signed());

    // Test message serialization
    let serialized = message.serialize().unwrap();
    let deserialized = ACPMessage::deserialize(&serialized).unwrap();

    assert_eq!(message.id, deserialized.id);
    assert_eq!(message.message_type, deserialized.message_type);
    assert_eq!(message.from, deserialized.from);
    assert_eq!(message.to, deserialized.to);
}

#[tokio::test]
async fn test_error_handling() {
    // Test invalid agent configuration
    let mut invalid_config = TestAgentFactory::create_basic_config("invalid");
    invalid_config.name = "".to_string(); // Empty name should fail
    
    let result = Agent::new(invalid_config).await;
    assert!(result.is_err());

    // Test invalid capability configuration
    let mut invalid_config2 = TestAgentFactory::create_basic_config("invalid2");
    invalid_config2.capabilities = vec![]; // No capabilities should fail
    
    let result2 = Agent::new(invalid_config2).await;
    assert!(result2.is_err());

    // Test invalid preferences
    let mut invalid_config3 = TestAgentFactory::create_basic_config("invalid3");
    invalid_config3.preferences.risk_tolerance = 1.5; // Out of bounds should fail
    
    let result3 = Agent::new(invalid_config3).await;
    assert!(result3.is_err());
}

#[tokio::test]
async fn test_concurrent_operations() {
    use futures::future::join_all;

    // Create multiple agents concurrently
    let configs = (0..10)
        .map(|i| TestAgentFactory::create_basic_config(&format!("concurrent-agent-{}", i)))
        .collect::<Vec<_>>();

    let agent_futures = configs.into_iter().map(|config| async move {
        let agent = Agent::new(config).await.unwrap();
        agent.start().await.unwrap();
        agent
    });

    let agents = join_all(agent_futures).await;
    assert_eq!(agents.len(), 10);

    // Test concurrent operations
    let operation_futures = agents.iter().map(|agent| async move {
        // Perform concurrent reputation updates
        agent.update_reputation(0.8).await.unwrap();
        agent.get_reputation().await
    });

    let reputations = join_all(operation_futures).await;
    assert!(reputations.iter().all(|&rep| rep == 0.8));

    // Cleanup
    let cleanup_futures = agents.iter().map(|agent| agent.stop());
    join_all(cleanup_futures).await;
}

#[tokio::test]
#[ignore] // Long-running test
async fn test_load_scenario() {
    const NUM_AGENTS: usize = 100;
    const NUM_TRANSACTIONS: usize = 1000;

    let mut env = TestEnvironment::new().await;

    // Create many agents
    for i in 0..NUM_AGENTS {
        let config = if i % 3 == 0 {
            TestAgentFactory::create_analysis_agent(&format!("agent-{}", i))
        } else if i % 3 == 1 {
            TestAgentFactory::create_trading_agent(&format!("agent-{}", i))
        } else {
            TestAgentFactory::create_basic_config(&format!("agent-{}", i))
        };
        
        env.add_agent(config).await.unwrap();
    }

    // Simulate many concurrent transactions
    let start_time = std::time::Instant::now();
    
    // In a real load test, we would create and process many transactions
    // For now, we just verify all agents are responsive
    for i in 0..env.agent_count() {
        let agent = env.get_agent(i).unwrap();
        assert!(agent.is_available().await);
    }

    let duration = start_time.elapsed();
    println!("Load test completed in {:?} for {} agents", duration, NUM_AGENTS);

    env.cleanup().await;
}

/// Property-based testing using proptest
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_balance_operations(sol_amount in 0.0f64..1000.0f64) {
            let balance = Balance::from_sol(sol_amount);
            let converted_back = balance.to_sol();
            
            // Should be approximately equal (allowing for floating point precision)
            prop_assert!((sol_amount - converted_back).abs() < 0.000001);
        }

        #[test]
        fn test_reputation_bounds(reputation in 0.0f64..1.0f64) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let config = TestAgentFactory::create_basic_config("prop-test");
                let agent = Agent::new(config).await.unwrap();
                
                let result = agent.update_reputation(reputation).await;
                prop_assert!(result.is_ok());
                
                let current_reputation = agent.get_reputation().await;
                prop_assert!((reputation - current_reputation).abs() < 0.000001);
            });
        }
    }
}

/// Benchmarking utilities
#[cfg(test)]
mod bench_utils {
    use super::*;
    use std::time::Instant;

    pub async fn benchmark_agent_creation(num_agents: usize) -> Duration {
        let start = Instant::now();
        
        let mut agents = Vec::new();
        for i in 0..num_agents {
            let config = TestAgentFactory::create_basic_config(&format!("bench-agent-{}", i));
            let agent = Agent::new(config).await.unwrap();
            agents.push(agent);
        }
        
        let duration = start.elapsed();
        
        // Cleanup
        for agent in agents {
            let _ = agent.stop().await;
        }
        
        duration
    }

    pub async fn benchmark_transaction_processing(num_transactions: usize) -> Duration {
        let start = Instant::now();
        
        for i in 0..num_transactions {
            let request = TransactionRequest::new(
                solace_protocol::AgentId::new(),
                ServiceType::DataAnalysis,
                format!("Transaction {}", i),
                Balance::from_sol(10.0),
                Timestamp::now(),
            );
            
            let _transaction = Transaction::new(request);
            // In a real benchmark, we would process the transaction fully
        }
        
        start.elapsed()
    }
}

#[tokio::test]
async fn test_performance_benchmarks() {
    // Light performance tests (full benchmarks in separate benchmark suite)
    let agent_creation_time = bench_utils::benchmark_agent_creation(10).await;
    println!("Created 10 agents in {:?}", agent_creation_time);
    
    let transaction_processing_time = bench_utils::benchmark_transaction_processing(100).await;
    println!("Processed 100 transactions in {:?}", transaction_processing_time);
    
    // Basic performance assertions
    assert!(agent_creation_time < Duration::from_secs(5));
    assert!(transaction_processing_time < Duration::from_millis(100));
}