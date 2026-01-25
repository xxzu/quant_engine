//! 实时行情数据模型

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 实时 Tick 数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickData {
    /// 股票代码
    pub code: String,
    /// 时间戳
    pub datetime: NaiveDateTime,
    /// 最新价
    pub last_price: Decimal,
    /// 成交量
    pub volume: i64,
    /// 成交额
    pub amount: Decimal,
    /// 买一价
    pub bid_price1: Decimal,
    /// 买一量
    pub bid_volume1: i64,
    /// 卖一价
    pub ask_price1: Decimal,
    /// 卖一量
    pub ask_volume1: i64,
    /// 开盘价
    pub open: Decimal,
    /// 最高价
    pub high: Decimal,
    /// 最低价
    pub low: Decimal,
    /// 昨收价
    pub pre_close: Decimal,
}

impl TickData {
    /// 计算涨跌幅
    pub fn change_pct(&self) -> Decimal {
        if self.pre_close.is_zero() {
            return Decimal::ZERO;
        }
        ((self.last_price - self.pre_close) / self.pre_close) * Decimal::from(100)
    }

    /// 获取买卖价差
    pub fn spread(&self) -> Decimal {
        self.ask_price1 - self.bid_price1
    }
}

/// 行情快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    /// 股票代码
    pub code: String,
    /// 股票名称
    pub name: String,
    /// 最新价
    pub last_price: Decimal,
    /// 涨跌幅 (%)
    pub change_pct: Decimal,
    /// 成交额 (万元)
    pub amount: Decimal,
    /// 换手率 (%)
    pub turnover: Decimal,
    /// 更新时间
    pub update_time: NaiveDateTime,
}

/// 行情订阅请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    /// 订阅的股票代码列表
    pub codes: Vec<String>,
}
