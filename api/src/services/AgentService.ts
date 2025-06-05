/**
 * Agent Service
 * 
 * Business logic for agent management operations
 */

import { v4 as uuidv4 } from 'uuid';
import { logger } from '../utils/logger';
import { Agent, AgentStatus, AgentCapability } from '../types/agent';
import { Transaction } from '../types/transaction';
import { ReputationScore } from '../types/reputation';
import { PaginatedResult, PaginationOptions } from '../types/api';
import { AgentRepository } from '../repositories/AgentRepository';
import { TransactionRepository } from '../repositories/TransactionRepository';
import { ReputationRepository } from '../repositories/ReputationRepository';
import { BlockchainService } from './BlockchainService';
import { ValidationError } from '../utils/errors';

export interface AgentFilters {
  capability?: string;
  status?: AgentStatus;
  minReputation?: number;
  maxReputation?: number;
}

export interface AgentRegistrationRequest {
  publicKey: string;
  capabilities: AgentCapability[];
  preferences: {
    riskTolerance: number;
    maxTransactionValue: number;
    minCounterpartyReputation?: number;
    preferredPaymentMethods?: string[];
    autoAcceptThreshold?: number;
  };
  name?: string;
  description?: string;
}

export interface AgentUpdateRequest {
  capabilities?: AgentCapability[];
  preferences?: Partial<AgentRegistrationRequest['preferences']>;
  status?: AgentStatus;
  name?: string;
  description?: string;
}

export class AgentService {
  private agentRepository: AgentRepository;
  private transactionRepository: TransactionRepository;
  private reputationRepository: ReputationRepository;
  private blockchainService: BlockchainService;

  constructor() {
    this.agentRepository = new AgentRepository();
    this.transactionRepository = new TransactionRepository();
    this.reputationRepository = new ReputationRepository();
    this.blockchainService = new BlockchainService();
  }

  /**
   * Get paginated list of agents
   */
  async getAgents(options: PaginationOptions & { filters?: AgentFilters }): Promise<PaginatedResult<Agent>> {
    try {
      const { page = 1, limit = 20, filters = {} } = options;
      
      const result = await this.agentRepository.findMany({
        page,
        limit,
        filters: {
          capability: filters.capability,
          status: filters.status,
          reputationRange: filters.minReputation !== undefined || filters.maxReputation !== undefined 
            ? { min: filters.minReputation, max: filters.maxReputation }
            : undefined,
        },
      });

      return {
        data: result.agents,
        pagination: {
          page,
          limit,
          total: result.total,
          pages: Math.ceil(result.total / limit),
        },
      };
    } catch (error) {
      logger.error('Error in getAgents:', error);
      throw error;
    }
  }

  /**
   * Get agent by ID
   */
  async getAgentById(agentId: string): Promise<Agent | null> {
    try {
      return await this.agentRepository.findById(agentId);
    } catch (error) {
      logger.error('Error in getAgentById:', error);
      throw error;
    }
  }

  /**
   * Register a new agent
   */
  async registerAgent(request: AgentRegistrationRequest): Promise<Agent> {
    try {
      // Validate input
      this.validateAgentRegistration(request);

      // Check if agent with this public key already exists
      const existingAgent = await this.agentRepository.findByPublicKey(request.publicKey);
      if (existingAgent) {
        throw new ValidationError('Agent with this public key already exists');
      }

      // Generate agent ID
      const agentId = uuidv4();

      // Create agent object
      const agent: Agent = {
        id: agentId,
        publicKey: request.publicKey,
        name: request.name || `Agent-${agentId.slice(0, 8)}`,
        description: request.description || 'Autonomous trading agent',
        capabilities: request.capabilities,
        preferences: {
          riskTolerance: request.preferences.riskTolerance,
          maxTransactionValue: request.preferences.maxTransactionValue,
          minCounterpartyReputation: request.preferences.minCounterpartyReputation || 0.3,
          preferredPaymentMethods: request.preferences.preferredPaymentMethods || ['SOL'],
          autoAcceptThreshold: request.preferences.autoAcceptThreshold || 0.8,
        },
        status: AgentStatus.ACTIVE,
        reputation: 0.5, // Initial reputation
        totalTransactions: 0,
        successfulTransactions: 0,
        stakingAmount: 0,
        createdAt: new Date(),
        updatedAt: new Date(),
        lastActiveAt: new Date(),
      };

      // Save to database
      const savedAgent = await this.agentRepository.create(agent);

      // Initialize reputation record
      await this.reputationRepository.initialize(agentId, 0.5);

      // Register on blockchain (async)
      this.blockchainService.registerAgent(savedAgent).catch((error) => {
        logger.error('Failed to register agent on blockchain:', error);
      });

      logger.info(`Agent registered successfully: ${agentId}`);
      return savedAgent;

    } catch (error) {
      logger.error('Error in registerAgent:', error);
      throw error;
    }
  }

