//! 订单管理 - 合约交易版
//!
//! 注: 核心订单类型已定义在 exchange::types 中
//! 此模块提供订单历史记录和本地状态管理

use crate::exchange::types::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 本地订单记录（持久化用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRecord {
    pub id: String,
    pub exchange_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Decimal,
    pub quantity: Decimal,
    pub executed_qty: Decimal,
    pub avg_price: Decimal,
    pub status: OrderStatus,
    pub leverage: u32,
    pub margin_mode: MarginMode,
    pub strategy_name: String,
    pub pnl: Decimal,
    pub commission: Decimal,
    pub created_at: i64,
    pub updated_at: i64,
}
