"""
Advanced Trading Agent for Solace Protocol

This module implements an intelligent trading agent that uses machine learning
to make autonomous trading decisions, negotiate with counterparties, and 
optimize portfolio performance.
"""

import asyncio
import logging
import numpy as np
import pandas as pd
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple, Any
from dataclasses import dataclass
from enum import Enum

import torch
import torch.nn as nn
from sklearn.preprocessing import StandardScaler
from sklearn.ensemble import RandomForestRegressor
import ta

from ..models.lstm_predictor import LSTMPredictor
from ..models.reinforcement_learner import DQNAgent
from ..utils.market_data import MarketDataProvider
from ..utils.risk_manager import RiskManager
from ..utils.performance_tracker import PerformanceTracker
from ..blockchain.solana_client import SolanaClient
from ..negotiation.strategy_engine import NegotiationStrategyEngine

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class TradingMode(Enum):
    """Trading modes for the agent"""
    CONSERVATIVE = "conservative"
    BALANCED = "balanced"
    AGGRESSIVE = "aggressive"
    CUSTOM = "custom"

class MarketCondition(Enum):
    """Market condition classifications"""
    BULL = "bull"
    BEAR = "bear"
    SIDEWAYS = "sideways"
    VOLATILE = "volatile"

@dataclass
class TradingConfig:
    """Configuration for trading agent"""
    agent_id: str
    initial_capital: float
    max_position_size: float
    risk_tolerance: float
    trading_mode: TradingMode
    min_profit_threshold: float
    max_loss_threshold: float
    update_frequency: int  # seconds
    use_ml_predictions: bool
    use_technical_analysis: bool
    use_sentiment_analysis: bool
    
