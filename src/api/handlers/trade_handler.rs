//! 手动交易处理器

use crate::engine::state::{SharedEngineState, TrackedOrder};
use crate::exchange::types::*;
use axum::{Extension, Json};
use rust_decimal::Decimal;
use std::sync::Arc;

/// 手动下单请求
#[derive(Debug, serde::Deserialize)]
pub struct ManualOrderRequest {
    pub symbol: String,
    pub side: String,      // "buy" or "sell"
    pub direction: String, // "open_long", "open_short", "close_long", "close_short"
    pub amount_usdt: f64,
    pub leverage: u32,
    pub price: Option<f64>,
    pub stop_loss_pct: Option<f64>,
    pub take_profit_pct: Option<f64>,
    pub strategy_id: Option<String>, // 所属策略ID，留空表示手动交易
}

/// 关闭指定订单请求
#[derive(Debug, serde::Deserialize)]
pub struct CloseOrderRequest {
    pub order_id: String,
}

/// 手动下单
pub async fn place_manual_order(
    Extension(state): Extension<SharedEngineState>,
    Extension(exchange): Extension<Arc<dyn ExchangeApi>>,
    Json(req): Json<ManualOrderRequest>,
) -> Json<serde_json::Value> {
    tracing::info!(
        "📝 手动下单请求: {} {} {} 金额={}U 杠杆={}x 价格={:?}",
        req.symbol,
        req.direction,
        req.side,
        req.amount_usdt,
        req.leverage,
        req.price
    );

    let (is_running, available_balance) = {
        let st = state.read().await;
        (st.is_running, st.available_balance)
    };

    if !is_running {
        return Json(serde_json::json!({
            "success": false,
            "error": "交易引擎未运行，无法下单",
        }));
    }

    // 平仓操作不需要检查余额
    let is_close = req.direction == "close_long" || req.direction == "close_short";

    let amount = Decimal::from_f64_retain(req.amount_usdt).unwrap_or(Decimal::ZERO);
    let mut stop_loss_pct = req.stop_loss_pct;
    let mut take_profit_pct = req.take_profit_pct;
    let mut leverage = req.leverage;
    if !is_close {
        if amount > available_balance {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("余额不足: 需要 {}U, 可用 {}U", amount, available_balance),
            }));
        }

        if amount < Decimal::from(5) {
            return Json(serde_json::json!({
                "success": false,
                "error": "最小下单金额为 5U",
            }));
        }

        // 如果指定了策略，检查策略可用资金
        if let Some(ref sid) = req.strategy_id {
            let st = state.read().await;
            if let Some(strategy) = st.strategies.iter().find(|s| &s.id == sid) {
                if !strategy.active {
                    return Json(
                        serde_json::json!({ "success": false, "error": "该策略未启用，请先到策略管理中分配资金并开启" }),
                    );
                }

                if sid == "manual_discipline" {
                    let total_funds = strategy.allocated_funds + strategy.total_pnl;
                    let avail = strategy.available_funds();

                    let max_amount = if total_funds >= Decimal::from(200) {
                        std::cmp::max(
                            Decimal::from(20),
                            total_funds * Decimal::from_f64_retain(0.1).unwrap_or(Decimal::ZERO),
                        )
                    } else if total_funds >= Decimal::from(80) {
                        Decimal::from(10)
                    } else {
                        avail * Decimal::from_f64_retain(0.5).unwrap_or(Decimal::ZERO)
                    };

                    if amount > max_amount {
                        return Json(
                            serde_json::json!({ "success": false, "error": format!("纪律限制: 当前阶段单笔最大开仓金额为 {}U", max_amount.round_dp(2)) }),
                        );
                    }

                    // 强制覆盖杠杆和止损止盈
                    leverage = 100;
                    stop_loss_pct = Some(20.0);
                    take_profit_pct = Some(100.0);
                }

                if amount > strategy.available_funds() {
                    return Json(serde_json::json!({
                        "success": false,
                        "error": format!("策略可用资金不足: 需要 {}U, 可用 {}U", amount, strategy.available_funds()),
                    }));
                }
            }
        }
    }

    // 设置杠杆和保证金模式 (仅在开仓时设置)
    if !is_close {
        if let Err(e) = exchange.set_leverage(&req.symbol, leverage).await {
            tracing::error!("设置杠杆失败: {}", e);
        }
    }

    // 获取当前价格和合约精度信息
    let ticker = match exchange.get_ticker(&req.symbol).await {
        Ok(t) => t,
        Err(e) => {
            return Json(
                serde_json::json!({ "success": false, "error": format!("获取行情失败: {}", e) }),
            )
        }
    };
    let current_price = ticker.last_price;
    let order_price = req
        .price
        .map(|p| Decimal::from_f64_retain(p).unwrap_or(current_price))
        .unwrap_or(current_price);

    let contract = match exchange.get_contract_info(&req.symbol).await {
        Ok(c) => c,
        Err(e) => {
            return Json(
                serde_json::json!({ "success": false, "error": format!("获取合约信息失败: {}", e) }),
            )
        }
    };

    // 计算数量: 金额 * 杠杆 / 价格
    let notional = amount * Decimal::from(leverage);
    let quantity = (notional / order_price).round_dp(contract.quantity_precision as u32);

    // Binance 默认是单向持仓模式 (One-way mode)，这种模式下 position_side 必须是 BOTH
    let (order_side, is_close_flag) = match req.direction.as_str() {
        "open_long" => (OrderSide::Buy, false),
        "open_short" => (OrderSide::Sell, false),
        "close_long" => (OrderSide::Sell, true),
        "close_short" => (OrderSide::Buy, true),
        _ => return Json(serde_json::json!({ "success": false, "error": "无效的交易方向" })),
    };

    // 平仓时需要获取当前持仓数量
    let close_qty = if is_close_flag {
        match exchange.get_positions(Some(&req.symbol)).await {
            Ok(positions) => {
                let pos = positions.iter().find(|p| !p.quantity.is_zero());
                match pos {
                    Some(p) => p.quantity.abs(),
                    None => {
                        return Json(
                            serde_json::json!({ "success": false, "error": "当前没有该交易对的持仓" }),
                        )
                    }
                }
            }
            Err(e) => {
                return Json(
                    serde_json::json!({ "success": false, "error": format!("获取持仓失败: {}", e) }),
                )
            }
        }
    } else {
        quantity
    };

    let order_req = OrderRequest {
        symbol: req.symbol.clone(),
        side: order_side,
        order_type: if req.price.is_some() {
            OrderType::Limit
        } else {
            OrderType::Market
        },
        quantity: Some(close_qty),
        price: req.price.map(|p| {
            Decimal::from_f64_retain(p)
                .unwrap_or_default()
                .round_dp(contract.price_precision as u32)
        }),
        stop_price: None,
        position_side: Some(PositionSide::Both),
        reduce_only: if is_close_flag { Some(true) } else { None },
        time_in_force: if req.price.is_some() {
            Some("GTC".to_string())
        } else {
            None
        },
        close_position: None,
    };

    match exchange.place_order(&order_req).await {
        Ok(resp) => {
            tracing::info!("✅ 手动下单成功: {} {:?}", req.symbol, resp);

            // 开仓时记录到本地追踪列表
            if !is_close_flag {
                let direction = if order_side == OrderSide::Buy {
                    "long"
                } else {
                    "short"
                };
                let strategy_id = req
                    .strategy_id
                    .clone()
                    .unwrap_or_else(|| "manual".to_string());
                let tracked = TrackedOrder {
                    id: resp.order_id.clone(),
                    symbol: req.symbol.clone(),
                    direction: direction.to_string(),
                    quantity: close_qty,
                    entry_price: if resp.avg_price > Decimal::ZERO {
                        resp.avg_price
                    } else {
                        order_price
                    },
                    leverage,
                    amount_usdt: amount,
                    stop_loss_pct,
                    take_profit_pct,
                    opened_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64,
                    status: "open".to_string(),
                    strategy_id: strategy_id.clone(),
                    closed_pnl: None,
                };
                let mut st = state.write().await;
                st.tracked_orders.push(tracked);
                // 扣减策略可用资金
                if let Some(strategy) = st.strategies.iter_mut().find(|s| s.id == strategy_id) {
                    strategy.used_funds += amount;
                }
            } else {
                // 平仓时标记所有同交易对的 tracked_orders 为 closed
                let mut st = state.write().await;
                for order in st.tracked_orders.iter_mut() {
                    if order.symbol == req.symbol && order.status == "open" {
                        order.status = "closed".to_string();
                    }
                }
            }

            // 如果是开仓，设置止损止盈
            if !is_close_flag {
                if let Some(sl_pct) = stop_loss_pct {
                    let sl_pct_dec = Decimal::from_f64_retain(sl_pct).unwrap_or(Decimal::ZERO)
                        / Decimal::from(100);
                    let sl_price = if order_side == OrderSide::Buy {
                        order_price * (Decimal::ONE - sl_pct_dec)
                    } else {
                        order_price * (Decimal::ONE + sl_pct_dec)
                    };
                    let sl_price = sl_price.round_dp(contract.price_precision as u32);

                    let sl_req = OrderRequest {
                        symbol: req.symbol.clone(),
                        side: if order_side == OrderSide::Buy {
                            OrderSide::Sell
                        } else {
                            OrderSide::Buy
                        },
                        order_type: OrderType::StopMarket,
                        quantity: None,
                        price: None,
                        stop_price: Some(sl_price),
                        position_side: Some(PositionSide::Both),
                        reduce_only: None,
                        time_in_force: None,
                        close_position: Some(true),
                    };
                    match exchange.place_order(&sl_req).await {
                        Ok(_) => tracing::info!("🛑 止损已设置: {}", sl_price),
                        Err(e) => tracing::error!("⚠️ 止损设置失败: {}", e),
                    }
                }

                if let Some(tp_pct) = take_profit_pct {
                    let tp_pct_dec = Decimal::from_f64_retain(tp_pct).unwrap_or(Decimal::ZERO)
                        / Decimal::from(100);
                    let tp_price = if order_side == OrderSide::Buy {
                        order_price * (Decimal::ONE + tp_pct_dec)
                    } else {
                        order_price * (Decimal::ONE - tp_pct_dec)
                    };
                    let tp_price = tp_price.round_dp(contract.price_precision as u32);

                    let tp_req = OrderRequest {
                        symbol: req.symbol.clone(),
                        side: if order_side == OrderSide::Buy {
                            OrderSide::Sell
                        } else {
                            OrderSide::Buy
                        },
                        order_type: OrderType::TakeProfitMarket,
                        quantity: None,
                        price: None,
                        stop_price: Some(tp_price),
                        position_side: Some(PositionSide::Both),
                        reduce_only: None,
                        time_in_force: None,
                        close_position: Some(true),
                    };
                    match exchange.place_order(&tp_req).await {
                        Ok(_) => tracing::info!("🎯 止盈已设置: {}", tp_price),
                        Err(e) => tracing::error!("⚠️ 止盈设置失败: {}", e),
                    }
                }
            }

            // 刷新持仓并写入 state
            if let Ok(positions) = exchange.get_positions(None).await {
                let mut st = state.write().await;
                st.positions = positions;
            }

            Json(serde_json::json!({
                "success": true,
                "message": format!("操作成功: {:?}", order_side),
            }))
        }
        Err(e) => {
            tracing::error!("❌ 手动下单失败: {}", e);
            Json(serde_json::json!({
                "success": false,
                "error": format!("下单失败: {}", e),
            }))
        }
    }
}

