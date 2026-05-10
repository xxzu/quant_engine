//! 币安 API 响应模型

use serde::Deserialize;

/// 交易规则信息
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeInfoResponse {
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolInfo {
    pub symbol: String,
    pub price_precision: u8,
    pub quantity_precision: u8,
    pub filters: Vec<SymbolFilter>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "filterType")]
pub enum SymbolFilter {
    #[serde(rename = "PRICE_FILTER")]
    #[serde(rename_all = "camelCase")]
    PriceFilter { tick_size: String },
    #[serde(rename = "LOT_SIZE")]
    #[serde(rename_all = "camelCase")]
    LotSize { step_size: String, min_qty: String },
    #[serde(other)]
    Other,
}

/// 账户信息
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponse {
    pub total_wallet_balance: String,
    pub available_balance: String,
    pub total_unrealized_profit: String,
    pub total_margin_balance: String,
    pub positions: Vec<AccountPositionResponse>,
}

/// 账户信息中的持仓
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountPositionResponse {
    pub symbol: String,
    pub position_side: String,
    pub position_amt: String,
    pub entry_price: String,
    pub unrealized_profit: String,
    pub leverage: String,
    pub isolated: bool,
}

/// 持仓信息
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionResponse {
    pub symbol: String,
    pub position_side: String,
    pub position_amt: String,
    pub entry_price: String,
    pub mark_price: Option<String>,
    pub un_realized_profit: String,
    pub leverage: String,
    pub margin_type: Option<String>,
    pub liquidation_price: Option<String>,
    pub isolated_margin: Option<String>,
}

/// 下单响应
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponseRaw {
    pub order_id: u64,
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub status: String,
    pub price: String,
    pub orig_qty: String,
    pub executed_qty: String,
    pub avg_price: String,
    pub update_time: i64,
}

/// 设置杠杆响应
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeverageResponse {
    pub leverage: u32,
    pub symbol: String,
}

/// K线数据 (数组格式)
/// [open_time, open, high, low, close, volume, close_time, ...]
pub type KlineRaw = Vec<serde_json::Value>;

/// WebSocket K线事件
#[derive(Debug, Deserialize)]
pub struct WsKlineEvent {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "k")]
    pub kline: WsKlineData,
}

#[derive(Debug, Deserialize)]
pub struct WsKlineData {
    #[serde(rename = "t")]
    pub open_time: i64,
    #[serde(rename = "T")]
    pub close_time: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "i")]
    pub interval: String,
    #[serde(rename = "o")]
    pub open: String,
    #[serde(rename = "h")]
    pub high: String,
    #[serde(rename = "l")]
    pub low: String,
    #[serde(rename = "c")]
    pub close: String,
    #[serde(rename = "v")]
    pub volume: String,
    #[serde(rename = "x")]
    pub is_closed: bool,
}

/// WebSocket 24hr Ticker
#[derive(Debug, Deserialize)]
pub struct WsTickerEvent {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "c")]
    pub last_price: String,
    #[serde(rename = "b")]
    pub bid_price: String,
    #[serde(rename = "a")]
    pub ask_price: String,
    #[serde(rename = "P")]
    pub change_pct: String,
    #[serde(rename = "q")]
    pub volume: String,
}

/// ListenKey 响应
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListenKeyResponse {
    pub listen_key: String,
}

/// WebSocket 用户数据 - 订单更新
#[derive(Debug, Deserialize)]
pub struct WsOrderUpdate {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "o")]
    pub order: WsOrderData,
}

#[derive(Debug, Deserialize)]
pub struct WsOrderData {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "X")]
    pub status: String,
    #[serde(rename = "i")]
    pub order_id: u64,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub orig_qty: String,
    #[serde(rename = "z")]
    pub executed_qty: String,
    #[serde(rename = "ap")]
    pub avg_price: String,
    #[serde(rename = "rp")]
    pub realized_pnl: String,
}

/// WebSocket 用户数据 - 账户更新 (ACCOUNT_UPDATE)
#[derive(Debug, Deserialize)]
pub struct WsAccountUpdate {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "a")]
    pub account: WsAccountData,
}

#[derive(Debug, Deserialize)]
pub struct WsAccountData {
    #[serde(rename = "B")]
    pub balances: Vec<WsBalanceData>,
    #[serde(rename = "P")]
    pub positions: Vec<WsPositionData>,
}

#[derive(Debug, Deserialize)]
pub struct WsBalanceData {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "wb")]
    pub wallet_balance: String,
    #[serde(rename = "cw")]
    pub cross_wallet_balance: String,
}

#[derive(Debug, Deserialize)]
pub struct WsPositionData {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "pa")]
    pub position_amount: String,
    #[serde(rename = "ep")]
    pub entry_price: String,
    #[serde(rename = "up")]
    pub unrealized_pnl: String,
    #[serde(rename = "ps")]
    pub position_side: String,
}
