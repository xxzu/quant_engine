//! 交易所公共类型定义

use async_trait::async_trait;
use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// 交易所枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Exchange {
    Binance,
    OKX,
}

impl std::fmt::Display for Exchange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Exchange::Binance => write!(f, "Binance"),
            Exchange::OKX => write!(f, "OKX"),
        }
    }
}

/// 保证金模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarginMode {
    Isolated,
    Cross,
}

impl std::fmt::Display for MarginMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarginMode::Isolated => write!(f, "ISOLATED"),
            MarginMode::Cross => write!(f, "CROSSED"),
        }
    }
}

/// 持仓方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionSide {
    Long,
    Short,
    Both,
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Long => write!(f, "LONG"),
            PositionSide::Short => write!(f, "SHORT"),
            PositionSide::Both => write!(f, "BOTH"),
        }
    }
}

/// 订单方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

/// 订单类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
    StopMarket,
    TakeProfitMarket,
    Stop,
    TakeProfit,
    TrailingStopMarket,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Market => write!(f, "MARKET"),
            OrderType::Limit => write!(f, "LIMIT"),
            OrderType::StopMarket => write!(f, "STOP_MARKET"),
            OrderType::TakeProfitMarket => write!(f, "TAKE_PROFIT_MARKET"),
            OrderType::Stop => write!(f, "STOP"),
            OrderType::TakeProfit => write!(f, "TAKE_PROFIT"),
            OrderType::TrailingStopMarket => write!(f, "TRAILING_STOP_MARKET"),
        }
    }
}

/// 订单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

/// 合约信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    pub symbol: String,
    pub exchange: Exchange,
    pub price_precision: u8,
    pub quantity_precision: u8,
    pub min_quantity: Decimal,
    pub max_leverage: u32,
    pub tick_size: Decimal,
    pub step_size: Decimal,
}

/// K线数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kline {
    pub symbol: String,
    pub interval: String,
    pub open_time: i64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub close_time: i64,
    pub is_closed: bool,
}

/// 实时行情 Tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerData {
    pub symbol: String,
    pub last_price: Decimal,
    pub mark_price: Decimal,
    pub bid_price: Decimal,
    pub ask_price: Decimal,
    pub volume_24h: Decimal,
    pub change_pct_24h: Decimal,
    pub timestamp: i64,
}

/// 合约账户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesAccount {
    pub total_balance: Decimal,
    pub available_balance: Decimal,
    pub unrealized_pnl: Decimal,
    pub margin_used: Decimal,
}

/// 合约持仓
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuturesPosition {
    pub symbol: String,
    pub position_side: PositionSide,
    pub quantity: Decimal,
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    pub unrealized_pnl: Decimal,
    pub leverage: u32,
    pub margin_mode: MarginMode,
    pub liquidation_price: Decimal,
    pub margin: Decimal,
}

/// 下单请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: Option<Decimal>,
    pub price: Option<Decimal>,
    pub stop_price: Option<Decimal>,
    pub position_side: Option<PositionSide>,
    pub reduce_only: Option<bool>,
    pub time_in_force: Option<String>,
    pub close_position: Option<bool>,
}

/// 下单响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order_id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub status: OrderStatus,
    pub price: Decimal,
    pub quantity: Decimal,
    pub executed_qty: Decimal,
    pub avg_price: Decimal,
    pub timestamp: i64,
}

/// 用户数据事件（WebSocket 推送）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserDataEvent {
    OrderUpdate(OrderResponse),
    PositionUpdate(Vec<FuturesPosition>),
    AccountUpdate(FuturesAccount),
}

/// 交易所 API 统一接口
#[async_trait]
pub trait ExchangeApi: Send + Sync {
    /// 获取合约信息
    async fn get_contract_info(&self, symbol: &str) -> Result<ContractInfo>;

    /// 获取历史K线
    async fn get_klines(&self, symbol: &str, interval: &str, limit: u32) -> Result<Vec<Kline>>;

    /// 获取当前价格
    async fn get_ticker(&self, symbol: &str) -> Result<TickerData>;

    /// 获取账户信息
    async fn get_account(&self) -> Result<FuturesAccount>;

    /// 获取持仓列表
    async fn get_positions(&self, symbol: Option<&str>) -> Result<Vec<FuturesPosition>>;

    /// 设置杠杆
    async fn set_leverage(&self, symbol: &str, leverage: u32) -> Result<()>;

    /// 设置保证金模式
    async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()>;

    /// 下单
    async fn place_order(&self, req: &OrderRequest) -> Result<OrderResponse>;

    /// 撤单
    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<()>;

    /// 撤销某交易对全部挂单
    async fn cancel_all_orders(&self, symbol: &str) -> Result<()>;

    /// 查询订单
    async fn get_order(&self, symbol: &str, order_id: &str) -> Result<OrderResponse>;

    /// 订阅 K线 WebSocket
    async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<broadcast::Receiver<Kline>>;

    /// 订阅行情 WebSocket
    async fn subscribe_ticker(&self, symbol: &str) -> Result<broadcast::Receiver<TickerData>>;

    /// 订阅用户数据 WebSocket
    async fn subscribe_user_data(&self) -> Result<broadcast::Receiver<UserDataEvent>>;
}
