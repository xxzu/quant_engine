//! 持仓管理 - 合约交易版
//!
//! 注: 核心持仓类型已定义在 exchange::types::FuturesPosition
//! 此模块提供本地持仓汇总和 PnL 计算

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 持仓汇总统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PositionSummary {
    /// 总持仓市值
    pub total_notional: Decimal,
    /// 总未实现盈亏
    pub total_unrealized_pnl: Decimal,
    /// 总已实现盈亏
    pub total_realized_pnl: Decimal,
    /// 持仓数量
    pub position_count: u32,
}
