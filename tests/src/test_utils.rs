//! Test Utilities for Solace Protocol
//!
//! Comprehensive testing utilities including mock implementations,
//! test data generators, and helper functions for protocol testing.

use solace_protocol::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Mock blockchain client for testing
pub struct MockBlockchainClient {
    pub transactions: Arc<RwLock<Vec<MockTransaction>>>,
    pub accounts: Arc<RwLock<HashMap<String, u64>>>,
    pub latency: Duration,
    pub failure_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockTransaction {
    pub id: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

impl MockBlockchainClient {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(Vec::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            latency: Duration::from_millis(100),
            failure_rate: 0.01, // 1% failure rate
        }
    }

    pub async fn transfer(&self, from: &str, to: &str, amount: u64) -> Result<String> {
        // Simulate network latency
        tokio::time::sleep(self.latency).await;
        
        // Simulate occasional failures
        if rand::random::<f64>() < self.failure_rate {
            return Err(anyhow::anyhow!("Mock transaction failed"));
        }
        
        let transaction_id = uuid::Uuid::new_v4().to_string();
        let transaction = MockTransaction {
            id: transaction_id.clone(),
            from: from.to_string(),
            to: to.to_string(),
            amount,
            timestamp: chrono::Utc::now(),
            status: TransactionStatus::Pending,
        };
        
        // Update account balances
        let mut accounts = self.accounts.write().await;
        let from_balance = accounts.get(from).unwrap_or(&0);
        if *from_balance < amount {
            return Err(anyhow::anyhow!("Insufficient balance"));
        }
        
        accounts.insert(from.to_string(), from_balance - amount);
        let to_balance = accounts.get(to).unwrap_or(&0);
        accounts.insert(to.to_string(), to_balance + amount);
        drop(accounts);
        
        // Store transaction
        let mut transactions = self.transactions.write().await;
        transactions.push(transaction);
        
        Ok(transaction_id)
    }

    pub async fn get_balance(&self, account: &str) -> u64 {
        let accounts = self.accounts.read().await;
        *accounts.get(account).unwrap_or(&0)
    }

    pub async fn confirm_transaction(&self, tx_id: &str) -> Result<()> {
        let mut transactions = self.transactions.write().await;
        if let Some(tx) = transactions.iter_mut().find(|tx| tx.id == tx_id) {
            tx.status = TransactionStatus::Confirmed;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Transaction not found"))
        }
    }
}

/// Test data generators
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate a random agent configuration
    pub fn random_agent_config() -> AgentConfig {
        let capabilities = vec![
            AgentCapability::DataAnalysis,
            AgentCapability::ComputationalTask,
            AgentCapability::MarketResearch,
        ];
        
        let selected_capabilities = capabilities
            .into_iter()
            .filter(|_| rand::random::<bool>())
            .collect::<Vec<_>>();
        
        AgentConfig {
            name: format!("test_agent_{}", uuid::Uuid::new_v4()),
            description: "Generated test agent".to_string(),
            capabilities: if selected_capabilities.is_empty() {
                vec![AgentCapability::DataAnalysis]
            } else {
                selected_capabilities
            },
            preferences: AgentPreferences {
                risk_tolerance: rand::random::<f64>(),
                max_transaction_value: Balance::from_lamports(
                    1000 + rand::random::<u64>() % 10000
                ),
                min_counterparty_reputation: 0.3 + rand::random::<f64>() * 0.4,
                preferred_payment_methods: vec!["SOL".to_string()],
                auto_accept_threshold: 0.7 + rand::random::<f64>() * 0.2,
                geographic_preferences: None,
            },
            ..Default::default()
        }
    }
    
