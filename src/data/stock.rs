//! 股票基础信息模型

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 市场类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR")]
#[sqlx(rename_all = "UPPERCASE")]
pub enum Market {
    /// 上海证券交易所
    SH,
    /// 深圳证券交易所
    SZ,
    /// 北京证券交易所
    BJ,
}

impl std::fmt::Display for Market {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Market::SH => write!(f, "SH"),
            Market::SZ => write!(f, "SZ"),
            Market::BJ => write!(f, "BJ"),
        }
    }
}

/// 股票状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR")]
#[sqlx(rename_all = "UPPERCASE")]
pub enum StockStatus {
    /// 正常交易
    Normal,
    /// 停牌
    Suspended,
    /// 退市
    Delisted,
}

/// 股票基础信息
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Stock {
    /// 主键 ID
    pub id: i64,
    /// 股票代码 (如: "600000")
    pub code: String,
    /// 股票名称
    pub name: String,
    /// 市场
    pub market: String,
    /// 状态
    pub status: String,
    /// 行业分类
    pub industry: Option<String>,
    /// 上市日期
    pub list_date: Option<NaiveDate>,
    /// 创建时间
    pub created_at: chrono::NaiveDateTime,
    /// 更新时间
    pub updated_at: chrono::NaiveDateTime,
}

impl Stock {
    /// 获取完整股票代码 (如: "600000.SH")
    pub fn full_code(&self) -> String {
        format!("{}.{}", self.code, self.market)
    }

    /// 判断是否为上证股票
    pub fn is_sh(&self) -> bool {
        self.market == "SH"
    }

    /// 判断是否为深证股票
    pub fn is_sz(&self) -> bool {
        self.market == "SZ"
    }
}

/// 创建股票的请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStockRequest {
    pub code: String,
    pub name: String,
    pub market: String,
    pub industry: Option<String>,
    pub list_date: Option<NaiveDate>,
}
