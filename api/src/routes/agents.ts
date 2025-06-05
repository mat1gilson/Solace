/**
 * Agent Management API Routes
 * 
 * Handles agent registration, updates, and queries
 */

import { Router, Request, Response } from 'express';
import { body, query, param, validationResult } from 'express-validator';
import { logger } from '../utils/logger';
import { AgentService } from '../services/AgentService';
import { ApiResponse } from '../types/api';

const router = Router();
const agentService = new AgentService();

/**
 * @swagger
 * components:
 *   schemas:
 *     Agent:
 *       type: object
 *       required:
 *         - id
 *         - publicKey
 *         - capabilities
 *       properties:
 *         id:
 *           type: string
 *           description: Unique agent identifier
 *         publicKey:
 *           type: string
 *           description: Agent's public key
 *         capabilities:
 *           type: array
 *           items:
 *             type: string
 *           description: List of agent capabilities
 *         reputation:
 *           type: number
 *           minimum: 0
 *           maximum: 1
 *           description: Agent's reputation score
 *         status:
 *           type: string
 *           enum: [active, inactive, banned]
 *           description: Agent's current status
 */

/**
 * @swagger
 * /api/v1/agents:
 *   get:
 *     summary: Get list of agents
 *     tags: [Agents]
 *     parameters:
 *       - in: query
 *         name: page
 *         schema:
 *           type: integer
 *           minimum: 1
 *         description: Page number
 *       - in: query
 *         name: limit
 *         schema:
 *           type: integer
 *           minimum: 1
 *           maximum: 100
 *         description: Number of items per page
 *       - in: query
 *         name: capability
 *         schema:
 *           type: string
 *         description: Filter by capability
 *       - in: query
 *         name: status
 *         schema:
 *           type: string
 *           enum: [active, inactive, banned]
 *         description: Filter by status
 *     responses:
 *       200:
 *         description: List of agents
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 data:
 *                   type: array
 *                   items:
 *                     $ref: '#/components/schemas/Agent'
 *                 pagination:
 *                   type: object
 *                   properties:
 *                     page:
 *                       type: integer
 *                     limit:
 *                       type: integer
 *                     total:
 *                       type: integer
 *                     pages:
 *                       type: integer
 */
router.get('/', 
  [
    query('page').optional().isInt({ min: 1 }),
    query('limit').optional().isInt({ min: 1, max: 100 }),
    query('capability').optional().isString(),
    query('status').optional().isIn(['active', 'inactive', 'banned']),
  ],
  async (req: Request, res: Response) => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        return res.status(400).json({
          success: false,
          error: 'Validation failed',
          details: errors.array(),
        } as ApiResponse);
      }

      const page = parseInt(req.query.page as string) || 1;
      const limit = parseInt(req.query.limit as string) || 20;
      const capability = req.query.capability as string;
      const status = req.query.status as string;

      const result = await agentService.getAgents({
        page,
        limit,
        filters: { capability, status },
      });

      res.json({
        success: true,
        data: result.agents,
        pagination: result.pagination,
      } as ApiResponse);

    } catch (error) {
      logger.error('Error fetching agents:', error);
      res.status(500).json({
        success: false,
        error: 'Internal server error',
      } as ApiResponse);
    }
  }
);

/**
 * @swagger
 * /api/v1/agents/{id}:
 *   get:
 *     summary: Get agent by ID
 *     tags: [Agents]
 *     parameters:
 *       - in: path
 *         name: id
 *         required: true
 *         schema:
 *           type: string
 *         description: Agent ID
 *     responses:
 *       200:
 *         description: Agent details
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 data:
 *                   $ref: '#/components/schemas/Agent'
 *       404:
 *         description: Agent not found
 */
router.get('/:id',
  [
    param('id').isString().isLength({ min: 1 }),
  ],
  async (req: Request, res: Response) => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        return res.status(400).json({
          success: false,
          error: 'Validation failed',
          details: errors.array(),
        } as ApiResponse);
      }

      const agent = await agentService.getAgentById(req.params.id);
      
      if (!agent) {
        return res.status(404).json({
          success: false,
          error: 'Agent not found',
        } as ApiResponse);
      }

      res.json({
        success: true,
        data: agent,
      } as ApiResponse);

    } catch (error) {
      logger.error('Error fetching agent:', error);
      res.status(500).json({
        success: false,
        error: 'Internal server error',
      } as ApiResponse);
    }
  }
);

/**
 * @swagger
 * /api/v1/agents:
 *   post:
 *     summary: Register a new agent
 *     tags: [Agents]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required:
 *               - publicKey
 *               - capabilities
 *               - preferences
 *             properties:
 *               publicKey:
 *                 type: string
 *                 description: Agent's public key
 *               capabilities:
 *                 type: array
 *                 items:
 *                   type: string
 *                 description: List of agent capabilities
 *               preferences:
 *                 type: object
 *                 properties:
 *                   riskTolerance:
 *                     type: number
 *                     minimum: 0
 *                     maximum: 1
 *                   maxTransactionValue:
 *                     type: number
 *                     minimum: 0
 *                   minCounterpartyReputation:
 *                     type: number
 *                     minimum: 0
 *                     maximum: 1
 *     responses:
 *       201:
 *         description: Agent registered successfully
 *       400:
 *         description: Invalid input
 *       409:
 *         description: Agent already exists
 */
