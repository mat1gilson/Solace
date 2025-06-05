/**
 * Solace Protocol TypeScript SDK
 * 
 * A comprehensive SDK for interacting with the Solace Protocol autonomous agent
 * commerce framework on Solana blockchain.
 */

export * from './agent';
export * from './transaction';
export * from './reputation';
export * from './types';
export * from './client';
export * from './utils';

// Version information
export const SDK_VERSION = '1.0.0';
export const PROTOCOL_VERSION = '1.0.0';

/**
 * Initialize the Solace Protocol SDK
 */
export function initializeSDK(config?: SDKConfig): void {
  console.log(`Solace Protocol SDK v${SDK_VERSION} initialized`);
  
  if (config?.debug) {
    console.log('Debug mode enabled');
  }
}

/**
 * SDK Configuration options
 */
export interface SDKConfig {
  debug?: boolean;
  rpcEndpoint?: string;
  network?: 'devnet' | 'testnet' | 'mainnet';
}

// Default configuration
export const DEFAULT_CONFIG: Required<SDKConfig> = {
  debug: false,
  rpcEndpoint: 'https://api.devnet.solana.com',
  network: 'devnet',
}; 