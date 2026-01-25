//! API 路由配置

use axum::{
    Router,
    routing::{get, post},
};
use crate::api::handlers::{stock_handler, strategy_handler, health_handler};

/// 创建 API 路由
pub fn create_router() -> Router {
    Router::new()
        // 健康检查
        .route("/health", get(health_handler::health_check))
        // 股票相关
        .nest("/api/v1/stocks", stock_routes())
        // 策略相关
        .nest("/api/v1/strategies", strategy_routes())
}

/// 股票相关路由
fn stock_routes() -> Router {
    Router::new()
        .route("/", get(stock_handler::list_stocks))
        .route("/:code", get(stock_handler::get_stock))
        .route("/:code/klines", get(stock_handler::get_klines))
}

/// 策略相关路由
fn strategy_routes() -> Router {
    Router::new()
        .route("/", get(strategy_handler::list_strategies))
        .route("/backtest", post(strategy_handler::run_backtest))
}
