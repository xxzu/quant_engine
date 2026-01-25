//! 风控系统

use crate::trading::order::{Order, OrderSide};
use crate::trading::position::Account;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 风控错误
#[derive(Debug, Error)]
pub enum RiskError {
    #[error("资金不足: 需要 {required}, 可用 {available}")]
    InsufficientFunds { required: Decimal, available: Decimal },

    #[error("持仓不足: 需要 {required}, 可用 {available}")]
    InsufficientPosition { required: i64, available: i64 },

    #[error("超过单笔最大金额限制: {amount} > {limit}")]
    ExceedMaxOrderAmount { amount: Decimal, limit: Decimal },

    #[error("超过单日交易次数限制: {count} >= {limit}")]
    ExceedMaxDailyTrades { count: u32, limit: u32 },

    #[error("超过单只股票持仓比例: {ratio}% > {limit}%")]
    ExceedMaxPositionRatio { ratio: Decimal, limit: Decimal },

    #[error("禁止交易该股票: {code}")]
    StockBlacklisted { code: String },

    #[error("非交易时段")]
    NotTradingHours,
}

/// 风控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// 单笔最大金额
    pub max_order_amount: Decimal,
    /// 单日最大交易次数
    pub max_daily_trades: u32,
    /// 单只股票最大持仓比例 (%)
    pub max_position_ratio: Decimal,
    /// 黑名单股票
    pub blacklist: Vec<String>,
    /// 是否检查交易时段
    pub check_trading_hours: bool,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_order_amount: Decimal::from(100_000),
            max_daily_trades: 50,
            max_position_ratio: Decimal::from(20),
            blacklist: Vec::new(),
            check_trading_hours: false, // 回测时不检查
        }
    }
}

/// 风控管理器
pub struct RiskManager {
    config: RiskConfig,
    daily_trade_count: u32,
}

impl RiskManager {
    pub fn new(config: RiskConfig) -> Self {
        Self {
            config,
            daily_trade_count: 0,
        }
    }

    /// 检查订单风险
    pub fn check_order(&self, order: &Order, account: &Account) -> Result<(), RiskError> {
        // 检查黑名单
        if self.config.blacklist.contains(&order.code) {
            return Err(RiskError::StockBlacklisted {
                code: order.code.clone(),
            });
        }

        // 检查交易次数
        if self.daily_trade_count >= self.config.max_daily_trades {
            return Err(RiskError::ExceedMaxDailyTrades {
                count: self.daily_trade_count,
                limit: self.config.max_daily_trades,
            });
        }

        // 检查订单金额
        let order_amount = order.price.unwrap_or(Decimal::ZERO) * Decimal::from(order.quantity);
        if order_amount > self.config.max_order_amount {
            return Err(RiskError::ExceedMaxOrderAmount {
                amount: order_amount,
                limit: self.config.max_order_amount,
            });
        }

        match order.side {
            OrderSide::Buy => {
                // 检查资金
                if order_amount > account.available_cash {
                    return Err(RiskError::InsufficientFunds {
                        required: order_amount,
                        available: account.available_cash,
                    });
                }

                // 检查持仓比例
                let new_position_value = account
                    .get_position(&order.code)
                    .map_or(Decimal::ZERO, |p| p.market_value)
                    + order_amount;
                let new_total = account.total_asset + order_amount;
                if !new_total.is_zero() {
                    let ratio = (new_position_value / new_total) * Decimal::from(100);
                    if ratio > self.config.max_position_ratio {
                        return Err(RiskError::ExceedMaxPositionRatio {
                            ratio,
                            limit: self.config.max_position_ratio,
                        });
                    }
                }
            }
            OrderSide::Sell => {
                // 检查持仓
                let available = account
                    .get_position(&order.code)
                    .map_or(0, |p| p.available);
                if order.quantity > available {
                    return Err(RiskError::InsufficientPosition {
                        required: order.quantity,
                        available,
                    });
                }
            }
        }

        Ok(())
    }

    /// 重置每日计数
    pub fn reset_daily(&mut self) {
        self.daily_trade_count = 0;
    }

    /// 增加交易计数
    pub fn increment_trade_count(&mut self) {
        self.daily_trade_count += 1;
    }
}
