/**
 * Type definitions for Solace Protocol SDK
 */

import { Keypair } from '@solana/web3.js';

/**
 * Agent capabilities enum
 */
export enum AgentCapability {
  DataAnalysis = 'data_analysis',
  ComputationalTask = 'computational_task',
  MarketResearch = 'market_research',
  ContentCreation = 'content_creation',
  TradingService = 'trading_service',
  MachineLearning = 'machine_learning',
}

/**
 * Agent state enum
 */
export enum AgentState {
  Offline = 'offline',
  Online = 'online',
  Busy = 'busy',
  Maintenance = 'maintenance',
}

/**
 * Agent preferences for autonomous decision-making
 */
export interface AgentPreferences {
  riskTolerance: number; // 0.0 - 1.0
  maxTransactionValue: number; // in SOL
  minCounterpartyReputation: number; // 0.0 - 1.0
  preferredPaymentMethods: string[];
  autoAcceptThreshold: number; // 0.0 - 1.0
  geographicPreferences?: string[];
}

/**
 * Agent configuration
 */
export interface AgentConfig {
  name: string;
  description: string;
  capabilities: AgentCapability[];
  preferences: AgentPreferences;
  wallet?: Keypair;
  rpcEndpoint?: string;
  initialReputation?: number;
}

/**
 * Transaction phases
 */
export enum TransactionPhase {
  Request = 'request',
  Negotiation = 'negotiation',
  Execution = 'execution',
  Evaluation = 'evaluation',
}

/**
 * Transaction status
 */
export enum TransactionStatus {
  Pending = 'pending',
  InProgress = 'in_progress',
  Completed = 'completed',
  Failed = 'failed',
  Cancelled = 'cancelled',
  Expired = 'expired',
}

/**
 * Service types
 */
export enum ServiceType {
  DataAnalysis = 'data_analysis',
  ComputationalTask = 'computational_task',
  MarketResearch = 'market_research',
  ContentCreation = 'content_creation',
  TradingService = 'trading_service',
}

/**
 * Transaction request interface
 */
export interface TransactionRequest {
  type: string;
  service: ServiceType;
  budget: number;
  deadline: number;
  requirements: Record<string, any>;
}

/**
 * Reputation weight enum
 */
export enum ReputationWeight {
  Low = 'low',
  Medium = 'medium',
  High = 'high',
  Critical = 'critical',
}

/**
 * Negotiation strategy enum
 */
export enum NegotiationStrategy {
  Conservative = 'conservative',
  Aggressive = 'aggressive',
  Balanced = 'balanced',
}