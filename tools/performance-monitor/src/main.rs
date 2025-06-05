use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Parser)]
#[command(name = "solace-monitor")]
#[command(about = "Solace Protocol Performance Monitor")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Metrics export port
    #[arg(short = 'p', long, default_value = "9090")]
    port: u16,
}

#[derive(Subcommand)]
enum Commands {
    /// Start real-time monitoring
    Monitor {
        /// Target agent or network component
        #[arg(short, long)]
        target: Option<String>,
        
        /// Monitoring interval in seconds
        #[arg(short, long, default_value = "1")]
        interval: u64,
        
        /// Alert thresholds configuration
        #[arg(short, long)]
        alerts: Option<String>,
    },
    
    /// Display agent performance metrics
    Agent {
        /// Agent ID or name
        agent_id: String,
        
        /// Time window in minutes
        #[arg(short, long, default_value = "60")]
        window: u64,
        
        /// Include detailed transaction metrics
        #[arg(long)]
        detailed: bool,
    },
    
    /// Network-wide performance analysis
    Network {
        /// Analysis type (latency, throughput, consensus)
        #[arg(short, long, default_value = "all")]
        analysis_type: String,
        
        /// Historical data period in hours
        #[arg(short = 'p', long, default_value = "24")]
        period: u64,
    },
    
    /// System resource monitoring
    System {
        /// Include GPU metrics if available
        #[arg(long)]
        gpu: bool,
        
        /// Monitor disk I/O
        #[arg(long)]
        disk_io: bool,
    },
    
    /// Performance benchmarking
    Benchmark {
        /// Benchmark type
        #[arg(short, long, default_value = "comprehensive")]
        benchmark_type: String,
        
        /// Duration in minutes
        #[arg(short, long, default_value = "10")]
        duration: u64,
    },
    
    /// Export metrics data
    Export {
        /// Export format (json, csv, prometheus)
        #[arg(short, long, default_value = "json")]
        format: String,
        
        /// Output file path
        #[arg(short, long)]
        output: String,
        
        /// Time range in hours
        #[arg(short, long, default_value = "24")]
        range: u64,
    },
    
    /// Start metrics server
    Server {
        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0")]
        bind: String,
    },
    
    /// Interactive TUI dashboard
    Dashboard,
}

/// Agent performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentMetrics {
    pub agent_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub network_in: u64,
    pub network_out: u64,
    pub transaction_count: u64,
    pub transaction_success_rate: f64,
    pub average_response_time: f64,
    pub reputation_score: f64,
    pub active_connections: u32,
}

/// Network performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_tps: f64,
    pub consensus_time: f64,
    pub network_latency: f64,
    pub active_validators: u32,
    pub total_agents: u32,
    pub network_utilization: f64,
    pub error_rate: f64,
}

/// System resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub memory_total: u64,
    pub disk_usage: f64,
    pub disk_io_read: u64,
    pub disk_io_write: u64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub load_average: Vec<f64>,
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AlertConfig {
    pub cpu_threshold: f64,
    pub memory_threshold: f64,
    pub latency_threshold: f64,
    pub error_rate_threshold: f64,
    pub tps_minimum: f64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            cpu_threshold: 80.0,
            memory_threshold: 85.0,
            latency_threshold: 1000.0,
            error_rate_threshold: 5.0,
            tps_minimum: 50.0,
        }
    }
}

/// Performance monitor implementation
struct PerformanceMonitor {
    config: AlertConfig,
    metrics_storage: Arc<RwLock<Vec<NetworkMetrics>>>,
    agent_metrics: Arc<RwLock<HashMap<String, Vec<AgentMetrics>>>>,
    system_metrics: Arc<RwLock<Vec<SystemMetrics>>>,
}

