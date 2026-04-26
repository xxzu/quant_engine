use std::sync::Arc;
use axum::{Router, routing::{get, post}, Extension};
use tower_http::services::ServeDir;
use crate::api::handlers::{health_handler, market_handler, strategy_handler, trade_handler};
use crate::engine::state::SharedEngineState;
use crate::exchange::types::ExchangeApi;

/// 创建 API 路由
pub fn create_router(state: SharedEngineState, exchange: Arc<dyn ExchangeApi>) -> Router {
    Router::new()
        // 系统
        .route("/health", get(health_handler::health_check))
        .route("/api/v1/status", get(health_handler::engine_status))
        // 行情
        .route("/api/v1/symbols", get(market_handler::get_symbols))
        .route("/api/v1/markets", get(market_handler::get_market_prices))
        .route("/api/v1/klines/:symbol/:interval", get(market_handler::get_klines))
        // 策略
        .route("/api/v1/strategy/stages", get(strategy_handler::get_stages))
        .route("/api/v1/strategy/stages/update", post(strategy_handler::update_stages))
        .route("/api/v1/strategy/allocate", post(strategy_handler::allocate_funds))
        // 手动交易
        .route("/api/v1/trade/order", post(trade_handler::place_manual_order))
        .route("/api/v1/trade/close_all", post(trade_handler::close_all_positions))
        .route("/api/v1/trade/close_order", post(trade_handler::close_tracked_order))
        // 前端静态文件
        .fallback_service(ServeDir::new("frontend"))
        .layer(Extension(state))
        .layer(Extension(exchange))
}
