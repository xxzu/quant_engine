//! 合约风控系统

use crate::exchange::types::*;
use crate::strategy::signal::Signal;
use anyhow::Result;
use rust_decimal::Decimal;
use thiserror::Error;
use tracing::warn;

/// 风控错误
#[derive(Debug, Error)]
pub enum RiskError {
    #[error("余额不足: 需要 {required}U, 可用 {available}U")]
    InsufficientBalance { required: Decimal, available: Decimal },

    #[error("超过最大同时持仓数: {count} >= {limit}")]
    ExceedMaxPositions { count: u32, limit: u32 },

    #[error("单日亏损已达上限: {loss}U >= {limit}U")]
    ExceedDailyLoss { loss: Decimal, limit: Decimal },

    #[error("余额低于 {threshold}U 时必须使用逐仓模式")]
    MustUseIsolated { threshold: Decimal },

    #[error("止损未设置，拒绝开仓")]
    NoStopLoss,

    #[error("冷却期中，{remaining_secs}秒后可交易")]
    InCooldown { remaining_secs: i64 },
}

/// 合约风控管理器
pub struct RiskManager {
    pub max_daily_loss: Decimal,
    pub max_concurrent_positions: u32,
    pub force_isolated_below: Decimal,
    daily_loss: Decimal,
    current_position_count: u32,
}

impl RiskManager {
    pub fn new(config: &crate::config::sys_config::RiskConfig) -> Self {
        Self {
            max_daily_loss: Decimal::from_f64_retain(config.max_daily_loss)
                .unwrap_or(Decimal::from(50)),
            max_concurrent_positions: config.max_concurrent_positions,
            force_isolated_below: Decimal::from_f64_retain(config.force_isolated_below)
                .unwrap_or(Decimal::from(1000)),
            daily_loss: Decimal::ZERO,
            current_position_count: 0,
        }
    }

    /// 检查信号是否通过风控
    pub fn check_signal(
        &self,
        signal: &Signal,
        account: &FuturesAccount,
    ) -> Result<(), RiskError> {
        // 1. 检查余额
        if signal.amount_usdt > account.available_balance {
            return Err(RiskError::InsufficientBalance {
                required: signal.amount_usdt,
                available: account.available_balance,
            });
        }

        // 2. 检查持仓数量
        if self.current_position_count >= self.max_concurrent_positions {
            return Err(RiskError::ExceedMaxPositions {
                count: self.current_position_count,
                limit: self.max_concurrent_positions,
            });
        }

        // 3. 检查单日亏损
        if self.daily_loss >= self.max_daily_loss {
            return Err(RiskError::ExceedDailyLoss {
                loss: self.daily_loss,
                limit: self.max_daily_loss,
            });
        }

        // 4. 检查止损必须设置
        if signal.stop_loss_pct.is_none() {
            return Err(RiskError::NoStopLoss);
        }

        // 5. 余额低于阈值必须逐仓
        if account.total_balance < self.force_isolated_below
            && signal.margin_mode != MarginMode::Isolated
        {
            return Err(RiskError::MustUseIsolated {
                threshold: self.force_isolated_below,
            });
        }

        Ok(())
    }

    /// 记录亏损
    pub fn record_loss(&mut self, amount: Decimal) {
        self.daily_loss += amount;
        warn!("📉 记录亏损: {}U, 当日累计: {}U / {}U",
            amount, self.daily_loss, self.max_daily_loss);
    }

    /// 重置每日统计
    pub fn reset_daily(&mut self) {
        self.daily_loss = Decimal::ZERO;
        self.current_position_count = 0;
    }

    /// 更新持仓数量
    pub fn update_position_count(&mut self, count: u32) {
        self.current_position_count = count;
    }
}
