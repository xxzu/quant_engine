//! 币安 WebSocket 客户端

use super::models::*;
use crate::exchange::types::*;
use anyhow::Result;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use std::str::FromStr;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};

/// 启动 K 线行情 WebSocket
pub async fn spawn_market_ws(
    ws_base_url: &str,
    stream: &str,
    tx: broadcast::Sender<Kline>,
    symbol: &str,
    interval: &str,
) -> Result<()> {
    let url = format!("{}/{}", ws_base_url, stream);
    let symbol = symbol.to_string();
    let interval = interval.to_string();

    tokio::spawn(async move {
        loop {
            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    info!("🔌 K线 WebSocket 已连接: {}", url);
                    let (_, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(event) = serde_json::from_str::<WsKlineEvent>(&text) {
                                    let kline = Kline {
                                        symbol: symbol.clone(),
                                        interval: interval.clone(),
                                        open_time: event.kline.open_time,
                                        open: parse_dec(&event.kline.open),
                                        high: parse_dec(&event.kline.high),
                                        low: parse_dec(&event.kline.low),
                                        close: parse_dec(&event.kline.close),
                                        volume: parse_dec(&event.kline.volume),
                                        close_time: event.kline.close_time,
                                        is_closed: event.kline.is_closed,
                                    };
                                    let _ = tx.send(kline);
                                }
                            }
                            Ok(Message::Ping(_data)) => {
                                // WebSocket Ping/Pong 由 tokio-tungstenite 自动处理
                            }
                            Ok(Message::Close(_)) => {
                                warn!("K线 WebSocket 连接关闭，5秒后重连...");
                                break;
                            }
                            Err(e) => {
                                error!("K线 WebSocket 错误: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    error!("K线 WebSocket 连接失败: {}", e);
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    Ok(())
}

/// 启动 Ticker 行情 WebSocket
pub async fn spawn_ticker_ws(
    ws_base_url: &str,
    stream: &str,
    tx: broadcast::Sender<TickerData>,
    symbol: &str,
) -> Result<()> {
    let url = format!("{}/{}", ws_base_url, stream);
    let symbol = symbol.to_string();

    tokio::spawn(async move {
        loop {
            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    info!("🔌 Ticker WebSocket 已连接: {}", url);
                    let (_, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text) {
                                    let ticker = TickerData {
                                        symbol: symbol.clone(),
                                        last_price: Decimal::ZERO,
                                        mark_price: Decimal::ZERO,
                                        bid_price: parse_dec(raw["b"].as_str().unwrap_or("0")),
                                        ask_price: parse_dec(raw["a"].as_str().unwrap_or("0")),
                                        volume_24h: Decimal::ZERO,
                                        change_pct_24h: Decimal::ZERO,
                                        timestamp: raw["T"].as_i64().unwrap_or(0),
                                    };
                                    let _ = tx.send(ticker);
                                }
                            }
                            Ok(Message::Close(_)) => {
                                break;
                            }
                            Err(e) => {
                                error!("Ticker WS error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    error!("Ticker WebSocket 连接失败: {}", e);
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    Ok(())
}

/// 启动用户数据 WebSocket（订单/持仓/账户更新）
pub async fn spawn_user_data_ws(
    ws_base_url: &str,
    listen_key: &str,
    tx: broadcast::Sender<UserDataEvent>,
) -> Result<()> {
    let url = format!("{}/{}", ws_base_url, listen_key);

    tokio::spawn(async move {
        loop {
            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    info!("🔌 用户数据 WebSocket 已连接");
                    let (_, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text) {
                                    let event_type = raw["e"].as_str().unwrap_or("");
                                    match event_type {
                                        "ORDER_TRADE_UPDATE" => {
                                            if let Ok(update) =
                                                serde_json::from_str::<WsOrderUpdate>(&text)
                                            {
                                                let order = OrderResponse {
                                                    order_id: update.order.order_id.to_string(),
                                                    client_order_id: String::new(),
                                                    symbol: update.order.symbol,
                                                    side: parse_side(&update.order.side),
                                                    order_type: parse_type(
                                                        &update.order.order_type,
                                                    ),
                                                    status: parse_status(&update.order.status),
                                                    price: parse_dec(&update.order.price),
                                                    quantity: parse_dec(&update.order.orig_qty),
                                                    executed_qty: parse_dec(
                                                        &update.order.executed_qty,
                                                    ),
                                                    avg_price: parse_dec(&update.order.avg_price),
                                                    timestamp: 0,
                                                };
                                                let _ = tx.send(UserDataEvent::OrderUpdate(order));
                                            }
                                        }
                                        "ACCOUNT_UPDATE" => {
                                            if let Ok(update) =
                                                serde_json::from_str::<WsAccountUpdate>(&text)
                                            {
                                                // 解析持仓变更
                                                let positions: Vec<FuturesPosition> = update
                                                    .account
                                                    .positions
                                                    .iter()
                                                    .filter(|p| {
                                                        let amt = parse_dec(&p.position_amount);
                                                        !amt.is_zero()
                                                    })
                                                    .map(|p| FuturesPosition {
                                                        symbol: p.symbol.clone(),
                                                        position_side: match p
                                                            .position_side
                                                            .as_str()
                                                        {
                                                            "LONG" => PositionSide::Long,
                                                            "SHORT" => PositionSide::Short,
                                                            _ => PositionSide::Both,
                                                        },
                                                        quantity: parse_dec(&p.position_amount),
                                                        entry_price: parse_dec(&p.entry_price),
                                                        mark_price: Decimal::ZERO,
                                                        unrealized_pnl: parse_dec(
                                                            &p.unrealized_pnl,
                                                        ),
                                                        leverage: 0,
                                                        margin_mode: MarginMode::Isolated,
                                                        liquidation_price: Decimal::ZERO,
                                                        margin: Decimal::ZERO,
                                                    })
                                                    .collect();

                                                let _ = tx
                                                    .send(UserDataEvent::PositionUpdate(positions));

                                                // 解析余额变更
                                                if let Some(usdt_bal) = update
                                                    .account
                                                    .balances
                                                    .iter()
                                                    .find(|b| b.asset == "USDT")
                                                {
                                                    let wallet =
                                                        parse_dec(&usdt_bal.wallet_balance);
                                                    let cross =
                                                        parse_dec(&usdt_bal.cross_wallet_balance);
                                                    let _ = tx.send(UserDataEvent::AccountUpdate(
                                                        FuturesAccount {
                                                            total_balance: wallet,
                                                            available_balance: cross,
                                                            unrealized_pnl: Decimal::ZERO,
                                                            margin_used: Decimal::ZERO,
                                                        },
                                                    ));
                                                }

                                                info!("📊 账户更新: 持仓和余额已同步");
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Ok(Message::Close(_)) => {
                                break;
                            }
                            Err(e) => {
                                error!("User data WS error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    error!("用户数据 WebSocket 连接失败: {}", e);
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    Ok(())
}

fn parse_dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or(Decimal::ZERO)
}

fn parse_side(s: &str) -> OrderSide {
    if s == "BUY" {
        OrderSide::Buy
    } else {
        OrderSide::Sell
    }
}

fn parse_type(s: &str) -> OrderType {
    match s {
        "MARKET" => OrderType::Market,
        "LIMIT" => OrderType::Limit,
        "STOP_MARKET" => OrderType::StopMarket,
        "TAKE_PROFIT_MARKET" => OrderType::TakeProfitMarket,
        _ => OrderType::Market,
    }
}

fn parse_status(s: &str) -> OrderStatus {
    match s {
        "NEW" => OrderStatus::New,
        "FILLED" => OrderStatus::Filled,
        "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
        "CANCELED" => OrderStatus::Canceled,
        _ => OrderStatus::New,
    }
}
