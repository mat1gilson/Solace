//! Blockchain Integration Module
//!
//! Provides comprehensive integration with the Solana blockchain,
//! including transaction submission, account management, and smart contract interaction.

use std::str::FromStr;
use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::{info, warn, debug, error};

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};

use crate::{
    AgentId, TransactionId, Balance, 
    error::SolaceError,
    types::Hash,
};

/// Blockchain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    /// Solana RPC endpoint URL
    pub rpc_url: String,
    /// Commitment level for transactions
    pub commitment: CommitmentLevel,
    /// Transaction confirmation timeout
    pub confirmation_timeout: Duration,
    /// Maximum retry attempts for failed transactions
    pub max_retries: u32,
    /// Fee payer keypair path (optional)
    pub fee_payer_path: Option<String>,
    /// Program ID for Solace smart contracts
    pub program_id: String,
    /// Skip preflight checks
    pub skip_preflight: bool,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            commitment: CommitmentLevel::Confirmed,
            confirmation_timeout: Duration::from_secs(60),
            max_retries: 3,
            fee_payer_path: None,
            program_id: "SoLaCeProgram1111111111111111111111111111111".to_string(),
            skip_preflight: false,
        }
    }
}

/// Commitment levels for transaction confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitmentLevel {
    Processed,
    Confirmed,
    Finalized,
}

impl From<CommitmentLevel> for CommitmentConfig {
    fn from(level: CommitmentLevel) -> Self {
        match level {
            CommitmentLevel::Processed => CommitmentConfig::processed(),
            CommitmentLevel::Confirmed => CommitmentConfig::confirmed(),
            CommitmentLevel::Finalized => CommitmentConfig::finalized(),
        }
    }
}

/// Solana account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub pubkey: Pubkey,
    pub lamports: u64,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: u64,
    pub data: Vec<u8>,
}

/// Transaction result from blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainTransactionResult {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub confirmation_status: ConfirmationStatus,
    pub fee: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfirmationStatus {
    Processed,
    Confirmed,
    Finalized,
    Failed,
}

/// Smart contract instruction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolaceInstruction {
    InitializeAgent {
        agent_id: AgentId,
        initial_reputation: u32,
    },
    CreateTransaction {
        transaction_id: TransactionId,
        amount: u64,
        recipient: Pubkey,
    },
    UpdateReputation {
        agent_id: AgentId,
        new_reputation: u32,
    },
    FinalizeTransaction {
        transaction_id: TransactionId,
        success: bool,
    },
    Stake {
        amount: u64,
    },
    Unstake {
        amount: u64,
    },
    Vote {
        proposal_id: String,
        vote: bool,
    },
}

/// Blockchain client for Solana interaction
pub struct SolanaClient {
    client: RpcClient,
    config: BlockchainConfig,
    program_id: Pubkey,
    fee_payer: Option<Keypair>,
}

impl SolanaClient {
    /// Create a new Solana client
    pub fn new(config: BlockchainConfig) -> Result<Self> {
        let client = RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            config.commitment.clone().into(),
        );

        let program_id = Pubkey::from_str(&config.program_id)
            .map_err(|e| SolaceError::InvalidPubkey(e.to_string()))?;

        let fee_payer = if let Some(path) = &config.fee_payer_path {
            Some(read_keypair_file(path)?)
        } else {
            None
        };