class TradingAgent:
    """
    Intelligent trading agent with ML-powered decision making
    """
    
    def __init__(self, config: TradingConfig):
        self.config = config
        self.is_running = False
        self.current_positions = {}
        self.pending_orders = {}
        
        # Initialize components
        self.market_data = MarketDataProvider()
        self.risk_manager = RiskManager(config)
        self.performance_tracker = PerformanceTracker(config.agent_id)
        self.blockchain_client = SolanaClient()
        self.negotiation_engine = NegotiationStrategyEngine()
        
        # ML Models
        self.lstm_predictor = LSTMPredictor()
        self.dqn_agent = DQNAgent(state_size=50, action_size=3)  # buy, sell, hold
        self.scaler = StandardScaler()
        
        # Technical indicators cache
        self.technical_indicators = {}
        
        # Performance metrics
        self.total_trades = 0
        self.successful_trades = 0
        self.total_pnl = 0.0
        self.max_drawdown = 0.0
        
        logger.info(f"Trading agent {config.agent_id} initialized with mode: {config.trading_mode}")
    
    async def start_trading(self):
        """Start the trading loop"""
        if self.is_running:
            logger.warning("Trading agent is already running")
            return
            
        self.is_running = True
        logger.info(f"Starting trading agent {self.config.agent_id}")
        
        try:
            # Load pre-trained models
            await self._load_models()
            
            # Start main trading loop
            await self._trading_loop()
            
        except Exception as e:
            logger.error(f"Error in trading loop: {e}")
            self.is_running = False
    
    async def stop_trading(self):
        """Stop the trading agent"""
        logger.info(f"Stopping trading agent {self.config.agent_id}")
        self.is_running = False
        
        # Close all positions
        await self._close_all_positions()
        
        # Save model states
        await self._save_models()
        
        # Generate final report
        await self._generate_performance_report()
    
    async def _trading_loop(self):
        """Main trading decision loop"""
        while self.is_running:
            try:
                # Collect market data
                market_data = await self.market_data.get_latest_data()
                
                # Analyze market conditions
                market_condition = await self._analyze_market_condition(market_data)
                
                # Update technical indicators
                await self._update_technical_indicators(market_data)
                
                # Generate ML predictions
                predictions = await self._generate_predictions(market_data)
                
                # Make trading decisions
                decisions = await self._make_trading_decisions(
                    market_data, market_condition, predictions
                )
                
                # Execute trades
                for decision in decisions:
                    await self._execute_trade(decision)
                
                # Update performance metrics
                await self._update_performance_metrics()
                
                # Risk management check
                await self._perform_risk_check()
                
                # Wait before next iteration
                await asyncio.sleep(self.config.update_frequency)
                
            except Exception as e:
                logger.error(f"Error in trading loop iteration: {e}")
                await asyncio.sleep(self.config.update_frequency)
    
    async def _analyze_market_condition(self, market_data: Dict) -> MarketCondition:
        """Analyze current market conditions using multiple indicators"""
        try:
            prices = market_data.get('prices', [])
            volumes = market_data.get('volumes', [])
            
            if len(prices) < 20:
                return MarketCondition.SIDEWAYS
            
            # Calculate trend indicators
            price_series = pd.Series(prices)
            sma_20 = price_series.rolling(window=20).mean().iloc[-1]
            sma_50 = price_series.rolling(window=min(50, len(prices))).mean().iloc[-1]
            
            # Volatility analysis
            returns = price_series.pct_change().dropna()
            volatility = returns.std() * np.sqrt(252)  # Annualized volatility
            
            # Price momentum
            price_change_20d = (prices[-1] - prices[-20]) / prices[-20] if len(prices) >= 20 else 0
            
            # Market condition classification
            if price_change_20d > 0.05 and prices[-1] > sma_20 > sma_50:
                return MarketCondition.BULL
            elif price_change_20d < -0.05 and prices[-1] < sma_20 < sma_50:
                return MarketCondition.BEAR
            elif volatility > 0.3:
                return MarketCondition.VOLATILE
            else:
                return MarketCondition.SIDEWAYS
                
        except Exception as e:
            logger.error(f"Error analyzing market condition: {e}")
            return MarketCondition.SIDEWAYS
    
    async def _update_technical_indicators(self, market_data: Dict):
        """Update technical analysis indicators"""
        try:
            prices = market_data.get('prices', [])
            volumes = market_data.get('volumes', [])
            
            if len(prices) < 20:
                return
            
            df = pd.DataFrame({
                'close': prices,
                'volume': volumes
            })
            
            # Moving averages
            df['sma_10'] = ta.trend.sma_indicator(df['close'], window=10)
            df['sma_20'] = ta.trend.sma_indicator(df['close'], window=20)
            df['ema_12'] = ta.trend.ema_indicator(df['close'], window=12)
            df['ema_26'] = ta.trend.ema_indicator(df['close'], window=26)
            
            # MACD
            df['macd'] = ta.trend.macd(df['close'])
            df['macd_signal'] = ta.trend.macd_signal(df['close'])
            df['macd_histogram'] = ta.trend.macd_diff(df['close'])
            
            # RSI
            df['rsi'] = ta.momentum.rsi(df['close'])
            
            # Bollinger Bands
            df['bb_upper'] = ta.volatility.bollinger_hband(df['close'])
            df['bb_lower'] = ta.volatility.bollinger_lband(df['close'])
            df['bb_middle'] = ta.volatility.bollinger_mavg(df['close'])
            
            # Volume indicators
            df['volume_sma'] = ta.volume.volume_sma(df['close'], df['volume'])
            df['vwap'] = ta.volume.volume_weighted_average_price(
                df['close'], df['close'], df['close'], df['volume']
            )
            
            # Store latest values
            self.technical_indicators = {
                'sma_10': df['sma_10'].iloc[-1],
                'sma_20': df['sma_20'].iloc[-1],
                'ema_12': df['ema_12'].iloc[-1],
                'ema_26': df['ema_26'].iloc[-1],
                'macd': df['macd'].iloc[-1],
                'macd_signal': df['macd_signal'].iloc[-1],
                'rsi': df['rsi'].iloc[-1],
                'bb_upper': df['bb_upper'].iloc[-1],
                'bb_lower': df['bb_lower'].iloc[-1],
                'bb_middle': df['bb_middle'].iloc[-1],
                'volume_sma': df['volume_sma'].iloc[-1],
                'vwap': df['vwap'].iloc[-1],
                'current_price': prices[-1]
            }
            
        except Exception as e:
            logger.error(f"Error updating technical indicators: {e}")
    
    async def _generate_predictions(self, market_data: Dict) -> Dict:
        """Generate ML-based price predictions"""
        predictions = {}
        
        if not self.config.use_ml_predictions:
            return predictions
        
        try:
            prices = market_data.get('prices', [])
            volumes = market_data.get('volumes', [])
            
            if len(prices) < 50:
                return predictions
            
            # Prepare features for ML models
            features = self._prepare_features(prices, volumes)
            
            # LSTM price prediction
            if self.lstm_predictor:
                lstm_prediction = await self.lstm_predictor.predict(features)
                predictions['lstm_price'] = lstm_prediction
            
            # Reinforcement learning action
            if self.dqn_agent:
                state = self._get_state_vector(features)
                action = self.dqn_agent.get_action(state)
                predictions['rl_action'] = action  # 0: sell, 1: hold, 2: buy
            
            # Ensemble prediction
            predictions['confidence'] = self._calculate_prediction_confidence(predictions)
            
        except Exception as e:
            logger.error(f"Error generating predictions: {e}")
        
        return predictions
    
    def _prepare_features(self, prices: List[float], volumes: List[float]) -> np.ndarray:
        """Prepare feature matrix for ML models"""
        try:
            # Create feature matrix
            features = []
            
            # Price-based features
            price_series = np.array(prices[-50:])  # Last 50 data points
            returns = np.diff(price_series) / price_series[:-1]
            
            # Statistical features
            features.extend([
                np.mean(returns),
                np.std(returns),
                np.min(returns),
                np.max(returns),
                np.percentile(returns, 25),
                np.percentile(returns, 75)
            ])
            
            # Technical indicator features
            if self.technical_indicators:
                features.extend([
                    self.technical_indicators.get('rsi', 50) / 100,
                    (self.technical_indicators.get('current_price', 0) - 
                     self.technical_indicators.get('sma_20', 0)) / 
                    self.technical_indicators.get('sma_20', 1),
                    self.technical_indicators.get('macd', 0),
                    (self.technical_indicators.get('current_price', 0) - 
                     self.technical_indicators.get('bb_middle', 0)) / 
                    (self.technical_indicators.get('bb_upper', 1) - 
                     self.technical_indicators.get('bb_lower', 1)),
                ])
            
            # Volume features
            if volumes and len(volumes) >= 10:
                volume_series = np.array(volumes[-10:])
                features.extend([
                    np.mean(volume_series),
                    np.std(volume_series),
                ])
            
            # Pad or truncate to fixed size
            target_size = 50
            if len(features) < target_size:
                features.extend([0] * (target_size - len(features)))
            else:
                features = features[:target_size]
            
            return np.array(features).reshape(1, -1)
            
        except Exception as e:
            logger.error(f"Error preparing features: {e}")
            return np.zeros((1, 50))
    
    def _get_state_vector(self, features: np.ndarray) -> np.ndarray:
        """Get state vector for reinforcement learning"""
        try:
            # Flatten features and add position information
            state = features.flatten()
            
            # Add current position information
            position_info = [
                len(self.current_positions),
                self.total_pnl,
                self.risk_manager.get_current_risk_level()
            ]
            
            # Normalize and combine
            state = np.concatenate([state, position_info])
            
            # Ensure fixed size
            if len(state) < 53:
                state = np.pad(state, (0, 53 - len(state)), mode='constant')
            else:
                state = state[:53]
            
            return state
            
        except Exception as e:
            logger.error(f"Error creating state vector: {e}")
            return np.zeros(53)
    
    def _calculate_prediction_confidence(self, predictions: Dict) -> float:
        """Calculate confidence in predictions"""
        try:
            confidence_factors = []
            
            # LSTM confidence (based on prediction variance)
            if 'lstm_price' in predictions:
                confidence_factors.append(0.7)  # Base confidence
            
            # RL action confidence (based on Q-values)
            if 'rl_action' in predictions:
                confidence_factors.append(0.6)
            
            # Technical analysis alignment
            if self.technical_indicators:
                rsi = self.technical_indicators.get('rsi', 50)
                if 20 < rsi < 80:  # Not oversold/overbought
                    confidence_factors.append(0.8)
                else:
                    confidence_factors.append(0.4)
            
            return np.mean(confidence_factors) if confidence_factors else 0.5
            
        except Exception as e:
            logger.error(f"Error calculating confidence: {e}")
            return 0.5
    
    async def _make_trading_decisions(
        self, 
        market_data: Dict, 
        market_condition: MarketCondition, 
        predictions: Dict
    ) -> List[Dict]:
        """Make trading decisions based on analysis"""
        decisions = []
        
        try:
            current_price = market_data.get('prices', [0])[-1]
            
            # Check if we should trade based on confidence
            confidence = predictions.get('confidence', 0.5)
            if confidence < 0.6:
                logger.info("Low confidence, skipping trade")
                return decisions
            
            # Determine trade direction
            trade_direction = self._determine_trade_direction(
                market_condition, predictions, current_price
            )
            
            if trade_direction == 'buy':
                size = self._calculate_position_size('buy', current_price)
                if size > 0:
                    decisions.append({
                        'action': 'buy',
                        'size': size,
                        'price': current_price,
                        'confidence': confidence,
                        'reasoning': self._get_decision_reasoning(
                            'buy', market_condition, predictions
                        )
                    })
            
            elif trade_direction == 'sell':
                size = self._calculate_position_size('sell', current_price)
                if size > 0:
                    decisions.append({
                        'action': 'sell',
                        'size': size,
                        'price': current_price,
                        'confidence': confidence,
                        'reasoning': self._get_decision_reasoning(
                            'sell', market_condition, predictions
                        )
                    })
            
            # Check for position management
            position_decisions = await self._check_position_management()
            decisions.extend(position_decisions)
            
        except Exception as e:
            logger.error(f"Error making trading decisions: {e}")
        
        return decisions
    
    def _determine_trade_direction(
        self, 
        market_condition: MarketCondition, 
        predictions: Dict, 
        current_price: float
    ) -> str:
        """Determine whether to buy, sell, or hold"""
        try:
            signals = []
            
            # ML predictions
            rl_action = predictions.get('rl_action', 1)  # Default to hold
            if rl_action == 2:
                signals.append('buy')
            elif rl_action == 0:
                signals.append('sell')
            
            # Technical analysis signals
            if self.technical_indicators:
                rsi = self.technical_indicators.get('rsi', 50)
                macd = self.technical_indicators.get('macd', 0)
                macd_signal = self.technical_indicators.get('macd_signal', 0)
                
                # RSI signals
                if rsi < 30:
                    signals.append('buy')
                elif rsi > 70:
                    signals.append('sell')
                
                # MACD signals
                if macd > macd_signal:
                    signals.append('buy')
                elif macd < macd_signal:
                    signals.append('sell')
                
                # Price vs moving averages
                sma_20 = self.technical_indicators.get('sma_20', current_price)
                if current_price > sma_20 * 1.01:
                    signals.append('buy')
                elif current_price < sma_20 * 0.99:
                    signals.append('sell')
            
            # Market condition influence
            if market_condition == MarketCondition.BULL:
                signals.append('buy')
            elif market_condition == MarketCondition.BEAR:
                signals.append('sell')
            
            # Majority vote
            buy_votes = signals.count('buy')
            sell_votes = signals.count('sell')
            
            if buy_votes > sell_votes:
                return 'buy'
            elif sell_votes > buy_votes:
                return 'sell'
            else:
                return 'hold'
                
        except Exception as e:
            logger.error(f"Error determining trade direction: {e}")
            return 'hold'
    
    def _calculate_position_size(self, action: str, price: float) -> float:
        """Calculate appropriate position size"""
        try:
            # Risk-based position sizing
            account_value = self.performance_tracker.get_account_value()
            risk_per_trade = account_value * (self.config.risk_tolerance / 100)
            
            # Kelly criterion adjustment
            win_rate = self.successful_trades / max(self.total_trades, 1)
            kelly_fraction = self._calculate_kelly_fraction(win_rate)
            
            # Position size calculation
            if action == 'buy':
                max_size = self.config.max_position_size
                risk_adjusted_size = risk_per_trade / price
                kelly_adjusted_size = account_value * kelly_fraction / price
                
                position_size = min(max_size, risk_adjusted_size, kelly_adjusted_size)
            else:  # sell
                # Can only sell what we own
                current_holdings = sum(pos['size'] for pos in self.current_positions.values())
                position_size = min(current_holdings, self.config.max_position_size)
            
            return max(0, position_size)
            
        except Exception as e:
            logger.error(f"Error calculating position size: {e}")
            return 0
    
    def _calculate_kelly_fraction(self, win_rate: float) -> float:
        """Calculate Kelly fraction for optimal position sizing"""
        try:
            if win_rate <= 0.5:
                return 0.01  # Very small position if not profitable
            
            # Simplified Kelly: f = (bp - q) / b
            # where b = odds received, p = win probability, q = loss probability
            avg_win = 0.02  # Assume 2% average win
            avg_loss = 0.015  # Assume 1.5% average loss
            
            if avg_loss == 0:
                return 0.01
            
            b = avg_win / avg_loss
            p = win_rate
            q = 1 - win_rate
            
            kelly_fraction = (b * p - q) / b
            
            # Cap at 25% of account for safety
            return max(0, min(kelly_fraction, 0.25))
            
        except Exception as e:
            logger.error(f"Error calculating Kelly fraction: {e}")
            return 0.01
    
    def _get_decision_reasoning(
        self, 
        action: str, 
        market_condition: MarketCondition, 
        predictions: Dict
    ) -> str:
        """Generate reasoning for trading decision"""
        try:
            reasons = []
            
            # Market condition
            reasons.append(f"Market condition: {market_condition.value}")
            
            # ML predictions
            if 'rl_action' in predictions:
                action_map = {0: 'sell', 1: 'hold', 2: 'buy'}
                rl_action = action_map.get(predictions['rl_action'], 'hold')
                reasons.append(f"RL model suggests: {rl_action}")
            
            # Technical indicators
            if self.technical_indicators:
                rsi = self.technical_indicators.get('rsi', 50)
                reasons.append(f"RSI: {rsi:.1f}")
                
                if rsi < 30:
                    reasons.append("RSI oversold")
                elif rsi > 70:
                    reasons.append("RSI overbought")
            
            # Confidence
            confidence = predictions.get('confidence', 0.5)
            reasons.append(f"Confidence: {confidence:.2f}")
            
            return "; ".join(reasons)
            
        except Exception as e:
            logger.error(f"Error generating reasoning: {e}")
            return f"Decision: {action}"
    
    async def _execute_trade(self, decision: Dict):
        """Execute a trading decision"""
        try:
            action = decision['action']
            size = decision['size']
            price = decision['price']
            
            logger.info(f"Executing {action} order: {size} @ {price}")
            logger.info(f"Reasoning: {decision.get('reasoning', 'N/A')}")
            
            # Create order
            order_id = f"{self.config.agent_id}_{datetime.now().timestamp()}"
            
            # Submit to blockchain (mock implementation)
            success = await self._submit_blockchain_transaction({
                'order_id': order_id,
                'action': action,
                'size': size,
                'price': price,
                'timestamp': datetime.now()
            })
            
            if success:
                # Update positions
                if action == 'buy':
                    self.current_positions[order_id] = {
                        'size': size,
                        'entry_price': price,
                        'timestamp': datetime.now()
                    }
                elif action == 'sell':
                    # Close oldest positions first (FIFO)
                    await self._close_positions(size, price)
                
                # Update metrics
                self.total_trades += 1
                await self.performance_tracker.record_trade(decision)
                
                # Train RL model with outcome
                if self.config.use_ml_predictions:
                    await self._update_rl_model(decision, success)
                
            else:
                logger.error(f"Failed to execute trade: {order_id}")
                
        except Exception as e:
            logger.error(f"Error executing trade: {e}")
    
    async def _submit_blockchain_transaction(self, order: Dict) -> bool:
        """Submit transaction to blockchain"""
        try:
            # In a real implementation, this would interact with Solana
            # For now, simulate with high success rate
            await asyncio.sleep(0.1)  # Simulate network delay
            return np.random.random() > 0.05  # 95% success rate
            
        except Exception as e:
            logger.error(f"Error submitting blockchain transaction: {e}")
            return False
    
    async def _close_positions(self, size: float, price: float):
        """Close positions FIFO style"""
        try:
            remaining_size = size
            positions_to_remove = []
            
            for order_id, position in self.current_positions.items():
                if remaining_size <= 0:
                    break
                
                position_size = position['size']
                entry_price = position['entry_price']
                
                if position_size <= remaining_size:
                    # Close entire position
                    pnl = (price - entry_price) * position_size
                    self.total_pnl += pnl
                    remaining_size -= position_size
                    positions_to_remove.append(order_id)
                    
                    if pnl > 0:
                        self.successful_trades += 1
                        
                else:
                    # Partial close
                    pnl = (price - entry_price) * remaining_size
                    self.total_pnl += pnl
                    position['size'] -= remaining_size
                    remaining_size = 0
                    
                    if pnl > 0:
                        self.successful_trades += 1
            
            # Remove fully closed positions
            for order_id in positions_to_remove:
                del self.current_positions[order_id]
                
        except Exception as e:
            logger.error(f"Error closing positions: {e}")
    
    async def _check_position_management(self) -> List[Dict]:
        """Check if any positions need management (stop loss, take profit)"""
        decisions = []
        
        try:
            current_price = self.technical_indicators.get('current_price', 0)
            
            for order_id, position in self.current_positions.items():
                entry_price = position['entry_price']
                size = position['size']
                
                # Calculate unrealized PnL
                unrealized_pnl = (current_price - entry_price) / entry_price
                
                # Stop loss check
                if unrealized_pnl < -self.config.max_loss_threshold:
                    decisions.append({
                        'action': 'sell',
                        'size': size,
                        'price': current_price,
                        'confidence': 1.0,
                        'reasoning': f"Stop loss triggered: {unrealized_pnl:.2%}",
                        'position_id': order_id
                    })
                
                # Take profit check
                elif unrealized_pnl > self.config.min_profit_threshold:
                    decisions.append({
                        'action': 'sell',
                        'size': size * 0.5,  # Partial profit taking
                        'price': current_price,
                        'confidence': 1.0,
                        'reasoning': f"Take profit triggered: {unrealized_pnl:.2%}",
                        'position_id': order_id
                    })
        
        except Exception as e:
            logger.error(f"Error in position management: {e}")
        
        return decisions
    
    async def _update_performance_metrics(self):
        """Update performance tracking metrics"""
        try:
            # Update unrealized PnL
            unrealized_pnl = 0
            current_price = self.technical_indicators.get('current_price', 0)
            
            for position in self.current_positions.values():
                entry_price = position['entry_price']
                size = position['size']
                unrealized_pnl += (current_price - entry_price) * size
            
            # Update performance tracker
            await self.performance_tracker.update_metrics({
                'total_pnl': self.total_pnl,
                'unrealized_pnl': unrealized_pnl,
                'total_trades': self.total_trades,
                'successful_trades': self.successful_trades,
                'win_rate': self.successful_trades / max(self.total_trades, 1),
                'active_positions': len(self.current_positions),
                'timestamp': datetime.now()
            })
            
        except Exception as e:
            logger.error(f"Error updating performance metrics: {e}")
    
    async def _perform_risk_check(self):
        """Perform risk management checks"""
        try:
            # Check overall risk exposure
            risk_level = self.risk_manager.get_current_risk_level()
            
            if risk_level > 0.8:  # High risk
                logger.warning(f"High risk level detected: {risk_level}")
                
                # Reduce positions if necessary
                if len(self.current_positions) > 0:
                    await self._reduce_positions(0.5)  # Reduce by 50%
            
            # Check drawdown
            current_drawdown = self.performance_tracker.get_current_drawdown()
            if current_drawdown > self.config.max_loss_threshold:
                logger.warning(f"Maximum drawdown exceeded: {current_drawdown}")
                await self._close_all_positions()
                
        except Exception as e:
            logger.error(f"Error in risk check: {e}")
    
    async def _reduce_positions(self, reduction_factor: float):
        """Reduce all positions by a factor"""
        try:
            current_price = self.technical_indicators.get('current_price', 0)
            
            for order_id, position in list(self.current_positions.items()):
                reduction_size = position['size'] * reduction_factor
                
                await self._execute_trade({
                    'action': 'sell',
                    'size': reduction_size,
                    'price': current_price,
                    'confidence': 1.0,
                    'reasoning': 'Risk management: position reduction'
                })
                
        except Exception as e:
            logger.error(f"Error reducing positions: {e}")
    
    async def _close_all_positions(self):
        """Close all open positions"""
        try:
            current_price = self.technical_indicators.get('current_price', 0)
            
            for order_id, position in list(self.current_positions.items()):
                await self._execute_trade({
                    'action': 'sell',
                    'size': position['size'],
                    'price': current_price,
                    'confidence': 1.0,
                    'reasoning': 'Risk management: close all positions'
                })
                
        except Exception as e:
            logger.error(f"Error closing all positions: {e}")
    
    async def _update_rl_model(self, decision: Dict, success: bool):
        """Update reinforcement learning model based on trade outcome"""
        try:
            if not self.dqn_agent:
                return
            
            # Create reward signal
            reward = 1.0 if success else -1.0
            
            # Additional reward based on PnL if available
            if 'pnl' in decision:
                reward += decision['pnl'] * 10  # Scale PnL to reward
            
            # Update the model (simplified)
            # In practice, you'd store state-action-reward-next_state tuples
            # and train the model periodically
            
            logger.debug(f"Updated RL model with reward: {reward}")
            
        except Exception as e:
            logger.error(f"Error updating RL model: {e}")
    
    async def _load_models(self):
        """Load pre-trained models"""
        try:
            # Load LSTM model
            if self.lstm_predictor:
                await self.lstm_predictor.load_model('models/lstm_predictor.pt')
            
            # Load DQN model
            if self.dqn_agent:
                self.dqn_agent.load_model('models/dqn_agent.pt')
            
            logger.info("Models loaded successfully")
            
        except Exception as e:
            logger.warning(f"Could not load models: {e}")
    
    async def _save_models(self):
        """Save trained models"""
        try:
            # Save LSTM model
            if self.lstm_predictor:
                await self.lstm_predictor.save_model('models/lstm_predictor.pt')
            
            # Save DQN model
            if self.dqn_agent:
                self.dqn_agent.save_model('models/dqn_agent.pt')
            
            logger.info("Models saved successfully")
            
        except Exception as e:
            logger.error(f"Error saving models: {e}")
    
    async def _generate_performance_report(self):
        """Generate final performance report"""
        try:
            report = {
                'agent_id': self.config.agent_id,
                'total_trades': self.total_trades,
                'successful_trades': self.successful_trades,
                'win_rate': self.successful_trades / max(self.total_trades, 1),
                'total_pnl': self.total_pnl,
                'max_drawdown': self.max_drawdown,
                'final_positions': len(self.current_positions),
                'trading_mode': self.config.trading_mode.value,
                'timestamp': datetime.now().isoformat()
            }
            
            logger.info(f"Performance Report: {report}")
            
            # Save to file
            import json
            with open(f'reports/{self.config.agent_id}_performance.json', 'w') as f:
                json.dump(report, f, indent=2)
                
        except Exception as e:
            logger.error(f"Error generating performance report: {e}")
    
    def get_status(self) -> Dict:
        """Get current agent status"""
        return {
            'agent_id': self.config.agent_id,
            'is_running': self.is_running,
            'total_trades': self.total_trades,
            'successful_trades': self.successful_trades,
            'win_rate': self.successful_trades / max(self.total_trades, 1),
            'total_pnl': self.total_pnl,
            'active_positions': len(self.current_positions),
            'current_risk_level': self.risk_manager.get_current_risk_level(),
            'last_update': datetime.now().isoformat()
        } 