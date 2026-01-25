//! 订单管理

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 订单方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    /// 买入
    Buy,
    /// 卖出
    Sell,
}

/// 订单类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    /// 市价单
    Market,
    /// 限价单
    Limit,
    /// 止损单
    Stop,
    /// 止盈单
    TakeProfit,
}

/// 订单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    /// 待提交
    Pending,
    /// 已提交
    Submitted,
    /// 部分成交
    PartialFilled,
    /// 全部成交
    Filled,
    /// 已取消
    Cancelled,
    /// 已拒绝
    Rejected,
}

/// 订单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// 订单 ID
    pub id: String,
    /// 股票代码
    pub code: String,
    /// 订单方向
    pub side: OrderSide,
    /// 订单类型
    pub order_type: OrderType,
    /// 委托价格
    pub price: Option<Decimal>,
    /// 委托数量
    pub quantity: i64,
    /// 已成交数量
    pub filled_quantity: i64,
    /// 成交均价
    pub avg_price: Option<Decimal>,
    /// 订单状态
    pub status: OrderStatus,
    /// 创建时间
    pub created_at: NaiveDateTime,
    /// 更新时间
    pub updated_at: NaiveDateTime,
    /// 策略名称
    pub strategy_name: Option<String>,
    /// 备注
    pub remark: Option<String>,
}

impl Order {
    /// 创建市价买单
    pub fn market_buy(code: &str, quantity: i64) -> Self {
        Self::new(code, OrderSide::Buy, OrderType::Market, None, quantity)
    }

    /// 创建市价卖单
    pub fn market_sell(code: &str, quantity: i64) -> Self {
        Self::new(code, OrderSide::Sell, OrderType::Market, None, quantity)
    }

    /// 创建限价买单
    pub fn limit_buy(code: &str, price: Decimal, quantity: i64) -> Self {
        Self::new(code, OrderSide::Buy, OrderType::Limit, Some(price), quantity)
    }

    /// 创建限价卖单
    pub fn limit_sell(code: &str, price: Decimal, quantity: i64) -> Self {
        Self::new(code, OrderSide::Sell, OrderType::Limit, Some(price), quantity)
    }

    fn new(
        code: &str,
        side: OrderSide,
        order_type: OrderType,
        price: Option<Decimal>,
        quantity: i64,
    ) -> Self {
        let now = chrono::Local::now().naive_local();
        Self {
            id: Uuid::new_v4().to_string(),
            code: code.to_string(),
            side,
            order_type,
            price,
            quantity,
            filled_quantity: 0,
            avg_price: None,
            status: OrderStatus::Pending,
            created_at: now,
            updated_at: now,
            strategy_name: None,
            remark: None,
        }
    }

    /// 是否可取消
    pub fn can_cancel(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Pending | OrderStatus::Submitted | OrderStatus::PartialFilled
        )
    }

    /// 是否已完成
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Rejected
        )
    }

    /// 未成交数量
    pub fn unfilled_quantity(&self) -> i64 {
        self.quantity - self.filled_quantity
    }
}

/// 成交记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// 成交 ID
    pub id: String,
    /// 关联订单 ID
    pub order_id: String,
    /// 股票代码
    pub code: String,
    /// 成交方向
    pub side: OrderSide,
    /// 成交价格
    pub price: Decimal,
    /// 成交数量
    pub quantity: i64,
    /// 手续费
    pub commission: Decimal,
    /// 成交时间
    pub trade_time: NaiveDateTime,
}

impl Trade {
    /// 计算成交金额
    pub fn amount(&self) -> Decimal {
        self.price * Decimal::from(self.quantity)
    }

    /// 计算总成本 (包含手续费)
    pub fn total_cost(&self) -> Decimal {
        self.amount() + self.commission
    }
}
