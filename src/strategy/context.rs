//! 策略运行上下文

use crate::exchange::types::*;
use rust_decimal::Decimal;

/// 策略运行上下文（由引擎注入）
#[derive(Debug, Clone)]
pub struct StrategyContext {
    /// 当前可用余额 (USDT)
    pub available_balance: Decimal,
    /// 总余额
    pub total_balance: Decimal,
    /// 当前持仓列表
    pub positions: Vec<FuturesPosition>,
    /// 合约信息
    pub contract_info: Option<ContractInfo>,
}

impl Default for StrategyContext {
    fn default() -> Self {
        Self {
            available_balance: Decimal::ZERO,
            total_balance: Decimal::ZERO,
            positions: Vec::new(),
            contract_info: None,
        }
    }
}

impl StrategyContext {
    /// 是否有持仓
    pub fn has_position(&self, symbol: &str) -> bool {
        self.positions
            .iter()
            .any(|p| p.symbol == symbol && !p.quantity.is_zero())
    }

    /// 获取指定交易对的持仓
    pub fn get_position(&self, symbol: &str) -> Option<&FuturesPosition> {
        self.positions
            .iter()
            .find(|p| p.symbol == symbol && !p.quantity.is_zero())
    }
}
