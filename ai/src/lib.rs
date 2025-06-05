//! AI Module for Solace Protocol
//! 
//! This module provides intelligent behavior for autonomous agents,
//! including decision-making, negotiation strategies, and learning capabilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// AI decision-making context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    pub agent_reputation: f64,
    pub counterparty_reputation: f64,
    pub transaction_value: f64,
    pub market_conditions: MarketConditions,
    pub historical_performance: Vec<TransactionOutcome>,
}

/// Market conditions that influence decision-making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditions {
    pub demand_level: f64,  // 0.0 to 1.0
    pub competition_level: f64,  // 0.0 to 1.0
    pub average_pricing: f64,
    pub risk_indicators: Vec<RiskIndicator>,
}

/// Risk indicators for market analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskIndicator {
    pub indicator_type: String,
    pub value: f64,
    pub confidence: f64,
}

/// Transaction outcome for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionOutcome {
    pub success: bool,
    pub profit_margin: f64,
    pub satisfaction_score: f64,
    pub completion_time: u64,  // seconds
}

/// AI-powered negotiation strategy
#[derive(Debug, Clone)]
pub struct NegotiationAI {
    learning_rate: f64,
    risk_tolerance: f64,
    historical_data: Vec<TransactionOutcome>,
}

impl NegotiationAI {
    /// Create a new negotiation AI with specified parameters
    pub fn new(learning_rate: f64, risk_tolerance: f64) -> Self {
        Self {
            learning_rate,
            risk_tolerance,
            historical_data: Vec::new(),
        }
    }

    /// Make a pricing decision based on context
    pub fn decide_pricing(&self, context: &DecisionContext, base_price: f64) -> f64 {
        let reputation_factor = self.calculate_reputation_factor(context);
        let market_factor = self.calculate_market_factor(&context.market_conditions);
        let risk_factor = self.calculate_risk_factor(context);

        let adjusted_price = base_price * reputation_factor * market_factor * risk_factor;
        
        // Ensure price is within reasonable bounds
        adjusted_price.max(base_price * 0.5).min(base_price * 2.0)
    }

    /// Decide whether to accept a counter-offer
    pub fn should_accept_counter_offer(&self, context: &DecisionContext, counter_offer: f64, original_ask: f64) -> bool {
        let acceptance_threshold = self.calculate_acceptance_threshold(context);
        let offer_ratio = counter_offer / original_ask;
        
        offer_ratio >= acceptance_threshold
    }

    /// Update the AI model with new transaction outcomes
    pub fn learn_from_outcome(&mut self, outcome: TransactionOutcome) {
        self.historical_data.push(outcome);
        
        // Keep only the last 1000 transactions for learning
        if self.historical_data.len() > 1000 {
            self.historical_data.drain(0..self.historical_data.len() - 1000);
        }
    }

    /// Calculate reputation-based pricing factor
    fn calculate_reputation_factor(&self, context: &DecisionContext) -> f64 {
        let reputation_diff = context.agent_reputation - context.counterparty_reputation;
        
        // Higher reputation allows for premium pricing
        1.0 + (reputation_diff * 0.2)
    }

    /// Calculate market condition factor
    fn calculate_market_factor(&self, market: &MarketConditions) -> f64 {
        let demand_factor = 0.8 + (market.demand_level * 0.4); // 0.8 to 1.2
        let competition_factor = 1.3 - (market.competition_level * 0.3); // 1.0 to 1.3
        
        demand_factor * competition_factor
    }

    /// Calculate risk-based adjustment factor
    fn calculate_risk_factor(&self, context: &DecisionContext) -> f64 {
        let base_risk = 1.0;
        let risk_adjustment = context.market_conditions.risk_indicators
            .iter()
            .map(|indicator| indicator.value * indicator.confidence)
            .sum::<f64>() / context.market_conditions.risk_indicators.len().max(1) as f64;
        
        base_risk + (risk_adjustment * self.risk_tolerance)
    }

