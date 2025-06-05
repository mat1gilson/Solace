//! Performance Benchmarks for Solace Protocol
//!
//! Comprehensive benchmark suite testing various aspects of the protocol
//! including agent performance, transaction throughput, and network efficiency.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use solace_protocol::*;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

/// Agent creation benchmark
fn bench_agent_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("agent_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let config = AgentConfig::default();
            let agent = Agent::new(config).await.unwrap();
            black_box(agent);
        });
    });
}

/// Agent creation with varying capabilities
fn bench_agent_creation_with_capabilities(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("agent_creation_capabilities");
    
    for capability_count in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("capabilities", capability_count),
            capability_count,
            |b, &capability_count| {
                b.to_async(&rt).iter(|| async move {
                    let capabilities = (0..capability_count)
                        .map(|i| AgentCapability::CustomCapability(format!("capability_{}", i)))
                        .collect();
                    
                    let config = AgentConfig {
                        capabilities,
                        ..Default::default()
                    };
                    
                    let agent = Agent::new(config).await.unwrap();
                    black_box(agent);
                });
            },
        );
    }
    group.finish();
}

/// Transaction processing benchmark
fn bench_transaction_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("transaction_processing", |b| {
        b.to_async(&rt).iter(|| async {
            let requester_config = AgentConfig::default();
            let provider_config = AgentConfig::default();
            
            let requester = Agent::new(requester_config).await.unwrap();
            let provider = Agent::new(provider_config).await.unwrap();
            
            let request = ServiceRequest {
                id: TransactionId::new(),
                service_type: ServiceType::DataAnalysis,
                requirements: "Test analysis".to_string(),
                max_payment: Balance::from_lamports(1000),
                deadline: chrono::Utc::now() + chrono::Duration::hours(1),
                requester_id: requester.id().clone(),
            };
            
            let transaction = Transaction::new(request, provider.id().clone()).await.unwrap();
            black_box(transaction);
        });
    });
}

/// Batch transaction processing
fn bench_batch_transaction_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("batch_transaction_processing");
    
    for batch_size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async move {
                    let mut transactions = Vec::new();
                    
                    for i in 0..batch_size {
                        let request = ServiceRequest {
                            id: TransactionId::new(),
                            service_type: ServiceType::DataAnalysis,
                            requirements: format!("Test analysis {}", i),
                            max_payment: Balance::from_lamports(1000),
                            deadline: chrono::Utc::now() + chrono::Duration::hours(1),
                            requester_id: AgentId::new(),
                        };
                        
                        let transaction = Transaction::new(request, AgentId::new()).await.unwrap();
                        transactions.push(transaction);
                    }
                    
                    black_box(transactions);
                });
            },
        );
    }
    group.finish();
}

/// Reputation system benchmark
fn bench_reputation_calculation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("reputation_calculation", |b| {
        b.to_async(&rt).iter(|| async {
            let agent_id = AgentId::new();
            let mut reputation_system = ReputationSystem::new();
            
            // Add some transaction history
            for i in 0..100 {
                let transaction_id = TransactionId::new();
                let score = 0.8 + (i as f64 * 0.001); // Varying scores
                
                reputation_system.update_reputation(
                    &agent_id,
                    &transaction_id,
                    score,
                    1000,
                ).await.unwrap();
            }
            
            let reputation = reputation_system.get_reputation(&agent_id).await.unwrap();
            black_box(reputation);
        });
    });
}

/// Network discovery benchmark
fn bench_network_discovery(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("network_discovery", |b| {
        b.to_async(&rt).iter(|| async {
            let config = DiscoveryConfig::default();
            let mut discovery = PeerDiscovery::new(config);
            
            // Simulate discovering peers
            for i in 0..50 {
                let peer = PeerInfo {
                    id: format!("peer_{}", i),
                    address: format!("192.168.1.{}:8080", i + 1).parse().unwrap(),
                    public_key: format!("key_{}", i),
                    capabilities: vec!["agent".to_string()],
                    reputation: 0.8,
                    last_seen: chrono::Utc::now(),
                    protocol_version: "1.0.0".to_string(),
                    node_type: NodeType::Agent,
                };
                
                discovery.add_peer(peer, DiscoveryMethod::DHT).await;
            }
            
            let peers = discovery.get_known_peers();
            black_box(peers);
        });
    });
}

