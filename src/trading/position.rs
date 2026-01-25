//! 持仓管理

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 持仓信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// 股票代码
    pub code: String,
    /// 股票名称
    pub name: Option<String>,
    /// 持仓数量
    pub quantity: i64,
    /// 可用数量 (T+1 可卖)
    pub available: i64,
    /// 成本价
    pub cost_price: Decimal,
    /// 当前价
    pub current_price: Decimal,
    /// 市值
    pub market_value: Decimal,
    /// 盈亏金额
    pub profit: Decimal,
    /// 盈亏比例 (%)
    pub profit_pct: Decimal,
    /// 更新时间
    pub updated_at: NaiveDateTime,
}

impl Position {
    /// 创建新持仓
    pub fn new(code: &str, quantity: i64, cost_price: Decimal) -> Self {
        let market_value = cost_price * Decimal::from(quantity);
        Self {
            code: code.to_string(),
            name: None,
            quantity,
            available: 0, // T+1 限制
            cost_price,
            current_price: cost_price,
            market_value,
            profit: Decimal::ZERO,
            profit_pct: Decimal::ZERO,
            updated_at: chrono::Local::now().naive_local(),
        }
    }

    /// 更新当前价格
    pub fn update_price(&mut self, current_price: Decimal) {
        self.current_price = current_price;
        self.market_value = current_price * Decimal::from(self.quantity);
        self.profit = self.market_value - (self.cost_price * Decimal::from(self.quantity));
        
        if !self.cost_price.is_zero() {
            self.profit_pct = (self.profit / (self.cost_price * Decimal::from(self.quantity))) 
                * Decimal::from(100);
        }
        
        self.updated_at = chrono::Local::now().naive_local();
    }

    /// 加仓
    pub fn add(&mut self, quantity: i64, price: Decimal) {
        let total_cost = self.cost_price * Decimal::from(self.quantity) 
            + price * Decimal::from(quantity);
        let total_qty = self.quantity + quantity;

        if total_qty > 0 {
            self.cost_price = total_cost / Decimal::from(total_qty);
        }
        self.quantity = total_qty;
        self.update_price(self.current_price);
    }

    /// 减仓
    pub fn reduce(&mut self, quantity: i64) {
        self.quantity -= quantity;
        self.available = self.available.min(self.quantity);
        self.update_price(self.current_price);
    }

    /// 是否空仓
    pub fn is_empty(&self) -> bool {
        self.quantity == 0
    }
}

/// 账户资金
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// 总资产
    pub total_asset: Decimal,
    /// 可用资金
    pub available_cash: Decimal,
    /// 冻结资金
    pub frozen_cash: Decimal,
    /// 持仓市值
    pub market_value: Decimal,
    /// 持仓列表
    pub positions: Vec<Position>,
}

impl Account {
    /// 创建新账户
    pub fn new(initial_cash: Decimal) -> Self {
        Self {
            total_asset: initial_cash,
            available_cash: initial_cash,
            frozen_cash: Decimal::ZERO,
            market_value: Decimal::ZERO,
            positions: Vec::new(),
        }
    }

    /// 更新总资产
    pub fn update_total_asset(&mut self) {
        self.market_value = self.positions.iter().map(|p| p.market_value).sum();
        self.total_asset = self.available_cash + self.frozen_cash + self.market_value;
    }

    /// 获取持仓
    pub fn get_position(&self, code: &str) -> Option<&Position> {
        self.positions.iter().find(|p| p.code == code)
    }

    /// 获取可变持仓
    pub fn get_position_mut(&mut self, code: &str) -> Option<&mut Position> {
        self.positions.iter_mut().find(|p| p.code == code)
    }
}