router.post('/',
  [
    body('publicKey').isString().isLength({ min: 32 }),
    body('capabilities').isArray().notEmpty(),
    body('capabilities.*').isString(),
    body('preferences.riskTolerance').isFloat({ min: 0, max: 1 }),
    body('preferences.maxTransactionValue').isFloat({ min: 0 }),
    body('preferences.minCounterpartyReputation').optional().isFloat({ min: 0, max: 1 }),
  ],
  async (req: Request, res: Response) => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        return res.status(400).json({
          success: false,
          error: 'Validation failed',
          details: errors.array(),
        } as ApiResponse);
      }

      const { publicKey, capabilities, preferences } = req.body;

      const agent = await agentService.registerAgent({
        publicKey,
        capabilities,
        preferences,
      });

      res.status(201).json({
        success: true,
        data: agent,
        message: 'Agent registered successfully',
      } as ApiResponse);

    } catch (error) {
      if (error instanceof Error && error.message.includes('already exists')) {
        return res.status(409).json({
          success: false,
          error: 'Agent already exists',
        } as ApiResponse);
      }

      logger.error('Error registering agent:', error);
      res.status(500).json({
        success: false,
        error: 'Internal server error',
      } as ApiResponse);
    }
  }
);

/**
 * @swagger
 * /api/v1/agents/{id}:
 *   put:
 *     summary: Update agent configuration
 *     tags: [Agents]
 *     parameters:
 *       - in: path
 *         name: id
 *         required: true
 *         schema:
 *           type: string
 *         description: Agent ID
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             properties:
 *               capabilities:
 *                 type: array
 *                 items:
 *                   type: string
 *               preferences:
 *                 type: object
 *               status:
 *                 type: string
 *                 enum: [active, inactive]
 *     responses:
 *       200:
 *         description: Agent updated successfully
 *       404:
 *         description: Agent not found
 */
router.put('/:id',
  [
    param('id').isString().isLength({ min: 1 }),
    body('capabilities').optional().isArray(),
    body('capabilities.*').optional().isString(),
    body('preferences').optional().isObject(),
    body('status').optional().isIn(['active', 'inactive']),
  ],
  async (req: Request, res: Response) => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        return res.status(400).json({
          success: false,
          error: 'Validation failed',
          details: errors.array(),
        } as ApiResponse);
      }

      const agent = await agentService.updateAgent(req.params.id, req.body);
      
      if (!agent) {
        return res.status(404).json({
          success: false,
          error: 'Agent not found',
        } as ApiResponse);
      }

      res.json({
        success: true,
        data: agent,
        message: 'Agent updated successfully',
      } as ApiResponse);

    } catch (error) {
      logger.error('Error updating agent:', error);
      res.status(500).json({
        success: false,
        error: 'Internal server error',
      } as ApiResponse);
    }
  }
);

/**
 * @swagger
 * /api/v1/agents/{id}/transactions:
 *   get:
 *     summary: Get agent's transaction history
 *     tags: [Agents]
 *     parameters:
 *       - in: path
 *         name: id
 *         required: true
 *         schema:
 *           type: string
 *         description: Agent ID
 *       - in: query
 *         name: page
 *         schema:
 *           type: integer
 *           minimum: 1
 *         description: Page number
 *       - in: query
 *         name: limit
 *         schema:
 *           type: integer
 *           minimum: 1
 *           maximum: 100
 *         description: Number of items per page
 *     responses:
 *       200:
 *         description: Agent's transaction history
 *       404:
 *         description: Agent not found
 */
router.get('/:id/transactions',
  [
    param('id').isString().isLength({ min: 1 }),
    query('page').optional().isInt({ min: 1 }),
    query('limit').optional().isInt({ min: 1, max: 100 }),
  ],
  async (req: Request, res: Response) => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        return res.status(400).json({
          success: false,
          error: 'Validation failed',
          details: errors.array(),
        } as ApiResponse);
      }

      const page = parseInt(req.query.page as string) || 1;
      const limit = parseInt(req.query.limit as string) || 20;

      const result = await agentService.getAgentTransactions(req.params.id, {
        page,
        limit,
      });

      res.json({
        success: true,
        data: result.transactions,
        pagination: result.pagination,
      } as ApiResponse);

    } catch (error) {
      logger.error('Error fetching agent transactions:', error);
      res.status(500).json({
        success: false,
        error: 'Internal server error',
      } as ApiResponse);
    }
  }
);

/**
 * @swagger
 * /api/v1/agents/{id}/reputation:
 *   get:
 *     summary: Get agent's reputation details
 *     tags: [Agents]
 *     parameters:
 *       - in: path
 *         name: id
 *         required: true
 *         schema:
 *           type: string
 *         description: Agent ID
 *     responses:
 *       200:
 *         description: Agent's reputation details
 *       404:
 *         description: Agent not found
 */
router.get('/:id/reputation',
  [
    param('id').isString().isLength({ min: 1 }),
  ],
  async (req: Request, res: Response) => {
    try {
      const errors = validationResult(req);
      if (!errors.isEmpty()) {
        return res.status(400).json({
          success: false,
          error: 'Validation failed',
          details: errors.array(),
        } as ApiResponse);
      }

      const reputation = await agentService.getAgentReputation(req.params.id);
      
      if (!reputation) {
        return res.status(404).json({
          success: false,
          error: 'Agent not found',
        } as ApiResponse);
      }

      res.json({
        success: true,
        data: reputation,
      } as ApiResponse);

    } catch (error) {
      logger.error('Error fetching agent reputation:', error);
      res.status(500).json({
        success: false,
        error: 'Internal server error',
      } as ApiResponse);
    }
  }
);

export default router; 