/// 关闭指定的单笔追踪订单
pub async fn close_tracked_order(
    Extension(state): Extension<SharedEngineState>,
    Extension(exchange): Extension<Arc<dyn ExchangeApi>>,
    Json(req): Json<CloseOrderRequest>,
) -> Json<serde_json::Value> {
    tracing::info!("📝 关闭指定订单: {}", req.order_id);

    // 查找这笔订单
    let tracked = {
        let st = state.read().await;
        st.tracked_orders
            .iter()
            .find(|o| o.id == req.order_id && o.status == "open")
            .cloned()
    };

    let tracked = match tracked {
        Some(t) => t,
        None => {
            return Json(serde_json::json!({ "success": false, "error": "未找到该订单或已关闭" }))
        }
    };

    let side = if tracked.direction == "long" {
        OrderSide::Sell
    } else {
        OrderSide::Buy
    };

    let order_req = OrderRequest {
        symbol: tracked.symbol.clone(),
        side,
        order_type: OrderType::Market,
        quantity: Some(tracked.quantity),
        price: None,
        stop_price: None,
        position_side: Some(PositionSide::Both),
        reduce_only: Some(true),
        time_in_force: None,
        close_position: None,
    };

    match exchange.place_order(&order_req).await {
        Ok(_) => {
            let mut st = state.write().await;
            let last_price = st.last_price;

            if let Some(order) = st.tracked_orders.iter_mut().find(|o| o.id == req.order_id) {
                order.status = "closed".to_string();
                // 计算实现盈亏
                let is_long = order.direction == "long";
                let pnl = if is_long {
                    (last_price - order.entry_price) * order.quantity
                } else {
                    (order.entry_price - last_price) * order.quantity
                };
                order.closed_pnl = Some(pnl);

                // 更新策略统计
                let strategy_id = order.strategy_id.clone();
                let amount = order.amount_usdt;
                if let Some(strategy) = st.strategies.iter_mut().find(|s| s.id == strategy_id) {
                    strategy.total_pnl += pnl;
                    strategy.used_funds = (strategy.used_funds - amount).max(Decimal::ZERO);
                    if pnl >= Decimal::ZERO {
                        strategy.win_count += 1;
                    } else {
                        strategy.loss_count += 1;
                    }
                }
            }
            // 刷新持仓
            if let Ok(positions) = exchange.get_positions(None).await {
                st.positions = positions;
            }
            Json(serde_json::json!({ "success": true, "message": "订单已平仓" }))
        }
        Err(e) => {
            tracing::error!("❌ 关闭订单失败: {}", e);
            Json(serde_json::json!({ "success": false, "error": format!("平仓失败: {}", e) }))
        }
    }
}