/// Gossip protocol benchmark
fn bench_gossip_protocol(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("gossip_message_processing", |b| {
        b.to_async(&rt).iter(|| async {
            let config = GossipConfig::default();
            let mut protocol = GossipProtocol::new("test_node".to_string(), config);
            
            // Add peers
            for i in 0..20 {
                protocol.add_peer(format!("peer_{}", i)).await;
            }
            
            // Create and process messages
            for i in 0..10 {
                let message = GossipMessage::new(
                    GossipMessageType::PeerAnnouncement,
                    format!("sender_{}", i),
                    serde_json::json!({"data": format!("message_{}", i)}),
                    10,
                );
                
                protocol.handle_incoming_message(message).await.unwrap();
            }
            
            let stats = protocol.get_stats().await;
            black_box(stats);
        });
    });
}

/// Memory usage benchmark
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("memory_usage_large_network", |b| {
        b.to_async(&rt).iter(|| async {
            let mut agents = Vec::new();
            
            // Create a large number of agents
            for i in 0..1000 {
                let config = AgentConfig {
                    name: format!("agent_{}", i),
                    capabilities: vec![
                        AgentCapability::DataAnalysis,
                        AgentCapability::ComputationalTask,
                    ],
                    ..Default::default()
                };
                
                let agent = Agent::new(config).await.unwrap();
                agents.push(agent);
            }
            
            black_box(agents);
        });
    });
}

/// Concurrent transaction processing
fn bench_concurrent_transactions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_transactions");
    
    for concurrency in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrency", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async move {
                    let tasks = (0..concurrency).map(|i| {
                        tokio::spawn(async move {
                            let request = ServiceRequest {
                                id: TransactionId::new(),
                                service_type: ServiceType::DataAnalysis,
                                requirements: format!("Concurrent analysis {}", i),
                                max_payment: Balance::from_lamports(1000),
                                deadline: chrono::Utc::now() + chrono::Duration::hours(1),
                                requester_id: AgentId::new(),
                            };
                            
                            let transaction = Transaction::new(request, AgentId::new()).await.unwrap();
                            transaction
                        })
                    });
                    
                    let results = futures::future::join_all(tasks).await;
                    black_box(results);
                });
            },
        );
    }
    group.finish();
}

/// Cryptographic operations benchmark
fn bench_crypto_operations(c: &mut Criterion) {
    c.bench_function("signature_generation", |b| {
        b.iter(|| {
            let keypair = ed25519_dalek::Keypair::generate(&mut rand::rngs::OsRng);
            let message = b"test message for signing";
            let signature = keypair.sign(message);
            black_box(signature);
        });
    });
    
    c.bench_function("signature_verification", |b| {
        let keypair = ed25519_dalek::Keypair::generate(&mut rand::rngs::OsRng);
        let message = b"test message for signing";
        let signature = keypair.sign(message);
        
        b.iter(|| {
            let verification_result = keypair.public.verify(message, &signature);
            black_box(verification_result);
        });
    });
}

/// Network latency simulation benchmark
fn bench_network_latency_simulation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("network_latency");
    
    for latency_ms in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("latency_ms", latency_ms),
            latency_ms,
            |b, &latency_ms| {
                b.to_async(&rt).iter(|| async move {
                    let start = Instant::now();
                    
                    // Simulate network operation with latency
                    tokio::time::sleep(Duration::from_millis(latency_ms)).await;
                    
                    let elapsed = start.elapsed();
                    black_box(elapsed);
                });
            },
        );
    }
    group.finish();
}

