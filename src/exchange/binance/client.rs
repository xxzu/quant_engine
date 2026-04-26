//! 币安合约 REST API 客户端

use super::models::*;
use crate::exchange::types::*;
use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use reqwest::Client;
use rust_decimal::Decimal;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::str::FromStr;
use tokio::sync::broadcast;
use tracing::info;

type HmacSha256 = Hmac<Sha256>;

/// 币安合约客户端
pub struct BinanceClient {
    api_key: String,
    secret_key: String,
    base_url: String,
    ws_url: String,
    http: Client,
}

impl BinanceClient {
    /// 创建新客户端
    pub fn new(api_key: &str, secret_key: &str, base_url: &str, ws_url: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            secret_key: secret_key.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            ws_url: ws_url.trim_end_matches('/').to_string(),
            http: Client::new(),
        }
    }

    /// 从配置创建（测试网 / 主网自动切换）
    pub fn from_config(config: &crate::config::sys_config::BinanceConfig) -> Self {
        let (base_url, ws_url) = if config.testnet {
            (
                "https://testnet.binancefuture.com".to_string(),
                "wss://fstream.binancefuture.com/ws".to_string(),
            )
        } else {
            (
                "https://fapi.binance.com".to_string(),
                "wss://fstream.binance.com/ws".to_string(),
            )
        };
        Self::new(&config.api_key, &config.secret_key, &base_url, &ws_url)
    }

    /// HMAC-SHA256 签名
    fn sign(&self, query_string: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(self.secret_key.as_bytes())
            .expect("HMAC key error");
        mac.update(query_string.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// 构建带签名的查询字符串
    fn build_signed_query(&self, params: &BTreeMap<&str, String>) -> String {
        let query: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        let signature = self.sign(&query);
        format!("{}&signature={}", query, signature)
    }

    /// 获取当前时间戳
    fn timestamp() -> String {
        chrono::Utc::now().timestamp_millis().to_string()
    }

    /// 发送签名 GET 请求
    async fn signed_get(&self, path: &str, params: &BTreeMap<&str, String>) -> Result<String> {
        let query = self.build_signed_query(params);
        let url = format!("{}{}?{}", self.base_url, path, query);

        let resp = self.http
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            return Err(anyhow!("Binance API error [{}]: {}", status, body));
        }
        Ok(body)
    }

    /// 发送签名 POST 请求
    async fn signed_post(&self, path: &str, params: &BTreeMap<&str, String>) -> Result<String> {
        let query = self.build_signed_query(params);
        let url = format!("{}{}?{}", self.base_url, path, query);

        let resp = self.http
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            return Err(anyhow!("Binance API error [{}]: {}", status, body));
        }
        Ok(body)
    }

    /// 发送签名 DELETE 请求
    async fn signed_delete(&self, path: &str, params: &BTreeMap<&str, String>) -> Result<String> {
        let query = self.build_signed_query(params);
        let url = format!("{}{}?{}", self.base_url, path, query);

        let resp = self.http
            .delete(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            return Err(anyhow!("Binance API error [{}]: {}", status, body));
        }
        Ok(body)
    }

    /// 发送公开 GET 请求（无需签名）
    async fn public_get(&self, path: &str, params: &[(&str, &str)]) -> Result<String> {
        let url = format!("{}{}", self.base_url, path);

        let resp = self.http
            .get(&url)
            .query(params)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            return Err(anyhow!("Binance API error [{}]: {}", status, body));
        }
        Ok(body)
    }

    /// 解析 Decimal
    fn parse_decimal(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap_or(Decimal::ZERO)
    }

    /// 解析订单方向
    fn parse_order_side(s: &str) -> OrderSide {
        match s {
            "BUY" => OrderSide::Buy,
            _ => OrderSide::Sell,
        }
    }

    /// 解析订单类型
    fn parse_order_type(s: &str) -> OrderType {
        match s {
            "MARKET" => OrderType::Market,
            "LIMIT" => OrderType::Limit,
            "STOP_MARKET" => OrderType::StopMarket,
            "TAKE_PROFIT_MARKET" => OrderType::TakeProfitMarket,
            "STOP" => OrderType::Stop,
            "TAKE_PROFIT" => OrderType::TakeProfit,
            "TRAILING_STOP_MARKET" => OrderType::TrailingStopMarket,
            _ => OrderType::Market,
        }
    }

    /// 解析订单状态
    fn parse_order_status(s: &str) -> OrderStatus {
        match s {
            "NEW" => OrderStatus::New,
            "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
            "FILLED" => OrderStatus::Filled,
            "CANCELED" => OrderStatus::Canceled,
            "REJECTED" => OrderStatus::Rejected,
            "EXPIRED" => OrderStatus::Expired,
            _ => OrderStatus::New,
        }
    }

    /// 解析保证金模式
    fn parse_margin_mode(s: &str) -> MarginMode {
        match s.to_lowercase().as_str() {
            "isolated" => MarginMode::Isolated,
            _ => MarginMode::Cross,
        }
    }

    /// 解析持仓方向
    fn parse_position_side(s: &str) -> PositionSide {
        match s {
            "LONG" => PositionSide::Long,
            "SHORT" => PositionSide::Short,
            _ => PositionSide::Both,
        }
    }

    /// 获取 listenKey（用户数据流）
    pub async fn get_listen_key(&self) -> Result<String> {
        let url = format!("{}/fapi/v1/listenKey", self.base_url);
        let resp = self.http
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        let body = resp.text().await?;
        let parsed: ListenKeyResponse = serde_json::from_str(&body)?;
        Ok(parsed.listen_key)
    }

    /// 获取 WebSocket URL
    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }
}

