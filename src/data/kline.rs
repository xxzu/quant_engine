//! K线数据模型

use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// K线周期类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KlinePeriod {
    /// 1分钟
    Min1,
    /// 5分钟
    Min5,
    /// 15分钟
    Min15,
    /// 30分钟
    Min30,
    /// 60分钟
    Min60,
    /// 日线
    Daily,
    /// 周线
    Weekly,
    /// 月线
    Monthly,
}

impl std::fmt::Display for KlinePeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KlinePeriod::Min1 => write!(f, "1m"),
            KlinePeriod::Min5 => write!(f, "5m"),
            KlinePeriod::Min15 => write!(f, "15m"),
            KlinePeriod::Min30 => write!(f, "30m"),
            KlinePeriod::Min60 => write!(f, "60m"),
            KlinePeriod::Daily => write!(f, "1d"),
            KlinePeriod::Weekly => write!(f, "1w"),
            KlinePeriod::Monthly => write!(f, "1M"),
        }
    }
}

/// 日K线数据
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Kline {
    /// 主键 ID
    pub id: i64,
    /// 股票代码
    pub code: String,
    /// 交易日期
    pub date: NaiveDate,
    /// 开盘价
    pub open: Decimal,
    /// 最高价
    pub high: Decimal,
    /// 最低价
    pub low: Decimal,
    /// 收盘价
    pub close: Decimal,
    /// 成交量 (股)
    pub volume: i64,
    /// 成交额 (元)
    pub amount: Decimal,
    /// 涨跌幅 (%)
    pub change_pct: Option<Decimal>,
    /// 换手率 (%)
    pub turnover: Option<Decimal>,
    /// 创建时间
    pub created_at: NaiveDateTime,
}

impl Kline {
    /// 计算振幅
    pub fn amplitude(&self) -> Decimal {
        if self.low.is_zero() {
            return Decimal::ZERO;
        }
        ((self.high - self.low) / self.low) * Decimal::from(100)
    }

    /// 判断是否为阳线
    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    /// 判断是否为阴线
    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }

    /// 获取实体大小
    pub fn body_size(&self) -> Decimal {
        (self.close - self.open).abs()
    }

    /// 获取上影线长度
    pub fn upper_shadow(&self) -> Decimal {
        self.high - self.close.max(self.open)
    }

    /// 获取下影线长度
    pub fn lower_shadow(&self) -> Decimal {
        self.close.min(self.open) - self.low
    }
}

/// Bar 数据 (用于策略回测)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarData {
    pub code: String,
    pub datetime: NaiveDateTime,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: i64,
    pub amount: Decimal,
}

impl From<Kline> for BarData {
    fn from(kline: Kline) -> Self {
        Self {
            code: kline.code,
            datetime: kline.date.and_hms_opt(15, 0, 0).unwrap(),
            open: kline.open,
            high: kline.high,
            low: kline.low,
            close: kline.close,
            volume: kline.volume,
            amount: kline.amount,
        }
    }
}