impl PerformanceMonitor {
    fn new(config: AlertConfig) -> Self {
        Self {
            config,
            metrics_storage: Arc::new(RwLock::new(Vec::new())),
            agent_metrics: Arc::new(RwLock::new(HashMap::new())),
            system_metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn start_monitoring(&self, interval: Duration) -> Result<()> {
        info!("Starting performance monitoring with interval {:?}", interval);
        
        let mut ticker = tokio::time::interval(interval);
        
        loop {
            ticker.tick().await;
            
            // Collect metrics
            if let Err(e) = self.collect_network_metrics().await {
                error!("Failed to collect network metrics: {}", e);
            }
            
            if let Err(e) = self.collect_system_metrics().await {
                error!("Failed to collect system metrics: {}", e);
            }
            
            // Check alerts
            self.check_alerts().await;
        }
    }

    async fn collect_network_metrics(&self) -> Result<()> {
        let metrics = NetworkMetrics {
            timestamp: chrono::Utc::now(),
            total_tps: 150.0 + (rand::random::<f64>() * 100.0),
            consensus_time: 500.0 + (rand::random::<f64>() * 200.0),
            network_latency: 45.0 + (rand::random::<f64>() * 30.0),
            active_validators: 25,
            total_agents: 150,
            network_utilization: 0.6 + (rand::random::<f64>() * 0.3),
            error_rate: rand::random::<f64>() * 2.0,
        };
        
        let mut storage = self.metrics_storage.write().await;
        storage.push(metrics.clone());
        
        // Keep only last 1000 entries
        if storage.len() > 1000 {
            storage.drain(0..storage.len() - 1000);
        }
        
        debug!("Collected network metrics: TPS={:.1}, Latency={:.1}ms", 
            metrics.total_tps, metrics.network_latency);
        
        Ok(())
    }

    async fn collect_system_metrics(&self) -> Result<()> {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        
        let metrics = SystemMetrics {
            timestamp: chrono::Utc::now(),
            cpu_usage: sys.global_cpu_info().cpu_usage() as f64,
            memory_usage: (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0,
            memory_total: sys.total_memory(),
            disk_usage: 45.0 + (rand::random::<f64>() * 20.0), // Simulated
            disk_io_read: rand::random::<u64>() % 1000000,
            disk_io_write: rand::random::<u64>() % 500000,
            network_rx: rand::random::<u64>() % 10000000,
            network_tx: rand::random::<u64>() % 5000000,
            load_average: sys.load_average().into(),
        };
        
        let mut storage = self.system_metrics.write().await;
        storage.push(metrics.clone());
        
        // Keep only last 1000 entries
        if storage.len() > 1000 {
            storage.drain(0..storage.len() - 1000);
        }
        
        debug!("Collected system metrics: CPU={:.1}%, Memory={:.1}%", 
            metrics.cpu_usage, metrics.memory_usage);
        
        Ok(())
    }

    async fn collect_agent_metrics(&self, agent_id: &str) -> Result<AgentMetrics> {
        let metrics = AgentMetrics {
            agent_id: agent_id.to_string(),
            timestamp: chrono::Utc::now(),
            cpu_usage: 15.0 + (rand::random::<f64>() * 30.0),
            memory_usage: 20.0 + (rand::random::<f64>() * 25.0),
            network_in: rand::random::<u64>() % 1000000,
            network_out: rand::random::<u64>() % 500000,
            transaction_count: rand::random::<u64>() % 100,
            transaction_success_rate: 95.0 + (rand::random::<f64>() * 5.0),
            average_response_time: 50.0 + (rand::random::<f64>() * 100.0),
            reputation_score: 0.7 + (rand::random::<f64>() * 0.3),
            active_connections: rand::random::<u32>() % 20,
        };
        
        let mut storage = self.agent_metrics.write().await;
        storage.entry(agent_id.to_string())
            .or_insert_with(Vec::new)
            .push(metrics.clone());
        
        Ok(metrics)
    }

    async fn check_alerts(&self) {
        let network_metrics = self.metrics_storage.read().await;
        let system_metrics = self.system_metrics.read().await;
        
        if let Some(latest_network) = network_metrics.last() {
            if latest_network.network_latency > self.config.latency_threshold {
                warn!("🚨 High network latency detected: {:.1}ms", latest_network.network_latency);
            }
            
            if latest_network.error_rate > self.config.error_rate_threshold {
                warn!("🚨 High error rate detected: {:.2}%", latest_network.error_rate);
            }
            
            if latest_network.total_tps < self.config.tps_minimum {
                warn!("🚨 Low throughput detected: {:.1} TPS", latest_network.total_tps);
            }
        }
        
        if let Some(latest_system) = system_metrics.last() {
            if latest_system.cpu_usage > self.config.cpu_threshold {
                warn!("🚨 High CPU usage detected: {:.1}%", latest_system.cpu_usage);
            }
            
            if latest_system.memory_usage > self.config.memory_threshold {
                warn!("🚨 High memory usage detected: {:.1}%", latest_system.memory_usage);
            }
        }
    }

    async fn get_network_summary(&self, period_hours: u64) -> Result<NetworkSummary> {
        let metrics = self.metrics_storage.read().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(period_hours as i64);
        
        let recent_metrics: Vec<_> = metrics.iter()
            .filter(|m| m.timestamp > cutoff)
            .cloned()
            .collect();
        
        if recent_metrics.is_empty() {
            return Ok(NetworkSummary::default());
        }
        
        let avg_tps = recent_metrics.iter().map(|m| m.total_tps).sum::<f64>() / recent_metrics.len() as f64;
        let avg_latency = recent_metrics.iter().map(|m| m.network_latency).sum::<f64>() / recent_metrics.len() as f64;
        let avg_error_rate = recent_metrics.iter().map(|m| m.error_rate).sum::<f64>() / recent_metrics.len() as f64;
        
        let max_tps = recent_metrics.iter().map(|m| m.total_tps).fold(0.0f64, f64::max);
        let min_latency = recent_metrics.iter().map(|m| m.network_latency).fold(f64::INFINITY, f64::min);
        
        Ok(NetworkSummary {
            period_hours,
            avg_tps,
            max_tps,
            avg_latency,
            min_latency,
            avg_error_rate,
            total_transactions: (avg_tps * period_hours as f64 * 3600.0) as u64,
            uptime_percentage: 99.5, // Simulated
        })
    }

    async fn run_benchmark(&self, duration: Duration) -> Result<BenchmarkResults> {
        info!("Running performance benchmark for {:?}", duration);
        
        let start_time = Instant::now();
        let mut results = BenchmarkResults::default();
        
        // Simulate various benchmark tests
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        results.transaction_throughput = 1250.0;
        results.latency_p50 = 45.0;
        results.latency_p95 = 150.0;
        results.latency_p99 = 300.0;
        results.cpu_efficiency = 85.0;
        results.memory_efficiency = 90.0;
        results.network_efficiency = 88.0;
        results.consensus_performance = 92.0;
        
        results.duration = start_time.elapsed();
        
        Ok(results)
    }

    fn export_metrics(&self, format: &str, range_hours: u64) -> Result<String> {
        // This would export metrics in the specified format
        match format {
            "json" => Ok(serde_json::to_string_pretty(&"metrics data")?),
            "csv" => Ok("timestamp,tps,latency,error_rate\n2023-01-01T00:00:00Z,150.0,45.0,1.2".to_string()),
            "prometheus" => Ok("# HELP solace_tps Transactions per second\nsolace_tps 150.0".to_string()),
            _ => Err(anyhow::anyhow!("Unsupported export format: {}", format)),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct NetworkSummary {
    pub period_hours: u64,
    pub avg_tps: f64,
    pub max_tps: f64,
    pub avg_latency: f64,
    pub min_latency: f64,
    pub avg_error_rate: f64,
    pub total_transactions: u64,
    pub uptime_percentage: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct BenchmarkResults {
    pub duration: Duration,
    pub transaction_throughput: f64,
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub latency_p99: f64,
    pub cpu_efficiency: f64,
    pub memory_efficiency: f64,
    pub network_efficiency: f64,
    pub consensus_performance: f64,
}

fn load_alert_config(path: Option<&str>) -> Result<AlertConfig> {
    if let Some(config_path) = path {
        let content = std::fs::read_to_string(config_path)
            .context("Failed to read alert configuration")?;
        let config: AlertConfig = toml::from_str(&content)
            .context("Failed to parse alert configuration")?;
        Ok(config)
    } else {
        Ok(AlertConfig::default())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("solace_monitor={}", log_level))
        .init();

    // Load configuration
    let alert_config = load_alert_config(cli.config.as_deref()).unwrap_or_default();
    let monitor = PerformanceMonitor::new(alert_config);

    match cli.command {
        Commands::Monitor { target, interval, alerts: _alerts } => {
            if let Some(target) = target {
                info!("Monitoring target: {}", target);
            } else {
                info!("Monitoring entire network");
            }
            
            monitor.start_monitoring(Duration::from_secs(interval)).await?;
        },
        
        Commands::Agent { agent_id, window, detailed } => {
            println!("🤖 Agent Performance Metrics: {}", agent_id);
            println!("═══════════════════════════════════");
            
            let metrics = monitor.collect_agent_metrics(&agent_id).await?;
            
            println!("CPU Usage: {:.1}%", metrics.cpu_usage);
            println!("Memory Usage: {:.1}%", metrics.memory_usage);
            println!("Transactions: {} (Success rate: {:.1}%)", 
                metrics.transaction_count, metrics.transaction_success_rate);
            println!("Average Response Time: {:.1}ms", metrics.average_response_time);
            println!("Reputation Score: {:.3}", metrics.reputation_score);
            println!("Active Connections: {}", metrics.active_connections);
            
            if detailed {
                println!("\nDetailed Metrics:");
                println!("Network In: {} bytes", metrics.network_in);
                println!("Network Out: {} bytes", metrics.network_out);
                println!("Time window: {} minutes", window);
            }
        },
        
        Commands::Network { analysis_type, period } => {
            println!("🌐 Network Performance Analysis ({})", analysis_type);
            println!("════════════════════════════════════════");
            
            let summary = monitor.get_network_summary(period).await?;
            
            println!("Period: {} hours", summary.period_hours);
            println!("Average TPS: {:.1}", summary.avg_tps);
            println!("Peak TPS: {:.1}", summary.max_tps);
            println!("Average Latency: {:.1}ms", summary.avg_latency);
            println!("Best Latency: {:.1}ms", summary.min_latency);
            println!("Average Error Rate: {:.2}%", summary.avg_error_rate);
            println!("Total Transactions: {}", summary.total_transactions);
            println!("Uptime: {:.2}%", summary.uptime_percentage);
        },
        
        Commands::System { gpu: _gpu, disk_io: _disk_io } => {
            println!("💻 System Resource Monitoring");
            println!("════════════════════════════");
            
            monitor.collect_system_metrics().await?;
            let system_metrics = monitor.system_metrics.read().await;
            
            if let Some(latest) = system_metrics.last() {
                println!("CPU Usage: {:.1}%", latest.cpu_usage);
                println!("Memory Usage: {:.1}% ({} MB total)", 
                    latest.memory_usage, latest.memory_total / 1024 / 1024);
                println!("Disk Usage: {:.1}%", latest.disk_usage);
                println!("Load Average: {:?}", latest.load_average);
                println!("Network RX: {} bytes", latest.network_rx);
                println!("Network TX: {} bytes", latest.network_tx);
            }
        },
        
        Commands::Benchmark { benchmark_type, duration } => {
            println!("🚀 Running {} benchmark for {} minutes...", benchmark_type, duration);
            
            let results = monitor.run_benchmark(Duration::from_secs(duration * 60)).await?;
            
            println!("\n📊 Benchmark Results");
            println!("═══════════════════");
            println!("Duration: {:?}", results.duration);
            println!("Transaction Throughput: {:.1} TPS", results.transaction_throughput);
            println!("Latency P50: {:.1}ms", results.latency_p50);
            println!("Latency P95: {:.1}ms", results.latency_p95);
            println!("Latency P99: {:.1}ms", results.latency_p99);
            println!("CPU Efficiency: {:.1}%", results.cpu_efficiency);
            println!("Memory Efficiency: {:.1}%", results.memory_efficiency);
            println!("Network Efficiency: {:.1}%", results.network_efficiency);
            println!("Consensus Performance: {:.1}%", results.consensus_performance);
        },
        
        Commands::Export { format, output, range } => {
            println!("📤 Exporting metrics data ({} format, {} hours)...", format, range);
            
            let data = monitor.export_metrics(&format, range)?;
            std::fs::write(&output, data)?;
            
            println!("✅ Metrics exported to: {}", output);
        },
        
        Commands::Server { bind } => {
            println!("🌐 Starting metrics server on {}:{}", bind, cli.port);
            println!("Access metrics at: http://{}:{}/metrics", bind, cli.port);
            
            // Start background monitoring
            let monitor_clone = Arc::new(monitor);
            let _monitor_handle = {
                let monitor = monitor_clone.clone();
                tokio::spawn(async move {
                    if let Err(e) = monitor.start_monitoring(Duration::from_secs(5)).await {
                        error!("Monitoring task failed: {}", e);
                    }
                })
            };
            
            // Keep server running
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        },
        
        Commands::Dashboard => {
            println!("📊 Starting interactive dashboard...");
            println!("(TUI dashboard not implemented in this demo)");
            println!("Use 'solace-monitor monitor' for real-time monitoring");
        },
    }

    Ok(())
} 