  /**
   * Update agent configuration
   */
  async updateAgent(agentId: string, request: AgentUpdateRequest): Promise<Agent | null> {
    try {
      const existingAgent = await this.agentRepository.findById(agentId);
      if (!existingAgent) {
        return null;
      }

      // Prepare update data
      const updateData: Partial<Agent> = {
        updatedAt: new Date(),
      };

      if (request.capabilities) {
        updateData.capabilities = request.capabilities;
      }

      if (request.preferences) {
        updateData.preferences = {
          ...existingAgent.preferences,
          ...request.preferences,
        };
      }

      if (request.status) {
        updateData.status = request.status;
      }

      if (request.name) {
        updateData.name = request.name;
      }

      if (request.description) {
        updateData.description = request.description;
      }

      // Update in database
      const updatedAgent = await this.agentRepository.update(agentId, updateData);

      if (updatedAgent) {
        logger.info(`Agent updated successfully: ${agentId}`);
      }

      return updatedAgent;

    } catch (error) {
      logger.error('Error in updateAgent:', error);
      throw error;
    }
  }

  /**
   * Get agent's transaction history
   */
  async getAgentTransactions(agentId: string, options: PaginationOptions): Promise<PaginatedResult<Transaction>> {
    try {
      const { page = 1, limit = 20 } = options;

      // Verify agent exists
      const agent = await this.agentRepository.findById(agentId);
      if (!agent) {
        throw new ValidationError('Agent not found');
      }

      const result = await this.transactionRepository.findByAgentId(agentId, {
        page,
        limit,
      });

      return {
        data: result.transactions,
        pagination: {
          page,
          limit,
          total: result.total,
          pages: Math.ceil(result.total / limit),
        },
      };

    } catch (error) {
      logger.error('Error in getAgentTransactions:', error);
      throw error;
    }
  }

  /**
   * Get agent's reputation details
   */
  async getAgentReputation(agentId: string): Promise<ReputationScore | null> {
    try {
      // Verify agent exists
      const agent = await this.agentRepository.findById(agentId);
      if (!agent) {
        return null;
      }

      return await this.reputationRepository.getReputation(agentId);

    } catch (error) {
      logger.error('Error in getAgentReputation:', error);
      throw error;
    }
  }

  /**
   * Update agent's last active timestamp
   */
  async updateLastActive(agentId: string): Promise<void> {
    try {
      await this.agentRepository.update(agentId, {
        lastActiveAt: new Date(),
      });
    } catch (error) {
      logger.error('Error in updateLastActive:', error);
      throw error;
    }
  }

  /**
   * Get agents by capability
   */
  async getAgentsByCapability(capability: AgentCapability, options: PaginationOptions): Promise<PaginatedResult<Agent>> {
    try {
      return await this.getAgents({
        ...options,
        filters: { capability },
      });
    } catch (error) {
      logger.error('Error in getAgentsByCapability:', error);
      throw error;
    }
  }

  /**
   * Ban an agent
   */
  async banAgent(agentId: string, reason: string): Promise<Agent | null> {
    try {
      const agent = await this.agentRepository.findById(agentId);
      if (!agent) {
        return null;
      }

      const updatedAgent = await this.agentRepository.update(agentId, {
        status: AgentStatus.BANNED,
        updatedAt: new Date(),
      });

      if (updatedAgent) {
        logger.warn(`Agent banned: ${agentId}, reason: ${reason}`);
        
        // Record ban in reputation system
        await this.reputationRepository.recordEvent(agentId, {
          type: 'ban',
          impact: -0.5,
          reason,
          timestamp: new Date(),
        });
      }

      return updatedAgent;

    } catch (error) {
      logger.error('Error in banAgent:', error);
      throw error;
    }
  }

  /**
   * Get agent statistics
   */
  async getAgentStatistics(agentId: string): Promise<any> {
    try {
      const agent = await this.agentRepository.findById(agentId);
      if (!agent) {
        throw new ValidationError('Agent not found');
      }

      const [reputation, transactionStats] = await Promise.all([
        this.reputationRepository.getReputation(agentId),
        this.transactionRepository.getAgentStats(agentId),
      ]);

      return {
        agent: {
          id: agent.id,
          name: agent.name,
          reputation: agent.reputation,
          status: agent.status,
          totalTransactions: agent.totalTransactions,
          successfulTransactions: agent.successfulTransactions,
          stakingAmount: agent.stakingAmount,
          createdAt: agent.createdAt,
          lastActiveAt: agent.lastActiveAt,
        },
        reputation,
        transactions: transactionStats,
      };

    } catch (error) {
      logger.error('Error in getAgentStatistics:', error);
      throw error;
    }
  }

  /**
   * Validate agent registration request
   */
  private validateAgentRegistration(request: AgentRegistrationRequest): void {
    if (!request.publicKey || request.publicKey.length < 32) {
      throw new ValidationError('Invalid public key');
    }

    if (!request.capabilities || request.capabilities.length === 0) {
      throw new ValidationError('At least one capability is required');
    }

    if (request.preferences.riskTolerance < 0 || request.preferences.riskTolerance > 1) {
      throw new ValidationError('Risk tolerance must be between 0 and 1');
    }

    if (request.preferences.maxTransactionValue <= 0) {
      throw new ValidationError('Max transaction value must be positive');
    }

    if (request.preferences.minCounterpartyReputation !== undefined) {
      if (request.preferences.minCounterpartyReputation < 0 || request.preferences.minCounterpartyReputation > 1) {
        throw new ValidationError('Min counterparty reputation must be between 0 and 1');
      }
    }
  }
}

export default AgentService; 