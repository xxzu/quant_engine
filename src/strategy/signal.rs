//! 交易信号定义 - 合约交易版

use crate::exchange::types::MarginMode;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 信号方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalDirection {
    /// 开多
    OpenLong,
    /// 开空
    OpenShort,
    /// 平多
    CloseLong,
    /// 平空
    CloseShort,
}

/// 交易信号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// 信号 ID
    pub id: String,
    /// 交易对
    pub symbol: String,
    /// 信号方向
    pub direction: SignalDirection,
    /// 下单数量 (USDT 金额)
    pub amount_usdt: Decimal,
    /// 下单数量 (合约数量, 可选)
    pub quantity: Option<Decimal>,
    /// 杠杆倍数
    pub leverage: u32,
    /// 保证金模式
    pub margin_mode: MarginMode,
    /// 止损百分比
    pub stop_loss_pct: Option<Decimal>,
    /// 止盈百分比
    pub take_profit_pct: Option<Decimal>,
    /// 限价 (None = 市价)
    pub price: Option<Decimal>,
    /// 信号强度 (0.0 ~ 1.0)
    pub strength: f64,
    /// 生成时间
    pub timestamp: i64,
    /// 策略名称
    pub strategy_name: String,
    /// 备注
    pub remark: Option<String>,
}

impl Signal {
    /// 创建开多信号
    pub fn open_long(
        symbol: &str,
        amount_usdt: Decimal,
        leverage: u32,
        strategy_name: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            direction: SignalDirection::OpenLong,
            amount_usdt,
            quantity: None,
            leverage,
            margin_mode: MarginMode::Isolated,
            stop_loss_pct: Some(Decimal::from(20)),
            take_profit_pct: Some(Decimal::from(100)),
            price: None,
            strength: 1.0,
            timestamp: chrono::Utc::now().timestamp_millis(),
            strategy_name: strategy_name.to_string(),
            remark: None,
        }
    }

    /// 创建开空信号
    pub fn open_short(
        symbol: &str,
        amount_usdt: Decimal,
        leverage: u32,
        strategy_name: &str,
    ) -> Self {
        let mut signal = Self::open_long(symbol, amount_usdt, leverage, strategy_name);
        signal.direction = SignalDirection::OpenShort;
        signal
    }

    /// 创建平仓信号
    pub fn close(symbol: &str, direction: SignalDirection, strategy_name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            direction,
            amount_usdt: Decimal::ZERO,
            quantity: None,
            leverage: 1,
            margin_mode: MarginMode::Isolated,
            stop_loss_pct: None,
            take_profit_pct: None,
            price: None,
            strength: 1.0,
            timestamp: chrono::Utc::now().timestamp_millis(),
            strategy_name: strategy_name.to_string(),
            remark: None,
        }
    }

    /// 设置止损止盈
    pub fn with_sl_tp(mut self, stop_loss_pct: Decimal, take_profit_pct: Decimal) -> Self {
        self.stop_loss_pct = Some(stop_loss_pct);
        self.take_profit_pct = Some(take_profit_pct);
        self
    }
}