#[async_trait::async_trait]
impl ExchangeApi for BinanceClient {
    /// 获取合约信息
    async fn get_contract_info(&self, symbol: &str) -> Result<ContractInfo> {
        let body = self.public_get("/fapi/v1/exchangeInfo", &[]).await?;
        let info: ExchangeInfoResponse = serde_json::from_str(&body)?;

        let sym = info.symbols.iter()
            .find(|s| s.symbol == symbol)
            .ok_or_else(|| anyhow!("Symbol {} not found", symbol))?;

        let mut tick_size = Decimal::ZERO;
        let mut step_size = Decimal::ZERO;
        let mut min_qty = Decimal::ZERO;

        for filter in &sym.filters {
            match filter {
                SymbolFilter::PriceFilter { tick_size: ts } => {
                    tick_size = Self::parse_decimal(ts);
                }
                SymbolFilter::LotSize { step_size: ss, min_qty: mq } => {
                    step_size = Self::parse_decimal(ss);
                    min_qty = Self::parse_decimal(mq);
                }
                _ => {}
            }
        }

        Ok(ContractInfo {
            symbol: sym.symbol.clone(),
            exchange: Exchange::Binance,
            price_precision: sym.price_precision,
            quantity_precision: sym.quantity_precision,
            min_quantity: min_qty,
            max_leverage: 125,
            tick_size,
            step_size,
        })
    }

    /// 获取 K线数据
    async fn get_klines(&self, symbol: &str, interval: &str, limit: u32) -> Result<Vec<Kline>> {
        let limit_str = limit.to_string();
        let body = self.public_get("/fapi/v1/klines", &[
            ("symbol", symbol),
            ("interval", interval),
            ("limit", &limit_str),
        ]).await?;

        let raw: Vec<KlineRaw> = serde_json::from_str(&body)?;
        let mut klines = Vec::with_capacity(raw.len());

        for item in &raw {
            if item.len() < 7 { continue; }
            klines.push(Kline {
                symbol: symbol.to_string(),
                interval: interval.to_string(),
                open_time: item[0].as_i64().unwrap_or(0),
                open: Self::parse_decimal(item[1].as_str().unwrap_or("0")),
                high: Self::parse_decimal(item[2].as_str().unwrap_or("0")),
                low: Self::parse_decimal(item[3].as_str().unwrap_or("0")),
                close: Self::parse_decimal(item[4].as_str().unwrap_or("0")),
                volume: Self::parse_decimal(item[5].as_str().unwrap_or("0")),
                close_time: item[6].as_i64().unwrap_or(0),
                is_closed: true,
            });
        }

        Ok(klines)
    }

    /// 获取当前价格
    async fn get_ticker(&self, symbol: &str) -> Result<TickerData> {
        let body = self.public_get("/fapi/v1/ticker/24hr", &[
            ("symbol", symbol),
        ]).await?;

        let raw: serde_json::Value = serde_json::from_str(&body)?;
        Ok(TickerData {
            symbol: symbol.to_string(),
            last_price: Self::parse_decimal(raw["lastPrice"].as_str().unwrap_or("0")),
            mark_price: Decimal::ZERO,
            bid_price: Self::parse_decimal(raw["bidPrice"].as_str().unwrap_or("0")),
            ask_price: Self::parse_decimal(raw["askPrice"].as_str().unwrap_or("0")),
            volume_24h: Self::parse_decimal(raw["volume"].as_str().unwrap_or("0")),
            change_pct_24h: Self::parse_decimal(raw["priceChangePercent"].as_str().unwrap_or("0")),
            timestamp: raw["closeTime"].as_i64().unwrap_or(0),
        })
    }

