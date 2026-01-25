//! 模拟交易 (Paper Trading)

use crate::trading::order::{Order, OrderSide, OrderStatus, Trade};
use crate::trading::position::{Account, Position};
use anyhow::Result;
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

/// 手续费配置
#[derive(Debug, Clone)]
pub struct CommissionConfig {
    /// 佣金率 (默认万三)
    pub commission_rate: Decimal,
    /// 最低佣金
    pub min_commission: Decimal,
    /// 印花税率 (仅卖出)
    pub stamp_tax_rate: Decimal,
    /// 过户费率
    pub transfer_fee_rate: Decimal,
}

impl Default for CommissionConfig {
    fn default() -> Self {
        Self {
            commission_rate: Decimal::new(3, 4),  // 0.0003 = 万三
            min_commission: Decimal::from(5),     // 最低 5 元
            stamp_tax_rate: Decimal::new(1, 3),   // 0.001 = 千一 (仅卖出)
            transfer_fee_rate: Decimal::new(2, 5), // 0.00002
        }
    }
}

/// 模拟交易引擎
pub struct PaperBroker {
    /// 账户
    account: Account,
    /// 手续费配置
    commission_config: CommissionConfig,
    /// 当前价格缓存
    current_prices: HashMap<String, Decimal>,
    /// 成交记录
    trades: Vec<Trade>,
}

impl PaperBroker {
    /// 创建模拟交易引擎
    pub fn new(initial_cash: Decimal) -> Self {
        Self {
            account: Account::new(initial_cash),
            commission_config: CommissionConfig::default(),
            current_prices: HashMap::new(),
            trades: Vec::new(),
        }
    }

    /// 设置手续费配置
    pub fn with_commission(mut self, config: CommissionConfig) -> Self {
        self.commission_config = config;
        self
    }

    /// 更新股票价格
    pub fn update_price(&mut self, code: &str, price: Decimal) {
        self.current_prices.insert(code.to_string(), price);
        
        // 更新持仓市值
        if let Some(pos) = self.account.get_position_mut(code) {
            pos.update_price(price);
        }
        self.account.update_total_asset();
    }

    /// 执行订单
    pub fn execute_order(&mut self, mut order: Order, trade_time: NaiveDateTime) -> Result<Trade> {
        let price = order.price.unwrap_or_else(|| {
            *self.current_prices.get(&order.code).unwrap_or(&Decimal::ZERO)
        });

        // 计算手续费
        let commission = self.calculate_commission(&order, price);

        // 执行成交
        let trade = Trade {
            id: Uuid::new_v4().to_string(),
            order_id: order.id.clone(),
            code: order.code.clone(),
            side: order.side,
            price,
            quantity: order.quantity,
            commission,
            trade_time,
        };

        // 更新账户
        match order.side {
            OrderSide::Buy => {
                let cost = trade.amount() + commission;
                self.account.available_cash -= cost;

                if let Some(pos) = self.account.get_position_mut(&order.code) {
                    pos.add(order.quantity, price);
                } else {
                    let mut pos = Position::new(&order.code, order.quantity, price);
                    pos.update_price(price);
                    self.account.positions.push(pos);
                }
            }
            OrderSide::Sell => {
                let income = trade.amount() - commission;
                self.account.available_cash += income;

                if let Some(pos) = self.account.get_position_mut(&order.code) {
                    pos.reduce(order.quantity);
                }

                // 移除空仓
                self.account.positions.retain(|p| !p.is_empty());
            }
        }

        self.account.update_total_asset();
        order.status = OrderStatus::Filled;
        order.filled_quantity = order.quantity;
        order.avg_price = Some(price);

        self.trades.push(trade.clone());
        Ok(trade)
    }

    /// 计算手续费
    fn calculate_commission(&self, order: &Order, price: Decimal) -> Decimal {
        let amount = price * Decimal::from(order.quantity);

        // 佣金
        let mut commission = amount * self.commission_config.commission_rate;
        if commission < self.commission_config.min_commission {
            commission = self.commission_config.min_commission;
        }

        // 过户费
        commission += amount * self.commission_config.transfer_fee_rate;

        // 印花税 (仅卖出)
        if order.side == OrderSide::Sell {
            commission += amount * self.commission_config.stamp_tax_rate;
        }

        commission
    }

    /// 获取账户信息
    pub fn get_account(&self) -> &Account {
        &self.account
    }

    /// 获取成交记录
    pub fn get_trades(&self) -> &[Trade] {
        &self.trades
    }

    /// 新交易日开始 (更新可用持仓)
    pub fn new_trading_day(&mut self) {
        for pos in &mut self.account.positions {
            pos.available = pos.quantity;
        }
    }
}