/// 一键平掉所有持仓
pub async fn close_all_positions(
    Extension(state): Extension<SharedEngineState>,
    Extension(exchange): Extension<Arc<dyn ExchangeApi>>,
) -> Json<serde_json::Value> {
    tracing::info!("📝 收到一键平仓请求");

    let is_running = { state.read().await.is_running };

    if !is_running {
        return Json(serde_json::json!({
            "success": false,
            "error": "交易引擎未运行",
        }));
    }

    // 获取当前所有持仓
    let positions = match exchange.get_positions(None).await {
        Ok(pos) => pos,
        Err(e) => {
            return Json(
                serde_json::json!({ "success": false, "error": format!("获取持仓失败: {}", e) }),
            )
        }
    };

    let mut closed_count = 0;
    let mut errors = vec![];

    for p in positions {
        if p.quantity.is_zero() {
            continue;
        }

        // 取消所有挂单（止盈止损等）
        let _ = exchange.cancel_all_orders(&p.symbol).await;

        let is_long = p.position_side == PositionSide::Long
            || (p.position_side == PositionSide::Both && p.quantity > Decimal::ZERO);
        let side = if is_long {
            OrderSide::Sell
        } else {
            OrderSide::Buy
        };
        let abs_qty = p.quantity.abs();

        let order_req = OrderRequest {
            symbol: p.symbol.clone(),
            side,
            order_type: OrderType::Market,
            quantity: Some(abs_qty),
            price: None,
            stop_price: None,
            position_side: Some(PositionSide::Both),
            reduce_only: Some(true),
            time_in_force: None,
            close_position: None,
        };

        match exchange.place_order(&order_req).await {
            Ok(_) => {
                closed_count += 1;
                tracing::info!("✅ 一键平仓成功: {} {:?}", p.symbol, side);
            }
            Err(e) => {
                let err_msg = format!("{}平仓失败: {}", p.symbol, e);
                tracing::error!("❌ {}", err_msg);
                errors.push(err_msg);
            }
        }
    }

    // 标记所有 tracked_orders 为 closed，刷新持仓
    {
        let mut st = state.write().await;
        for order in st.tracked_orders.iter_mut() {
            if order.status == "open" {
                order.status = "closed".to_string();
            }
        }
        if let Ok(new_pos) = exchange.get_positions(None).await {
            st.positions = new_pos;
        }
    }

    if errors.is_empty() {
        Json(serde_json::json!({
            "success": true,
            "message": format!("已成功平掉 {} 个持仓", closed_count),
        }))
    } else {
        Json(serde_json::json!({
            "success": false,
            "error": format!("部分平仓失败: {:?}", errors),
        }))
    }
}
