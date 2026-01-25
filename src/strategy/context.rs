//! 策略上下文

use rust_decimal::Decimal;
use std::collections::HashMap;

/// 策略运行上下文
#[derive(Debug, Clone)]
pub struct StrategyContext {
    /// 初始资金
    pub initial_capital: Decimal,
    /// 当前可用资金
    pub available_cash: Decimal,
    /// 持仓信息 (股票代码 -> 持仓数量)
    pub positions: HashMap<String, i64>,
    /// 策略参数
    pub params: HashMap<String, String>,
}

impl Default for StrategyContext {
    fn default() -> Self {
        Self::new(Decimal::from(100_000))
    }
}

impl StrategyContext {
    /// 创建新的策略上下文
    pub fn new(initial_capital: Decimal) -> Self {
        Self {
            initial_capital,
            available_cash: initial_capital,
            positions: HashMap::new(),
            params: HashMap::new(),
        }
    }

    /// 获取持仓数量
    pub fn get_position(&self, code: &str) -> i64 {
        *self.positions.get(code).unwrap_or(&0)
    }

    /// 更新持仓
    pub fn update_position(&mut self, code: &str, quantity: i64) {
        if quantity == 0 {
            self.positions.remove(code);
        } else {
            self.positions.insert(code.to_string(), quantity);
        }
    }

    /// 设置策略参数
    pub fn set_param(&mut self, key: &str, value: &str) {
        self.params.insert(key.to_string(), value.to_string());
    }

    /// 获取策略参数
    pub fn get_param(&self, key: &str) -> Option<&String> {
        self.params.get(key)
    }
}
