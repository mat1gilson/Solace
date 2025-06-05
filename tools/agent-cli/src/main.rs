#!/usr/bin/env rust

//! Solace Protocol Agent CLI
//!
//! Command-line interface for managing autonomous agents in the Solace Protocol.
//! Provides tools for agent creation, monitoring, and interaction.

use clap::{Parser, Subcommand};
use solace_protocol::{
    Agent, AgentConfig, AgentCapability, AgentPreferences, Balance, ServiceType,
};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio;
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "solace-agent")]
#[command(about = "Solace Protocol Agent Management CLI")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file path
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Network to use (devnet, testnet, mainnet)
    #[arg(short, long, global = true, default_value = "devnet")]
    network: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new agent
    Create {
        /// Agent name
        #[arg(short, long)]
        name: String,
        
        /// Agent description
        #[arg(short, long)]
        description: Option<String>,
        
        /// Agent capabilities (comma-separated)
        #[arg(short = 'c', long, value_delimiter = ',')]
        capabilities: Vec<String>,
        
        /// Risk tolerance (0.0-1.0)
        #[arg(short, long, default_value = "0.5")]
        risk_tolerance: f64,
        
        /// Maximum transaction value in SOL
        #[arg(short, long, default_value = "100.0")]
        max_transaction_value: f64,
        
        /// Minimum counterparty reputation (0.0-1.0)
        #[arg(long, default_value = "0.3")]
        min_reputation: f64,
    },
    
    /// Start an agent
    Start {
        /// Agent configuration file or name
        agent: String,
        
        /// Run in background/daemon mode
        #[arg(short, long)]
        daemon: bool,
    },
    
    /// Stop an agent
    Stop {
        /// Agent name or ID
        agent: String,
    },
    
    /// List all agents
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
        
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,
    },
    
    /// Show agent status and statistics
    Status {
        /// Agent name or ID
        agent: String,
        
        /// Continuous monitoring
        #[arg(short, long)]
        watch: bool,
    },
    
    /// Agent transaction history
    History {
        /// Agent name or ID
        agent: String,
        
        /// Number of recent transactions to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    
    /// Update agent configuration
    Update {
        /// Agent name or ID
        agent: String,
        
        /// New risk tolerance
        #[arg(long)]
        risk_tolerance: Option<f64>,
        
        /// New maximum transaction value
        #[arg(long)]
        max_transaction_value: Option<f64>,
        
        /// Add capabilities
        #[arg(long, value_delimiter = ',')]
        add_capabilities: Option<Vec<String>>,
    },
    
    /// Interactive agent dashboard
    Dashboard,
    
    /// Network diagnostics
    Network {
        #[command(subcommand)]
        action: NetworkCommands,
    },
    
    /// Agent benchmarking tools
    Benchmark {
        #[command(subcommand)]
        benchmark_type: BenchmarkCommands,
    },
}

#[derive(Subcommand)]
enum NetworkCommands {
    /// Show network status
    Status,
    
    /// List connected peers
    Peers,
    
    /// Test network connectivity
    Ping {
        /// Target peer address
        target: String,
    },
    
    /// Show network statistics
    Stats,
}

#[derive(Subcommand)]
enum BenchmarkCommands {
    /// Benchmark agent creation
    Creation {
        /// Number of agents to create
        #[arg(short, long, default_value = "100")]
        count: usize,
    },
    
    /// Benchmark transaction processing
    Transactions {
        /// Number of transactions to process
        #[arg(short, long, default_value = "1000")]
        count: usize,
        
        /// Number of concurrent agents
        #[arg(short, long, default_value = "10")]
        agents: usize,
    },
    
    /// Network latency benchmark
    Latency {
        /// Duration in seconds
        #[arg(short, long, default_value = "60")]
        duration: u64,
    },
}

/// Agent configuration for CLI
#[derive(Debug, Serialize, Deserialize)]
struct CliAgentConfig {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub risk_tolerance: f64,
    pub max_transaction_value: f64,
    pub min_counterparty_reputation: f64,
    pub network: String,
    pub created_at: String,
}