/// JSON serialization/deserialization benchmark
fn bench_json_operations(c: &mut Criterion) {
    let request = ServiceRequest {
        id: TransactionId::new(),
        service_type: ServiceType::DataAnalysis,
        requirements: "Complex data analysis with multiple parameters".to_string(),
        max_payment: Balance::from_lamports(5000),
        deadline: chrono::Utc::now() + chrono::Duration::hours(2),
        requester_id: AgentId::new(),
    };
    
    c.bench_function("json_serialization", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&request).unwrap();
            black_box(json);
        });
    });
    
    let json_data = serde_json::to_string(&request).unwrap();
    c.bench_function("json_deserialization", |b| {
        b.iter(|| {
            let parsed: ServiceRequest = serde_json::from_str(&json_data).unwrap();
            black_box(parsed);
        });
    });
}

/// AI decision making benchmark
fn bench_ai_decisions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("ai_pricing_decision", |b| {
        b.to_async(&rt).iter(|| async {
            let ai_module = AIModule::new().await;
            
            let context = NegotiationContext {
                service_request: ServiceRequest {
                    id: TransactionId::new(),
                    service_type: ServiceType::DataAnalysis,
                    requirements: "AI pricing analysis".to_string(),
                    max_payment: Balance::from_lamports(10000),
                    deadline: chrono::Utc::now() + chrono::Duration::hours(1),
                    requester_id: AgentId::new(),
                },
                market_conditions: MarketConditions::default(),
                agent_reputation: 0.8,
                historical_pricing: vec![1000, 1200, 950, 1100],
            };
            
            let decision = ai_module.make_pricing_decision(&context).await.unwrap();
            black_box(decision);
        });
    });
}

criterion_group!(
    benches,
    bench_agent_creation,
    bench_agent_creation_with_capabilities,
    bench_transaction_processing,
    bench_batch_transaction_processing,
    bench_reputation_calculation,
    bench_network_discovery,
    bench_gossip_protocol,
    bench_memory_usage,
    bench_concurrent_transactions,
    bench_crypto_operations,
    bench_network_latency_simulation,
    bench_json_operations,
    bench_ai_decisions
);

criterion_main!(benches)

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_benchmark_setup() {
        // Ensure benchmark components can be created
        let config = AgentConfig::default();
        let agent = Agent::new(config).await.unwrap();
        assert!(!agent.id().to_string().is_empty());
    }
    
    #[tokio::test]
    async fn test_concurrent_agent_creation() {
        let tasks = (0..10).map(|i| {
            tokio::spawn(async move {
                let config = AgentConfig {
                    name: format!("concurrent_agent_{}", i),
                    ..Default::default()
                };
                Agent::new(config).await.unwrap()
            })
        });
        
        let agents = futures::future::join_all(tasks).await;
        assert_eq!(agents.len(), 10);
        
        for agent_result in agents {
            assert!(agent_result.is_ok());
        }
    }
    
    #[tokio::test]
    async fn test_transaction_throughput() {
        let start = Instant::now();
        let transaction_count = 100;
        
        let mut transactions = Vec::new();
        for i in 0..transaction_count {
            let request = ServiceRequest {
                id: TransactionId::new(),
                service_type: ServiceType::DataAnalysis,
                requirements: format!("Throughput test {}", i),
                max_payment: Balance::from_lamports(1000),
                deadline: chrono::Utc::now() + chrono::Duration::hours(1),
                requester_id: AgentId::new(),
            };
            
            let transaction = Transaction::new(request, AgentId::new()).await.unwrap();
            transactions.push(transaction);
        }
        
        let elapsed = start.elapsed();
        let tps = transaction_count as f64 / elapsed.as_secs_f64();
        
        println!("Transaction throughput: {:.2} TPS", tps);
        assert!(tps > 100.0); // Should handle at least 100 TPS
    }
} 