    /// Generate a random service request
    pub fn random_service_request() -> ServiceRequest {
        let service_types = vec![
            ServiceType::DataAnalysis,
            ServiceType::ComputationalTask,
            ServiceType::MarketResearch,
            ServiceType::ContentCreation,
        ];
        
        ServiceRequest {
            id: TransactionId::new(),
            service_type: service_types[rand::random::<usize>() % service_types.len()].clone(),
            requirements: format!("Test requirement {}", uuid::Uuid::new_v4()),
            max_payment: Balance::from_lamports(1000 + rand::random::<u64>() % 5000),
            deadline: chrono::Utc::now() + chrono::Duration::hours(1 + rand::random::<i64>() % 24),
            requester_id: AgentId::new(),
        }
    }
    
    /// Generate multiple agents for testing
    pub async fn create_test_agents(count: usize) -> Result<Vec<Agent>> {
        let mut agents = Vec::new();
        
        for _ in 0..count {
            let config = Self::random_agent_config();
            let agent = Agent::new(config).await?;
            agents.push(agent);
        }
        
        Ok(agents)
    }
    
    /// Generate a test network of peers
    pub fn generate_test_peers(count: usize) -> Vec<PeerInfo> {
        (0..count)
            .map(|i| PeerInfo {
                id: format!("test_peer_{}", i),
                address: format!("192.168.1.{}:8080", i + 1).parse().unwrap(),
                public_key: format!("test_key_{}", i),
                capabilities: vec!["agent".to_string(), "relay".to_string()],
                reputation: 0.5 + rand::random::<f64>() * 0.5,
                last_seen: chrono::Utc::now(),
                protocol_version: "1.0.0".to_string(),
                node_type: NodeType::Agent,
            })
            .collect()
    }
}

/// Network simulation environment
pub struct NetworkSimulator {
    pub agents: Vec<Agent>,
    pub latency_matrix: HashMap<(AgentId, AgentId), Duration>,
    pub message_loss_rate: f64,
    pub bandwidth_limits: HashMap<AgentId, u64>,
}

impl NetworkSimulator {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            latency_matrix: HashMap::new(),
            message_loss_rate: 0.01,
            bandwidth_limits: HashMap::new(),
        }
    }
    
    pub async fn add_agent(&mut self, config: AgentConfig) -> Result<AgentId> {
        let agent = Agent::new(config).await?;
        let agent_id = agent.id().clone();
        self.agents.push(agent);
        Ok(agent_id)
    }
    
    pub fn set_latency(&mut self, agent1: AgentId, agent2: AgentId, latency: Duration) {
        self.latency_matrix.insert((agent1.clone(), agent2.clone()), latency);
        self.latency_matrix.insert((agent2, agent1), latency);
    }
    
    pub async fn simulate_transaction(
        &self,
        requester_id: &AgentId,
        provider_id: &AgentId,
        request: ServiceRequest,
    ) -> Result<Transaction> {
        // Simulate network latency
        if let Some(latency) = self.latency_matrix.get(&(requester_id.clone(), provider_id.clone())) {
            tokio::time::sleep(*latency).await;
        }
        
        // Simulate message loss
        if rand::random::<f64>() < self.message_loss_rate {
            return Err(anyhow::anyhow!("Message lost in network simulation"));
        }
        
        Transaction::new(request, provider_id.clone()).await
    }
}

/// Performance measurement utilities
pub struct PerformanceMetrics {
    pub start_time: Instant,
    pub measurements: Arc<Mutex<Vec<(String, Duration)>>>,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            measurements: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn measure<F, R>(&self, name: &str, operation: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = operation();
        let duration = start.elapsed();
        
        let mut measurements = self.measurements.lock().unwrap();
        measurements.push((name.to_string(), duration));
        
        result
    }
    
    pub async fn measure_async<F, Fut, R>(&self, name: &str, operation: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();
        
        let mut measurements = self.measurements.lock().unwrap();
        measurements.push((name.to_string(), duration));
        
        result
    }
    
