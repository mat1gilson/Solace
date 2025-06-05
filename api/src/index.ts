/**
 * Solace Protocol API Gateway
 * 
 * Main entry point for the RESTful API and WebSocket server
 * providing access to Solace Protocol functionality.
 */

import express from 'express';
import cors from 'cors';
import helmet from 'helmet';
import compression from 'compression';
import morgan from 'morgan';
import rateLimit from 'express-rate-limit';
import { createServer } from 'http';
import { Server } from 'socket.io';
import dotenv from 'dotenv';

import { logger } from './utils/logger';
import { errorHandler } from './middleware/errorHandler';
import { authMiddleware } from './middleware/auth';
import { setupSwagger } from './config/swagger';
import { connectDatabase } from './config/database';
import { setupWebSocket } from './websocket/socketHandler';

// Route imports
import agentRoutes from './routes/agents';
import transactionRoutes from './routes/transactions';
import reputationRoutes from './routes/reputation';
import networkRoutes from './routes/network';
import analyticsRoutes from './routes/analytics';
import adminRoutes from './routes/admin';

// Load environment variables
dotenv.config();

const app = express();
const server = createServer(app);
const io = new Server(server, {
  cors: {
    origin: process.env.CORS_ORIGIN || '*',
    methods: ['GET', 'POST'],
  },
});

const PORT = process.env.PORT || 3000;
const API_VERSION = process.env.API_VERSION || 'v1';

// Security middleware
app.use(helmet());
app.use(cors({
  origin: process.env.CORS_ORIGIN || '*',
  credentials: true,
}));

// Rate limiting
const limiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 1000, // limit each IP to 1000 requests per windowMs
  message: 'Too many requests from this IP, please try again later.',
  standardHeaders: true,
  legacyHeaders: false,
});
app.use(limiter);

// Compression and parsing
app.use(compression());
app.use(express.json({ limit: '10mb' }));
app.use(express.urlencoded({ extended: true, limit: '10mb' }));

// Logging
app.use(morgan('combined', {
  stream: {
    write: (message: string) => logger.info(message.trim()),
  },
}));

// Health check endpoint
app.get('/health', (req, res) => {
  res.status(200).json({
    status: 'healthy',
    timestamp: new Date().toISOString(),
    version: process.env.npm_package_version || '0.1.0',
    uptime: process.uptime(),
  });
});

// API routes
const apiRouter = express.Router();

// Public routes (no authentication required)
apiRouter.use('/network', networkRoutes);

// Protected routes (authentication required)
apiRouter.use('/agents', authMiddleware, agentRoutes);
apiRouter.use('/transactions', authMiddleware, transactionRoutes);
apiRouter.use('/reputation', authMiddleware, reputationRoutes);
apiRouter.use('/analytics', authMiddleware, analyticsRoutes);
apiRouter.use('/admin', authMiddleware, adminRoutes);

app.use(`/api/${API_VERSION}`, apiRouter);

// Setup Swagger documentation
setupSwagger(app);

// WebSocket setup
setupWebSocket(io);

// Error handling middleware (must be last)
app.use(errorHandler);

// 404 handler
app.use('*', (req, res) => {
  res.status(404).json({
    error: 'Not Found',
    message: `Route ${req.originalUrl} not found`,
    timestamp: new Date().toISOString(),
  });
});

/**
 * Initialize the server
 */
async function initializeServer(): Promise<void> {
  try {
    // Connect to database
    await connectDatabase();
    logger.info('Database connected successfully');

    // Start server
    server.listen(PORT, () => {
      logger.info(`🚀 Solace Protocol API Server running on port ${PORT}`);
      logger.info(`📚 API Documentation available at http://localhost:${PORT}/api-docs`);
      logger.info(`🌐 Health check available at http://localhost:${PORT}/health`);
      logger.info(`🔌 WebSocket server ready for connections`);
    });

    // Graceful shutdown handlers
    process.on('SIGTERM', gracefulShutdown);
    process.on('SIGINT', gracefulShutdown);

  } catch (error) {
    logger.error('Failed to initialize server:', error);
    process.exit(1);
  }
}

/**
 * Graceful shutdown handler
 */
function gracefulShutdown(signal: string): void {
  logger.info(`Received ${signal}. Starting graceful shutdown...`);
  
  server.close((err) => {
    if (err) {
      logger.error('Error during server shutdown:', err);
      process.exit(1);
    }
    
    logger.info('Server closed successfully');
    process.exit(0);
  });

  // Force shutdown if graceful shutdown takes too long
  setTimeout(() => {
    logger.error('Graceful shutdown timed out, forcing exit');
    process.exit(1);
  }, 30000);
}

// Start the server
if (require.main === module) {
  initializeServer();
}

export { app, server, io }; 