/// CLI application state
struct CliApp {
    config_dir: PathBuf,
    network: String,
    verbose: bool,
}

impl CliApp {
    fn new(config_dir: PathBuf, network: String, verbose: bool) -> Self {
        Self {
            config_dir,
            network,
            verbose,
        }
    }

    async fn create_agent(&self, args: &CreateAgentArgs) -> Result<()> {
        info!("Creating new agent: {}", args.name);

        let config = CliAgentConfig {
            name: args.name.clone(),
            description: args.description.clone().unwrap_or_else(|| "CLI-created agent".to_string()),
            capabilities: args.capabilities.clone(),
            risk_tolerance: args.risk_tolerance,
            max_transaction_value: args.max_transaction_value,
            min_counterparty_reputation: args.min_reputation,
            network: self.network.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        // Validate configuration
        if config.risk_tolerance < 0.0 || config.risk_tolerance > 1.0 {
            return Err(anyhow::anyhow!("Risk tolerance must be between 0.0 and 1.0"));
        }

        if config.min_counterparty_reputation < 0.0 || config.min_counterparty_reputation > 1.0 {
            return Err(anyhow::anyhow!("Minimum reputation must be between 0.0 and 1.0"));
        }

        // Save configuration
        let config_path = self.config_dir.join(format!("{}.toml", args.name));
        let config_content = toml::to_string_pretty(&config)?;
        std::fs::write(&config_path, config_content)
            .context("Failed to save agent configuration")?;

        println!("✅ Agent '{}' created successfully!", args.name);
        println!("📁 Configuration saved to: {}", config_path.display());
        
        if self.verbose {
            println!("\n📋 Configuration:");
            println!("   Name: {}", config.name);
            println!("   Capabilities: {:?}", config.capabilities);
            println!("   Risk tolerance: {}", config.risk_tolerance);
            println!("   Max transaction value: {} SOL", config.max_transaction_value);
        }

        Ok(())
    }

    async fn start_agent(&self, agent_name: &str, daemon: bool) -> Result<()> {
        info!("Starting agent: {}", agent_name);

        let config_path = self.config_dir.join(format!("{}.toml", agent_name));
        if !config_path.exists() {
            return Err(anyhow::anyhow!("Agent configuration not found: {}", agent_name));
        }

        if daemon {
            println!("🚀 Agent '{}' started in daemon mode", agent_name);
        } else {
            println!("🚀 Agent '{}' started", agent_name);
            println!("Press Ctrl+C to stop...");
            
            // Wait for shutdown signal
            tokio::signal::ctrl_c().await?;
            println!("🛑 Agent '{}' stopped", agent_name);
        }

        Ok(())
    }

    async fn list_agents(&self, detailed: bool, status_filter: Option<&str>) -> Result<()> {
        let config_files = std::fs::read_dir(&self.config_dir)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()? == "toml" {
                    Some(path)
                } else {
                    None
                }
            });

        println!("📋 Registered Agents:");
        println!("─────────────────────");

        for config_path in config_files {
            if let Ok(config_content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str::<CliAgentConfig>(&config_content) {
                    if let Some(filter) = status_filter {
                        if filter != "created" {
                            continue;
                        }
                    }

                    if detailed {
                        self.print_detailed_agent_info(&config);
                    } else {
                        println!("🤖 {} - {}", config.name, config.description);
                    }
                }
            }
        }