    /// 获取账户信息
    async fn get_account(&self) -> Result<FuturesAccount> {
        let mut params = BTreeMap::new();
        params.insert("timestamp", Self::timestamp());
        let body = self.signed_get("/fapi/v2/account", &params).await?;
        let raw: AccountResponse = serde_json::from_str(&body)?;

        Ok(FuturesAccount {
            total_balance: Self::parse_decimal(&raw.total_wallet_balance),
            available_balance: Self::parse_decimal(&raw.available_balance),
            unrealized_pnl: Self::parse_decimal(&raw.total_unrealized_profit),
            margin_used: Self::parse_decimal(&raw.total_margin_balance)
                - Self::parse_decimal(&raw.available_balance),
        })
    }

    /// 获取持仓
    async fn get_positions(&self, symbol: Option<&str>) -> Result<Vec<FuturesPosition>> {
        let mut params = BTreeMap::new();
        params.insert("timestamp", Self::timestamp());
        let body = self.signed_get("/fapi/v2/positionRisk", &params).await?;
        let raw: Vec<PositionResponse> = serde_json::from_str(&body)?;

        let positions: Vec<FuturesPosition> = raw.iter()
            .filter(|p| {
                let amt = Self::parse_decimal(&p.position_amt);
                if amt.is_zero() { return false; }
                if let Some(s) = symbol { p.symbol == s } else { true }
            })
            .map(|p| FuturesPosition {
                symbol: p.symbol.clone(),
                position_side: Self::parse_position_side(&p.position_side),
                quantity: Self::parse_decimal(&p.position_amt),
                entry_price: Self::parse_decimal(&p.entry_price),
                mark_price: p.mark_price.as_ref().map(|s| Self::parse_decimal(s)).unwrap_or(Decimal::ZERO),
                unrealized_pnl: Self::parse_decimal(&p.un_realized_profit),
                leverage: p.leverage.parse().unwrap_or(1),
                margin_mode: Self::parse_margin_mode(p.margin_type.as_deref().unwrap_or("cross")),
                liquidation_price: p.liquidation_price.as_ref().map(|s| Self::parse_decimal(s)).unwrap_or(Decimal::ZERO),
                margin: p.isolated_margin.as_ref().map(|s| Self::parse_decimal(s)).unwrap_or(Decimal::ZERO),
            })
            .collect();

        Ok(positions)
    }

    /// 设置杠杆
    async fn set_leverage(&self, symbol: &str, leverage: u32) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("symbol", symbol.to_string());
        params.insert("leverage", leverage.to_string());
        params.insert("timestamp", Self::timestamp());

