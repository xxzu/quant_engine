//! 引擎状态共享模块

use crate::exchange::types::FuturesPosition;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 本地追踪的单笔订单（币安会合并同交易对持仓，我们在本地分开记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedOrder {
    pub id: String,              // 唯一ID (币安 order_id)
    pub symbol: String,
    pub direction: String,       // "long" or "short"
    pub quantity: Decimal,       // 开仓数量
    pub entry_price: Decimal,    // 开仓价格
    pub leverage: u32,
    pub amount_usdt: Decimal,    // 投入保证金 (USDT)
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub opened_at: i64,          // 开仓时间戳 (ms)
    pub status: String,          // "open", "closed"
    pub strategy_id: String,     // 所属策略ID
    pub closed_pnl: Option<Decimal>, // 平仓后的实际盈亏 (平仓时填入)
}

/// 策略配置（每个策略是一个独立的资金池）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub allocated_funds: Decimal,    // 分配的资金 (USDT)
    pub used_funds: Decimal,         // 当前使用中的资金
    pub total_pnl: Decimal,          // 累计盈亏
    pub win_count: u32,              // 盈利次数
    pub loss_count: u32,             // 亏损次数
}

impl StrategyConfig {
    /// 可用资金 = 分配资金 - 已用资金
    pub fn available_funds(&self) -> Decimal {
        self.allocated_funds - self.used_funds
    }
}

/// 引擎实时状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineState {
    pub is_running: bool,
    pub symbol: String,
    pub last_price: Decimal,
    pub total_balance: Decimal,
    pub available_balance: Decimal,
    pub unrealized_pnl: Decimal,
    pub positions: Vec<FuturesPosition>,
    pub strategy_stage: String,
    pub latest_logs: Vec<String>,
    /// 策略列表（每个策略有独立的资金池）
    pub strategies: Vec<StrategyConfig>,
    /// 本地追踪的独立订单列表（所有策略的订单汇总）
    pub tracked_orders: Vec<TrackedOrder>,
}

impl Default for EngineState {
    fn default() -> Self {
        let strategies = vec![
            StrategyConfig {
                id: "ema_cross".to_string(),
                name: "EMA 金叉死叉".to_string(),
                description: "基于 EMA7/EMA25 交叉信号，结合 RSI 过滤".to_string(),
                active: false,
                allocated_funds: Decimal::ZERO,
                used_funds: Decimal::ZERO,
                total_pnl: Decimal::ZERO,
                win_count: 0,
                loss_count: 0,
            },
            StrategyConfig {
                id: "rsi_reversal".to_string(),
                name: "RSI 超买超卖".to_string(),
                description: "RSI 低于30做多、高于70做空的反转策略".to_string(),
                active: false,
                allocated_funds: Decimal::ZERO,
                used_funds: Decimal::ZERO,
                total_pnl: Decimal::ZERO,
                win_count: 0,
                loss_count: 0,
            },
            StrategyConfig {
                id: "grid_trading".to_string(),
                name: "网格交易".to_string(),
                description: "在设定价格区间内自动挂单买卖，赚取波动差价".to_string(),
                active: false,
                allocated_funds: Decimal::ZERO,
                used_funds: Decimal::ZERO,
                total_pnl: Decimal::ZERO,
                win_count: 0,
                loss_count: 0,
            },
        ];

        Self {
            is_running: false,
            symbol: String::new(),
            last_price: Decimal::ZERO,
            total_balance: Decimal::ZERO,
            available_balance: Decimal::ZERO,
            unrealized_pnl: Decimal::ZERO,
            positions: vec![],
            strategy_stage: "等待连接".to_string(),
            latest_logs: vec![],
            strategies,
            tracked_orders: vec![],
        }
    }
}

pub type SharedEngineState = Arc<RwLock<EngineState>>;

/// 计算策略所处的阶段
pub fn calc_strategy_stage(balance: Decimal) -> String {
    if balance >= Decimal::from(200) {
        "稳健扩张期 (200U+)".to_string()
    } else if balance >= Decimal::from(80) {
        "进阶期 (80U - 200U)".to_string()
    } else if balance > Decimal::ZERO {
        "新手起步期 (10U - 80U)".to_string()
    } else {
        "等待资金".to_string()
    }
}