    pub fn get_summary(&self) -> PerformanceSummary {
        let measurements = self.measurements.lock().unwrap();
        let total_time = self.start_time.elapsed();
        
        let mut operations = HashMap::new();
        for (name, duration) in measurements.iter() {
            let entry = operations.entry(name.clone()).or_insert_with(Vec::new);
            entry.push(*duration);
        }
        
        let mut summaries = HashMap::new();
        for (name, durations) in operations {
            let count = durations.len();
            let total: Duration = durations.iter().sum();
            let average = total / count as u32;
            let min = durations.iter().min().copied().unwrap_or_default();
            let max = durations.iter().max().copied().unwrap_or_default();
            
            summaries.insert(name, OperationSummary {
                count,
                total,
                average,
                min,
                max,
            });
        }
        
        PerformanceSummary {
            total_time,
            operations: summaries,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub total_time: Duration,
    pub operations: HashMap<String, OperationSummary>,
}

#[derive(Debug, Clone)]
pub struct OperationSummary {
    pub count: usize,
    pub total: Duration,
    pub average: Duration,
    pub min: Duration,
    pub max: Duration,
}

/// Test assertion helpers
pub struct TestAssertions;

impl TestAssertions {
    pub fn assert_agent_valid(agent: &Agent) {
        assert!(!agent.id().to_string().is_empty());
        assert!(!agent.public_key().is_empty());
    }
    
    pub fn assert_transaction_valid(transaction: &Transaction) {
        assert!(!transaction.id().to_string().is_empty());
        assert!(!transaction.requester_id().to_string().is_empty());
        assert!(!transaction.provider_id().to_string().is_empty());
    }
    
    pub fn assert_reputation_in_range(reputation: f64) {
        assert!(reputation >= 0.0 && reputation <= 1.0, 
            "Reputation {} not in valid range [0.0, 1.0]", reputation);
    }
    
    pub fn assert_balance_positive(balance: &Balance) {
        assert!(balance.lamports() > 0, "Balance should be positive");
    }
    
    pub async fn assert_eventually<F, Fut>(condition: F, timeout: Duration, message: &str)
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            if condition().await {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        panic!("Condition not met within timeout: {}", message);
    }
}

/// Load testing utilities
pub struct LoadTester {
    pub concurrent_operations: usize,
    pub operation_count: usize,
    pub ramp_up_time: Duration,
}

impl LoadTester {
    pub fn new(concurrent_operations: usize, operation_count: usize) -> Self {
        Self {
            concurrent_operations,
            operation_count,
            ramp_up_time: Duration::from_secs(10),
        }
    }
    
    pub async fn run_agent_creation_load_test(&self) -> Result<LoadTestResults> {
        let start_time = Instant::now();
        let mut handles = Vec::new();
        
        let operations_per_worker = self.operation_count / self.concurrent_operations;
        
        for worker_id in 0..self.concurrent_operations {
            let handle = tokio::spawn(async move {
                let mut results = Vec::new();
                
                for i in 0..operations_per_worker {
                    let operation_start = Instant::now();
                    
                    let config = TestDataGenerator::random_agent_config();
                    match Agent::new(config).await {
                        Ok(_agent) => {
                            results.push(LoadTestOperation {
                                worker_id,
                                operation_id: i,
                                duration: operation_start.elapsed(),
                                success: true,
                                error: None,
                            });
                        },
                        Err(e) => {
                            results.push(LoadTestOperation {
                                worker_id,
                                operation_id: i,
                                duration: operation_start.elapsed(),
                                success: false,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
                
                results
            });
            
            handles.push(handle);
            
            // Ramp up gradually
            if self.ramp_up_time > Duration::from_secs(0) {
                let delay = self.ramp_up_time / self.concurrent_operations as u32;
                tokio::time::sleep(delay).await;
            }
        }
        
        let mut all_operations = Vec::new();
        for handle in handles {
            let operations = handle.await?;
            all_operations.extend(operations);
        }
        
        let total_duration = start_time.elapsed();
        let successful_operations = all_operations.iter().filter(|op| op.success).count();
        let failed_operations = all_operations.len() - successful_operations;
        
        let average_duration = if !all_operations.is_empty() {
            all_operations.iter().map(|op| op.duration).sum::<Duration>() / all_operations.len() as u32
        } else {
            Duration::from_secs(0)
        };
        
        Ok(LoadTestResults {
            total_duration,
            total_operations: all_operations.len(),
            successful_operations,
            failed_operations,
            average_duration,
            operations_per_second: successful_operations as f64 / total_duration.as_secs_f64(),
            operations: all_operations,
        })
    }
}

#[derive(Debug, Clone)]
pub struct LoadTestResults {
    pub total_duration: Duration,
    pub total_operations: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub average_duration: Duration,
    pub operations_per_second: f64,
    pub operations: Vec<LoadTestOperation>,
}

#[derive(Debug, Clone)]
pub struct LoadTestOperation {
    pub worker_id: usize,
    pub operation_id: usize,
    pub duration: Duration,
    pub success: bool,
    pub error: Option<String>,
}

/// Configuration builder for tests
pub struct TestConfigBuilder;

impl TestConfigBuilder {
    pub fn fast_config() -> AgentConfig {
        AgentConfig {
            name: "fast_test_agent".to_string(),
            description: "Fast configuration for testing".to_string(),
            capabilities: vec![AgentCapability::DataAnalysis],
            preferences: AgentPreferences {
                risk_tolerance: 0.8,
                max_transaction_value: Balance::from_lamports(1000),
                min_counterparty_reputation: 0.1,
                preferred_payment_methods: vec!["SOL".to_string()],
                auto_accept_threshold: 0.9,
                geographic_preferences: None,
            },
            ..Default::default()
        }
    }
    
    pub fn high_throughput_config() -> GossipConfig {
        GossipConfig {
            fanout: 10,
            gossip_interval: Duration::from_millis(100),
            message_ttl: 5,
            max_message_cache: 10000,
            duplicate_window: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(5),
            enable_anti_entropy: true,
            compression: true,
        }
    }
    
    pub fn minimal_discovery_config() -> DiscoveryConfig {
        DiscoveryConfig {
            bootstrap_nodes: vec!["127.0.0.1:8080".parse().unwrap()],
            max_peers: 10,
            discovery_interval: Duration::from_secs(5),
            peer_timeout: Duration::from_secs(30),
            enable_dht: false,
            enable_gossip: true,
            enable_mdns: false,
            reputation_threshold: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_blockchain_client() {
        let client = MockBlockchainClient::new();
        
        // Set initial balance
        {
            let mut accounts = client.accounts.write().await;
            accounts.insert("alice".to_string(), 1000);
        }
        
        // Test transfer
        let tx_id = client.transfer("alice", "bob", 500).await.unwrap();
        assert!(!tx_id.is_empty());
        
        // Check balances
        assert_eq!(client.get_balance("alice").await, 500);
        assert_eq!(client.get_balance("bob").await, 500);
        
        // Confirm transaction
        client.confirm_transaction(&tx_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_data_generator() {
        let config = TestDataGenerator::random_agent_config();
        assert!(!config.name.is_empty());
        assert!(!config.capabilities.is_empty());
        
        let request = TestDataGenerator::random_service_request();
        assert!(request.max_payment.lamports() > 0);
        
        let agents = TestDataGenerator::create_test_agents(5).await.unwrap();
        assert_eq!(agents.len(), 5);
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new();
        
        metrics.measure("test_operation", || {
            std::thread::sleep(Duration::from_millis(10));
            42
        });
        
        let result = metrics.measure_async("async_operation", || async {
            tokio::time::sleep(Duration::from_millis(5)).await;
            "done"
        }).await;
        
        assert_eq!(result, "done");
        
        let summary = metrics.get_summary();
        assert_eq!(summary.operations.len(), 2);
        assert!(summary.operations.contains_key("test_operation"));
        assert!(summary.operations.contains_key("async_operation"));
    }
} 