//! 交易信号定义

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 信号方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalDirection {
    /// 买入
    Buy,
    /// 卖出
    Sell,
}

/// 信号类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    /// 开仓
    Open,
    /// 平仓
    Close,
    /// 加仓
    Add,
    /// 减仓
    Reduce,
}

/// 交易信号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// 信号 ID
    pub id: String,
    /// 股票代码
    pub code: String,
    /// 信号方向
    pub direction: SignalDirection,
    /// 信号类型
    pub signal_type: SignalType,
    /// 目标价格 (限价单使用)
    pub price: Option<Decimal>,
    /// 目标数量 (股)
    pub quantity: i64,
    /// 信号强度 (0.0 ~ 1.0)
    pub strength: f64,
    /// 生成时间
    pub timestamp: NaiveDateTime,
    /// 信号来源策略
    pub strategy_name: String,
    /// 备注
    pub remark: Option<String>,
}

impl Signal {
    /// 创建买入开仓信号
    pub fn buy_open(
        code: &str,
        quantity: i64,
        price: Option<Decimal>,
        strategy_name: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            code: code.to_string(),
            direction: SignalDirection::Buy,
            signal_type: SignalType::Open,
            price,
            quantity,
            strength: 1.0,
            timestamp: chrono::Local::now().naive_local(),
            strategy_name: strategy_name.to_string(),
            remark: None,
        }
    }

    /// 创建卖出平仓信号
    pub fn sell_close(
        code: &str,
        quantity: i64,
        price: Option<Decimal>,
        strategy_name: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            code: code.to_string(),
            direction: SignalDirection::Sell,
            signal_type: SignalType::Close,
            price,
            quantity,
            strength: 1.0,
            timestamp: chrono::Local::now().naive_local(),
            strategy_name: strategy_name.to_string(),
            remark: None,
        }
    }

    /// 设置信号强度
    pub fn with_strength(mut self, strength: f64) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }

    /// 设置备注
    pub fn with_remark(mut self, remark: &str) -> Self {
        self.remark = Some(remark.to_string());
        self
    }
}
