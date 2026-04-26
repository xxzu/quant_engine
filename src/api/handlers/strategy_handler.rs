//! 策略管理处理器

use crate::engine::state::SharedEngineState;
use axum::{Extension, Json};
use rust_decimal::Decimal;

/// 获取所有策略的完整状态
pub async fn get_stages(Extension(state): Extension<SharedEngineState>) -> Json<serde_json::Value> {
    let st = state.read().await;
    let last_price = st.last_price;

    // 为每个策略收集其下的订单
    let strategies: Vec<serde_json::Value> = st
        .strategies
        .iter()
        .map(|s| {
            let orders: Vec<&_> = st
                .tracked_orders
                .iter()
                .filter(|o| o.strategy_id == s.id)
                .collect();

            let open_orders: Vec<serde_json::Value> = orders
                .iter()
                .filter(|o| o.status == "open")
                .map(|o| {
                    let qty = o.quantity;
                    let entry = o.entry_price;
                    let is_long = o.direction == "long";
                    let pnl = if is_long {
                        (last_price - entry) * qty
                    } else {
                        (entry - last_price) * qty
                    };
                    serde_json::json!({
                        "id": o.id,
                        "symbol": o.symbol,
                        "direction": o.direction,
                        "quantity": o.quantity,
                        "entry_price": o.entry_price,
                        "leverage": o.leverage,
                        "amount_usdt": o.amount_usdt,
                        "opened_at": o.opened_at,
                        "status": o.status,
                        "unrealized_pnl": pnl,
                    })
                })
                .collect();

            // 计算当前策略下所有 open 订单的未实现盈亏
            let unrealized_pnl: Decimal = orders
                .iter()
                .filter(|o| o.status == "open")
                .map(|o| {
                    let is_long = o.direction == "long";
                    if is_long {
                        (last_price - o.entry_price) * o.quantity
                    } else {
                        (o.entry_price - last_price) * o.quantity
                    }
                })
                .sum();

            serde_json::json!({
                "id": s.id,
                "name": s.name,
                "description": s.description,
                "active": s.active,
                "allocated_funds": s.allocated_funds,
                "used_funds": s.used_funds,
                "available_funds": s.available_funds(),
                "total_pnl": s.total_pnl,
                "unrealized_pnl": unrealized_pnl,
                "win_count": s.win_count,
                "loss_count": s.loss_count,
                "open_orders": open_orders,
            })
        })
        .collect();

    Json(serde_json::json!({
        "current_stage": st.strategy_stage,
        "balance": st.total_balance,
        "last_price": st.last_price,
        "strategies": strategies,
    }))
}

/// 更新策略启用状态
pub async fn update_stages(
    Extension(state): Extension<SharedEngineState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let stage_id = body["stage_id"].as_str().unwrap_or("").to_string();
    let active = body["active"].as_bool().unwrap_or(true);

    {
        let mut st = state.write().await;
        if let Some(s) = st.strategies.iter_mut().find(|s| s.id == stage_id) {
            s.active = active;
        }
    }

    tracing::info!(
        "🔄 策略更新: {} -> {}",
        stage_id,
        if active { "启用" } else { "禁用" }
    );

    Json(serde_json::json!({
        "success": true,
        "stage_id": stage_id,
        "active": active,
    }))
}

/// 给策略分配资金
pub async fn allocate_funds(
    Extension(state): Extension<SharedEngineState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let strategy_id = body["strategy_id"].as_str().unwrap_or("").to_string();
    let amount = body["amount"].as_f64().unwrap_or(0.0);
    let amount_dec = Decimal::from_f64_retain(amount).unwrap_or(Decimal::ZERO);

    if amount_dec < Decimal::ZERO {
        return Json(serde_json::json!({ "success": false, "error": "分配金额不能为负" }));
    }

    {
        let mut st = state.write().await;

        // 检查总分配不超过可用余额
        let total_allocated: Decimal = st
            .strategies
            .iter()
            .filter(|s| s.id != strategy_id)
            .map(|s| s.allocated_funds)
            .sum();

        if total_allocated + amount_dec
            > st.available_balance
                + st.strategies
                    .iter()
                    .find(|s| s.id == strategy_id)
                    .map(|s| s.allocated_funds)
                    .unwrap_or(Decimal::ZERO)
        {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("资金不足: 可用余额 {}U", st.available_balance)
            }));
        }

        if let Some(s) = st.strategies.iter_mut().find(|s| s.id == strategy_id) {
            s.allocated_funds = amount_dec;
            tracing::info!("💰 策略 {} 资金分配: {}U", strategy_id, amount_dec);
        } else {
            return Json(serde_json::json!({ "success": false, "error": "策略不存在" }));
        }
    }

    Json(serde_json::json!({
        "success": true,
        "message": format!("已分配 {}U 到策略 {}", amount_dec, strategy_id),
    }))
}