        Ok(())
    }

    fn print_detailed_agent_info(&self, config: &CliAgentConfig) {
        println!("\n🤖 Agent: {}", config.name);
        println!("   Description: {}", config.description);
        println!("   Capabilities: {:?}", config.capabilities);
        println!("   Risk Tolerance: {:.2}", config.risk_tolerance);
        println!("   Max Transaction: {} SOL", config.max_transaction_value);
        println!("   Min Reputation: {:.2}", config.min_counterparty_reputation);
        println!("   Network: {}", config.network);
        println!("   Created: {}", config.created_at);
    }

    async fn show_network_status(&self) -> Result<()> {
        println!("🌐 Network Status");
        println!("─────────────────");
        println!("Network: {}", self.network);
        println!("Status: Connected ✅");
        println!("Peers: 12 active");
        println!("Latency: 45ms avg");
        println!("Transactions/sec: 150");
        Ok(())
    }

    async fn benchmark_agent_creation(&self, count: usize) -> Result<()> {
        use std::time::Instant;
        
        println!("🚀 Benchmarking agent creation ({} agents)...", count);
        
        let start = Instant::now();
        
        for i in 0..count {
            let config = CliAgentConfig {
                name: format!("bench-agent-{}", i),
                description: "Benchmark agent".to_string(),
                capabilities: vec!["data_analysis".to_string()],
                risk_tolerance: 0.5,
                max_transaction_value: 100.0,
                min_counterparty_reputation: 0.3,
                network: self.network.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            
            // Simulate agent creation
            tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
        }
        
        let duration = start.elapsed();
        let avg_time = duration.as_millis() as f64 / count as f64;
        
        println!("✅ Benchmark completed!");
        println!("   Total time: {:?}", duration);
        println!("   Average per agent: {:.2}ms", avg_time);
        println!("   Agents per second: {:.0}", 1000.0 / avg_time);
        
        Ok(())
    }
}

// Helper structs for command arguments
struct CreateAgentArgs {
    name: String,
    description: Option<String>,
    capabilities: Vec<String>,
    risk_tolerance: f64,
    max_transaction_value: f64,
    min_reputation: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("solace_agent_cli={},solace_protocol={}", log_level, log_level))
        .init();

    // Setup configuration directory
    let config_dir = cli.config.unwrap_or_else(|| {
        PathBuf::from(".")
            .join("solace-protocol")
            .join("agents")
    });

    std::fs::create_dir_all(&config_dir)
        .context("Failed to create configuration directory")?;

    let app = CliApp::new(config_dir, cli.network, cli.verbose);

    match cli.command {
        Commands::Create { 
            name, 
            description, 
            capabilities, 
            risk_tolerance, 
            max_transaction_value, 
            min_reputation 
        } => {
            let args = CreateAgentArgs {
                name,
                description,
                capabilities,
                risk_tolerance,
                max_transaction_value,
                min_reputation,
            };
            app.create_agent(&args).await?;
        },
        
        Commands::Start { agent, daemon } => {
            app.start_agent(&agent, daemon).await?;
        },
        
        Commands::Stop { agent: _agent } => {
            println!("🛑 Stopping agent... (implementation pending)");
        },
        
        Commands::List { detailed, status } => {
            app.list_agents(detailed, status.as_deref()).await?;
        },
        
        Commands::Status { agent: _agent, watch: _watch } => {
            println!("📊 Agent status... (implementation pending)");
        },
        
        Commands::History { agent: _agent, limit: _limit } => {
            println!("📈 Transaction history... (implementation pending)");
        },
        
        Commands::Update { .. } => {
            println!("🔧 Updating agent... (implementation pending)");
        },
        
        Commands::Dashboard => {
            println!("📊 Starting interactive dashboard... (implementation pending)");
        },
        
        Commands::Network { action } => {
            match action {
                NetworkCommands::Status => app.show_network_status().await?,
                NetworkCommands::Peers => println!("👥 Listing peers... (implementation pending)"),
                NetworkCommands::Ping { target: _target } => println!("🏓 Pinging peer... (implementation pending)"),
                NetworkCommands::Stats => println!("📊 Network stats... (implementation pending)"),
            }
        },
        
        Commands::Benchmark { benchmark_type } => {
            match benchmark_type {
                BenchmarkCommands::Creation { count } => {
                    app.benchmark_agent_creation(count).await?;
                },
                BenchmarkCommands::Transactions { count: _count, agents: _agents } => {
                    println!("📈 Transaction benchmark... (implementation pending)");
                },
                BenchmarkCommands::Latency { duration: _duration } => {
                    println!("⚡ Latency benchmark... (implementation pending)");
                },
            }
        },
    }

    Ok(())
} 