        self.signed_post("/fapi/v1/leverage", &params).await?;
        info!("✅ 杠杆已设置: {} -> {}x", symbol, leverage);
        Ok(())
    }

    /// 设置保证金模式
    async fn set_margin_mode(&self, symbol: &str, mode: MarginMode) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("symbol", symbol.to_string());
        params.insert("marginType", mode.to_string());
        params.insert("timestamp", Self::timestamp());

        match self.signed_post("/fapi/v1/marginType", &params).await {
            Ok(_) => {
                info!("✅ 保证金模式已设置: {} -> {:?}", symbol, mode);
            }
            Err(e) => {
                // -4046 = No need to change margin type (已经是该模式)
                if e.to_string().contains("-4046") {
                    info!("保证金模式无需更改: {} 已是 {:?}", symbol, mode);
                } else {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// 下单
    async fn place_order(&self, req: &OrderRequest) -> Result<OrderResponse> {
        let mut params = BTreeMap::new();
        params.insert("symbol", req.symbol.clone());
        params.insert("side", req.side.to_string());
        params.insert("type", req.order_type.to_string());
        params.insert("timestamp", Self::timestamp());

        if let Some(qty) = &req.quantity {
            params.insert("quantity", qty.to_string());
        }
        if let Some(price) = &req.price {
            params.insert("price", price.to_string());
        }
        if let Some(stop) = &req.stop_price {
            params.insert("stopPrice", stop.to_string());
        }
        if let Some(ps) = &req.position_side {
            params.insert("positionSide", ps.to_string());
        }
        if let Some(true) = req.reduce_only {
            params.insert("reduceOnly", "true".to_string());
        }
        if let Some(tif) = &req.time_in_force {
            params.insert("timeInForce", tif.clone());
        }
        if let Some(true) = req.close_position {
            params.insert("closePosition", "true".to_string());
        }

        let body = self.signed_post("/fapi/v1/order", &params).await?;
        let raw: OrderResponseRaw = serde_json::from_str(&body)?;

        let resp = OrderResponse {
            order_id: raw.order_id.to_string(),
            client_order_id: raw.client_order_id,
            symbol: raw.symbol,
            side: Self::parse_order_side(&raw.side),
            order_type: Self::parse_order_type(&raw.order_type),
            status: Self::parse_order_status(&raw.status),
            price: Self::parse_decimal(&raw.price),
            quantity: Self::parse_decimal(&raw.orig_qty),
            executed_qty: Self::parse_decimal(&raw.executed_qty),
            avg_price: Self::parse_decimal(&raw.avg_price),
            timestamp: raw.update_time,
        };

        info!("📝 订单已提交: {} {} {} {} @ {:?}",
            resp.symbol, resp.side, resp.order_type, resp.quantity, resp.price);

        Ok(resp)
    }

    /// 撤单
    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("symbol", symbol.to_string());
        params.insert("orderId", order_id.to_string());
        params.insert("timestamp", Self::timestamp());

        self.signed_delete("/fapi/v1/order", &params).await?;
        info!("❌ 订单已撤销: {} #{}", symbol, order_id);
        Ok(())
    }

    /// 撤销全部挂单
    async fn cancel_all_orders(&self, symbol: &str) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("symbol", symbol.to_string());
        params.insert("timestamp", Self::timestamp());

        self.signed_delete("/fapi/v1/allOpenOrders", &params).await?;
        info!("❌ 已撤销 {} 全部挂单", symbol);
        Ok(())
    }

    /// 查询订单
    async fn get_order(&self, symbol: &str, order_id: &str) -> Result<OrderResponse> {
        let mut params = BTreeMap::new();
        params.insert("symbol", symbol.to_string());
        params.insert("orderId", order_id.to_string());
        params.insert("timestamp", Self::timestamp());

        let body = self.signed_get("/fapi/v1/order", &params).await?;
        let raw: OrderResponseRaw = serde_json::from_str(&body)?;

        Ok(OrderResponse {
            order_id: raw.order_id.to_string(),
            client_order_id: raw.client_order_id,
            symbol: raw.symbol,
            side: Self::parse_order_side(&raw.side),
            order_type: Self::parse_order_type(&raw.order_type),
            status: Self::parse_order_status(&raw.status),
            price: Self::parse_decimal(&raw.price),
            quantity: Self::parse_decimal(&raw.orig_qty),
            executed_qty: Self::parse_decimal(&raw.executed_qty),
            avg_price: Self::parse_decimal(&raw.avg_price),
            timestamp: raw.update_time,
        })
    }

    /// 订阅 K线 (WebSocket)
    async fn subscribe_kline(&self, symbol: &str, interval: &str) -> Result<broadcast::Receiver<Kline>> {
        let stream = format!("{}@kline_{}", symbol.to_lowercase(), interval);
        let (tx, rx) = broadcast::channel(256);

        super::websocket::spawn_market_ws(
            &self.ws_url, &stream, tx, symbol, interval,
        ).await?;

        Ok(rx)
    }

    /// 订阅 Ticker (WebSocket)
    async fn subscribe_ticker(&self, symbol: &str) -> Result<broadcast::Receiver<TickerData>> {
        let stream = format!("{}@bookTicker", symbol.to_lowercase());
        let (tx, rx) = broadcast::channel(256);

        super::websocket::spawn_ticker_ws(
            &self.ws_url, &stream, tx, symbol,
        ).await?;

        Ok(rx)
    }

    /// 订阅用户数据 (WebSocket)
    async fn subscribe_user_data(&self) -> Result<broadcast::Receiver<UserDataEvent>> {
        let listen_key = self.get_listen_key().await?;
        let (tx, rx) = broadcast::channel(64);

        super::websocket::spawn_user_data_ws(
            &self.ws_url, &listen_key, tx,
        ).await?;

        Ok(rx)
    }
}