    /// Calculate the minimum acceptable offer ratio
    fn calculate_acceptance_threshold(&self, context: &DecisionContext) -> f64 {
        let base_threshold = 0.8; // Accept offers >= 80% of asking price
        let reputation_adjustment = (context.counterparty_reputation - 0.5) * 0.2;
        let market_adjustment = (context.market_conditions.demand_level - 0.5) * 0.1;
        
        (base_threshold + reputation_adjustment + market_adjustment).clamp(0.6, 0.95)
    }

    /// Get success rate from historical data
    pub fn get_success_rate(&self) -> f64 {
        if self.historical_data.is_empty() {
            0.5 // Default assumption
        } else {
            let successful = self.historical_data.iter().filter(|outcome| outcome.success).count();
            successful as f64 / self.historical_data.len() as f64
        }
    }

    /// Get average profit margin
    pub fn get_average_profit_margin(&self) -> f64 {
        if self.historical_data.is_empty() {
            0.0
        } else {
            let total_margin: f64 = self.historical_data.iter().map(|outcome| outcome.profit_margin).sum();
            total_margin / self.historical_data.len() as f64
        }
    }
}

/// Predictive market analysis using simple statistical methods
pub struct MarketPredictor {
    price_history: Vec<f64>,
    demand_history: Vec<f64>,
}

impl MarketPredictor {
    pub fn new() -> Self {
        Self {
            price_history: Vec::new(),
            demand_history: Vec::new(),
        }
    }

    /// Add new market data point
    pub fn add_data_point(&mut self, price: f64, demand: f64) {
        self.price_history.push(price);
        self.demand_history.push(demand);

        // Keep only last 100 data points
        if self.price_history.len() > 100 {
            self.price_history.remove(0);
            self.demand_history.remove(0);
        }
    }

    /// Predict future price trend
    pub fn predict_price_trend(&self) -> PriceTrend {
        if self.price_history.len() < 3 {
            return PriceTrend::Stable;
        }

        let recent_prices: Vec<f64> = self.price_history
            .iter()
            .rev()
            .take(5)
            .cloned()
            .collect();

        let trend = self.calculate_linear_trend(&recent_prices);

        if trend > 0.05 {
            PriceTrend::Rising
        } else if trend < -0.05 {
            PriceTrend::Falling
        } else {
            PriceTrend::Stable
        }
    }

    /// Calculate simple linear trend
    fn calculate_linear_trend(&self, prices: &[f64]) -> f64 {
        if prices.len() < 2 {
            return 0.0;
        }

        let n = prices.len() as f64;
        let sum_x: f64 = (0..prices.len()).map(|i| i as f64).sum();
        let sum_y: f64 = prices.iter().sum();
        let sum_xy: f64 = prices.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_x2: f64 = (0..prices.len()).map(|i| (i as f64).powi(2)).sum();

        // Linear regression slope
        (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2))
    }
}

/// Price trend prediction
#[derive(Debug, Clone, PartialEq)]
pub enum PriceTrend {
    Rising,
    Falling,
    Stable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negotiation_ai_creation() {
        let ai = NegotiationAI::new(0.1, 0.6);
        assert_eq!(ai.learning_rate, 0.1);
        assert_eq!(ai.risk_tolerance, 0.6);
    }

    #[test]
    fn test_pricing_decision() {
        let ai = NegotiationAI::new(0.1, 0.6);
        let context = DecisionContext {
            agent_reputation: 0.8,
            counterparty_reputation: 0.6,
            transaction_value: 100.0,
            market_conditions: MarketConditions {
                demand_level: 0.7,
                competition_level: 0.4,
                average_pricing: 95.0,
                risk_indicators: vec![],
            },
            historical_performance: vec![],
        };

        let price = ai.decide_pricing(&context, 100.0);
        assert!(price > 50.0 && price < 200.0);
    }

    #[test]
    fn test_market_predictor() {
        let mut predictor = MarketPredictor::new();
        
        // Add some rising price data
        for i in 0..10 {
            predictor.add_data_point(100.0 + i as f64 * 2.0, 0.5);
        }

        let trend = predictor.predict_price_trend();
        assert_eq!(trend, PriceTrend::Rising);
    }
} 