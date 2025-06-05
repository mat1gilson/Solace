#!/usr/bin/env python3
"""
Basic Agent Example for Solace Protocol

This example demonstrates how to create and operate a simple autonomous agent
using the Solace Protocol Python SDK.
"""

import asyncio
import logging
from typing import List

# Import Solace Protocol SDK
from solace_protocol import (
    SolaceAgent,
    AgentConfig,
    AgentCapability,
    AgentPreferences,
    ServiceType,
    TransactionRequest
)

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class DataAnalysisAgent:
    """
    Example autonomous agent specializing in data analysis services
    """
    
    def __init__(self, name: str = "DataBot-001"):
        """Initialize the data analysis agent"""
        self.name = name
        self.agent = None
        
        # Configure agent preferences
        self.preferences = AgentPreferences(
            risk_tolerance=0.6,
            max_transaction_value=50.0,  # SOL
            min_counterparty_reputation=0.4,
            auto_accept_threshold=0.8,
            preferred_services=[ServiceType.DATA_ANALYSIS]
        )
        
        # Agent capabilities
        self.capabilities = [
            AgentCapability.DATA_ANALYSIS,
            AgentCapability.MARKET_RESEARCH
        ]
    
    async def initialize(self):
        """Initialize and register the agent"""
        logger.info(f"Initializing agent: {self.name}")
        
        config = AgentConfig(
            name=self.name,
            description="Specialized in financial data analysis and market research",
            capabilities=self.capabilities,
            preferences=self.preferences,
            network="devnet"  # Use devnet for testing
        )
        
        # Create and register agent
        self.agent = SolaceAgent(config)
        await self.agent.register()
        
        logger.info(f"Agent {self.name} registered with ID: {self.agent.get_id()}")
    
    async def start_operations(self):
        """Start autonomous agent operations"""
        logger.info("Starting autonomous operations...")
        
        # Start listening for transaction requests
        await self.agent.start_commerce()
        
        # Set up event handlers
        self.agent.on_transaction_request(self.handle_request)
        self.agent.on_negotiation_message(self.handle_negotiation)
        
        logger.info("Agent is now operational and listening for requests")
    
    async def handle_request(self, request: TransactionRequest):
        """Handle incoming transaction requests"""
        logger.info(f"Received request: {request.service_type} - Budget: {request.budget} SOL")
        
        # Check if we can handle this service type
        if request.service_type not in [ServiceType.DATA_ANALYSIS, ServiceType.MARKET_RESEARCH]:
            logger.info("Request not compatible with agent capabilities")
            return
        
        # Check if budget meets our requirements
        if request.budget < 5.0:  # Minimum 5 SOL
            logger.info("Budget too low for this service")
            return
        
        # Create proposal
        proposal_price = max(5.0, request.budget * 0.8)  # 80% of budget or 5 SOL minimum
        
        proposal = {
            "price": proposal_price,
            "estimated_completion": 24,  # hours
            "quality_guarantee": 0.95,
            "methodology": "Advanced statistical analysis with ML validation",
            "deliverables": [
                "Comprehensive data analysis report",
                "Visualizations and charts",
                "Actionable insights and recommendations",
                "Raw processed data files"
            ]
        }
        
        logger.info(f"Submitting proposal for {proposal_price} SOL")
        await self.agent.submit_proposal(request.id, proposal)
    
    async def handle_negotiation(self, message):
        """Handle negotiation messages"""
        logger.info(f"Negotiation message received: {message.type}")
        
        if message.type == "counter_offer":
            # Simple negotiation strategy: accept if within 20% of our proposal
            our_price = message.original_proposal.price
            their_offer = message.counter_price
            
            if their_offer >= our_price * 0.8:
                logger.info(f"Accepting counter-offer of {their_offer} SOL")
                await self.agent.accept_counter_offer(message.transaction_id, their_offer)
            else:
                logger.info("Counter-offer too low, declining")
                await self.agent.decline_counter_offer(message.transaction_id)
    
    async def perform_data_analysis(self, requirements: dict) -> dict:
        """
        Perform the actual data analysis service
        """
        logger.info("Performing data analysis...")
        
        # Simulate data analysis process
        await asyncio.sleep(2)  # Simulate processing time
        
        # Return mock analysis results
        return {
            "status": "completed",
            "analysis_type": requirements.get("type", "general"),
            "data_points_analyzed": 10000,
            "confidence_score": 0.94,
            "key_insights": [
                "Strong positive correlation found in dataset",
                "Seasonal patterns detected in Q3-Q4",
                "Anomaly detection identified 3 outliers"
            ],
            "recommendations": [
                "Increase investment allocation by 15%",
                "Monitor Q4 performance closely",
                "Investigate anomalies for risk assessment"
            ],
            "report_url": f"https://reports.solaceprotocol.com/analysis_{self.agent.get_id()}.pdf"
        }


async def main():
    """Main execution function"""
    logger.info("Starting Solace Protocol Basic Agent Example")
    
    # Create and initialize agent
    agent = DataAnalysisAgent("DataBot-001")
    await agent.initialize()
    
    # Start operations
    await agent.start_operations()
    
    # Keep agent running
    try:
        logger.info("Agent is running. Press Ctrl+C to stop...")
        while True:
            await asyncio.sleep(10)
            # Could add periodic tasks here (health checks, reputation updates, etc.)
            
    except KeyboardInterrupt:
        logger.info("Stopping agent...")
        await agent.agent.stop()
        logger.info("Agent stopped successfully")


if __name__ == "__main__":
    asyncio.run(main())