        Ok(Self {
            client,
            config,
            program_id,
            fee_payer,
        })
    }

    /// Get account information
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<AccountInfo>> {
        match self.client.get_account(pubkey) {
            Ok(account) => Ok(Some(AccountInfo {
                pubkey: *pubkey,
                lamports: account.lamports,
                owner: account.owner,
                executable: account.executable,
                rent_epoch: account.rent_epoch,
                data: account.data,
            })),
            Err(e) => {
                if e.to_string().contains("AccountNotFound") {
                    Ok(None)
                } else {
                    Err(SolaceError::BlockchainError(e.to_string()).into())
                }
            }
        }
    }

    /// Get account balance in lamports
    pub async fn get_balance(&self, pubkey: &Pubkey) -> Result<u64> {
        self.client
            .get_balance(pubkey)
            .map_err(|e| SolaceError::BlockchainError(e.to_string()).into())
    }

    /// Send SOL from one account to another
    pub async fn transfer(
        &self,
        from_keypair: &Keypair,
        to_pubkey: &Pubkey,
        amount_lamports: u64,
    ) -> Result<BlockchainTransactionResult> {
        let recent_blockhash = self.client.get_latest_blockhash()
            .map_err(|e| SolaceError::BlockchainError(e.to_string()))?;

        let transfer_instruction = system_instruction::transfer(
            &from_keypair.pubkey(),
            to_pubkey,
            amount_lamports,
        );

        let message = Message::new(&[transfer_instruction], Some(&from_keypair.pubkey()));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.sign(&[from_keypair], recent_blockhash);

        self.send_transaction_with_confirmation(transaction).await
    }

    /// Submit a Solace protocol instruction
    pub async fn submit_instruction(
        &self,
        instruction: SolaceInstruction,
        signer: &Keypair,
        additional_accounts: Vec<AccountMeta>,
    ) -> Result<BlockchainTransactionResult> {
        let instruction_data = self.serialize_instruction(&instruction)?;
        
        let mut accounts = vec![
            AccountMeta::new(signer.pubkey(), true),
            AccountMeta::new_readonly(self.program_id, false),
        ];
        accounts.extend(additional_accounts);

        let solana_instruction = Instruction {
            program_id: self.program_id,
            accounts,
            data: instruction_data,
        };

        let recent_blockhash = self.client.get_latest_blockhash()
            .map_err(|e| SolaceError::BlockchainError(e.to_string()))?;

        let message = Message::new(&[solana_instruction], Some(&signer.pubkey()));
        let mut transaction = Transaction::new_unsigned(message);
        transaction.sign(&[signer], recent_blockhash);

        self.send_transaction_with_confirmation(transaction).await
    }

    /// Initialize a new agent on the blockchain
    pub async fn initialize_agent(
        &self,
        agent_keypair: &Keypair,
        agent_id: AgentId,
        initial_reputation: f64,
    ) -> Result<BlockchainTransactionResult> {
        let reputation_scaled = (initial_reputation * 1000.0) as u32;
        
        let instruction = SolaceInstruction::InitializeAgent {
            agent_id,
            initial_reputation: reputation_scaled,
        };

        self.submit_instruction(instruction, agent_keypair, vec![]).await
    }

    /// Create a transaction record on the blockchain
    pub async fn create_blockchain_transaction(
        &self,
        creator_keypair: &Keypair,
        transaction_id: TransactionId,
        amount: Balance,
        recipient: Pubkey,
    ) -> Result<BlockchainTransactionResult> {
        let instruction = SolaceInstruction::CreateTransaction {
            transaction_id,
            amount: amount.lamports(),
            recipient,
        };

        self.submit_instruction(instruction, creator_keypair, vec![
            AccountMeta::new(recipient, false),
        ]).await
    }

    /// Update agent reputation on the blockchain
    pub async fn update_reputation(
        &self,
        agent_keypair: &Keypair,
        agent_id: AgentId,
        new_reputation: f64,
    ) -> Result<BlockchainTransactionResult> {
        let reputation_scaled = (new_reputation * 1000.0) as u32;
        
        let instruction = SolaceInstruction::UpdateReputation {
            agent_id,
            new_reputation: reputation_scaled,
        };

        self.submit_instruction(instruction, agent_keypair, vec![]).await
    }

    /// Finalize a transaction on the blockchain
    pub async fn finalize_transaction(
        &self,
        finalizer_keypair: &Keypair,
        transaction_id: TransactionId,
        success: bool,
    ) -> Result<BlockchainTransactionResult> {
        let instruction = SolaceInstruction::FinalizeTransaction {
            transaction_id,
            success,
        };

        self.submit_instruction(instruction, finalizer_keypair, vec![]).await
    }

    /// Stake tokens for consensus participation
    pub async fn stake(
        &self,
        staker_keypair: &Keypair,
        amount: Balance,
    ) -> Result<BlockchainTransactionResult> {
        let instruction = SolaceInstruction::Stake {
            amount: amount.lamports(),
        };

        self.submit_instruction(instruction, staker_keypair, vec![]).await
    }

    /// Unstake tokens
    pub async fn unstake(
        &self,
        staker_keypair: &Keypair,
        amount: Balance,
    ) -> Result<BlockchainTransactionResult> {
        let instruction = SolaceInstruction::Unstake {
            amount: amount.lamports(),
        };

        self.submit_instruction(instruction, staker_keypair, vec![]).await
    }

    /// Submit a governance vote
    pub async fn vote(
        &self,
        voter_keypair: &Keypair,
        proposal_id: String,
        vote: bool,
    ) -> Result<BlockchainTransactionResult> {
        let instruction = SolaceInstruction::Vote {
            proposal_id,
            vote,
        };

        self.submit_instruction(instruction, voter_keypair, vec![]).await
    }

    /// Get transaction history for an account
    pub async fn get_transaction_history(
        &self,
        pubkey: &Pubkey,
        limit: usize,
    ) -> Result<Vec<BlockchainTransactionResult>> {
        let signatures = self.client
            .get_signatures_for_address_with_config(
                pubkey,
                solana_client::rpc_client_api::config::GetConfirmedSignaturesForAddress2Config {
                    limit: Some(limit),
                    ..Default::default()
                },
            )
            .map_err(|e| SolaceError::BlockchainError(e.to_string()))?;

        let mut results = Vec::new();
        for signature_info in signatures {
            if let Ok(signature) = Signature::from_str(&signature_info.signature) {
                if let Ok(transaction) = self.client.get_transaction(
                    &signature,
                    solana_sdk::transaction_status::UiTransactionEncoding::Json,
                ) {
                    results.push(BlockchainTransactionResult {
                        signature: signature_info.signature,
                        slot: signature_info.slot,
                        block_time: signature_info.block_time,
                        confirmation_status: if signature_info.confirmation_status.is_some() {
                            ConfirmationStatus::Confirmed
                        } else {
                            ConfirmationStatus::Processed
                        },
                        fee: transaction.transaction.meta
                            .and_then(|meta| meta.fee)
                            .unwrap_or(0),
                        error: signature_info.err.map(|e| format!("{:?}", e)),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Get current network status
    pub async fn get_network_status(&self) -> Result<NetworkStatus> {
        let health = self.client.get_health()
            .map_err(|e| SolaceError::BlockchainError(e.to_string()))?;

        let slot = self.client.get_slot()
            .map_err(|e| SolaceError::BlockchainError(e.to_string()))?;

        let epoch_info = self.client.get_epoch_info()
            .map_err(|e| SolaceError::BlockchainError(e.to_string()))?;

        Ok(NetworkStatus {
            health: health.to_string(),
            slot,
            epoch: epoch_info.epoch,
            block_height: epoch_info.block_height,
            absolute_slot: epoch_info.absolute_slot,
        })
    }

    /// Send transaction with confirmation
    async fn send_transaction_with_confirmation(
        &self,
        transaction: Transaction,
    ) -> Result<BlockchainTransactionResult> {
        let signature = self.client
            .send_and_confirm_transaction_with_spinner_and_config(
                &transaction,
                self.config.commitment.clone().into(),
                RpcSendTransactionConfig {
                    skip_preflight: self.config.skip_preflight,
                    ..Default::default()
                },
            )
            .map_err(|e| SolaceError::BlockchainError(e.to_string()))?;

        // Get transaction details
        let transaction_result = self.client
            .get_transaction(
                &signature,
                solana_sdk::transaction_status::UiTransactionEncoding::Json,
            )
            .map_err(|e| SolaceError::BlockchainError(e.to_string()))?;

        Ok(BlockchainTransactionResult {
            signature: signature.to_string(),
            slot: transaction_result.slot,
            block_time: transaction_result.block_time,
            confirmation_status: ConfirmationStatus::Confirmed,
            fee: transaction_result.transaction.meta
                .and_then(|meta| meta.fee)
                .unwrap_or(0),
            error: None,
        })
    }

    /// Serialize instruction data
    fn serialize_instruction(&self, instruction: &SolaceInstruction) -> Result<Vec<u8>> {
        // In a real implementation, this would use a proper serialization format
        // like Borsh that matches the on-chain program expectations
        serde_json::to_vec(instruction)
            .map_err(|e| SolaceError::SerializationError(e.to_string()).into())
    }
}

/// Network status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub health: String,
    pub slot: u64,
    pub epoch: u64,
    pub block_height: u64,
    pub absolute_slot: u64,
}

/// Read keypair from file
fn read_keypair_file(path: &str) -> Result<Keypair> {
    let keypair_data = std::fs::read_to_string(path)
        .map_err(|e| SolaceError::IoError(e.to_string()))?;
    
    let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_data)
        .map_err(|e| SolaceError::DeserializationError(e.to_string()))?;
    
    Keypair::from_bytes(&keypair_bytes)
        .map_err(|e| SolaceError::InvalidKeypair(e.to_string()).into())
}

/// Blockchain event listener for monitoring on-chain activity
pub struct BlockchainEventListener {
    client: SolanaClient,
    event_handlers: HashMap<String, Box<dyn Fn(BlockchainEvent) + Send + Sync>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockchainEvent {
    AgentRegistered { agent_id: AgentId, pubkey: Pubkey },
    TransactionCreated { transaction_id: TransactionId, amount: u64 },
    ReputationUpdated { agent_id: AgentId, new_reputation: f64 },
    TransactionFinalized { transaction_id: TransactionId, success: bool },
    StakeChanged { agent_id: AgentId, new_stake: u64 },
    VoteCast { agent_id: AgentId, proposal_id: String, vote: bool },
}

impl BlockchainEventListener {
    pub fn new(client: SolanaClient) -> Self {
        Self {
            client,
            event_handlers: HashMap::new(),
        }
    }

    pub fn register_handler<F>(&mut self, event_type: String, handler: F)
    where
        F: Fn(BlockchainEvent) + Send + Sync + 'static,
    {
        self.event_handlers.insert(event_type, Box::new(handler));
    }

    pub async fn start_listening(&self) -> Result<()> {
        // In a real implementation, this would use WebSocket subscriptions
        // to listen for account changes and log messages
        info!("Started blockchain event listener");
        
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            // Poll for events and call handlers
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_level_conversion() {
        let processed: CommitmentConfig = CommitmentLevel::Processed.into();
        let confirmed: CommitmentConfig = CommitmentLevel::Confirmed.into();
        let finalized: CommitmentConfig = CommitmentLevel::Finalized.into();

        assert_eq!(processed, CommitmentConfig::processed());
        assert_eq!(confirmed, CommitmentConfig::confirmed());
        assert_eq!(finalized, CommitmentConfig::finalized());
    }

    #[test]
    fn test_blockchain_config_default() {
        let config = BlockchainConfig::default();
        assert!(!config.rpc_url.is_empty());
        assert!(!config.program_id.is_empty());
        assert!(config.confirmation_timeout.as_secs() > 0);
    }

    #[test]
    fn test_instruction_serialization() {
        let instruction = SolaceInstruction::InitializeAgent {
            agent_id: AgentId::new(),
            initial_reputation: 800,
        };

        let serialized = serde_json::to_vec(&instruction).unwrap();
        assert!(!serialized.is_empty());
    }
} 