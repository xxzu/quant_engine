//! QuantEngine - 加密货币合约交易引擎

use quant_engine::api::router;
use quant_engine::config::sys_config::GLOBAL_CONFIG;
use quant_engine::engine::TradingEngine;
use quant_engine::exchange::binance::BinanceClient;
use quant_engine::strategy::strategies::discipline::DisciplineStrategy;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .init();

    tracing::info!("🚀 QuantEngine 合约交易引擎启动中...");

    // 加载配置
    let config = GLOBAL_CONFIG.lock().unwrap().clone();
    let port = config.server.port;

    tracing::info!("📋 交易对: {} | 杠杆: {}x | 模式: {} | 止损: {}% | 止盈: {}%",
        config.strategy.symbol,
        config.strategy.leverage,
        config.strategy.margin_mode,
        config.strategy.stop_loss_pct,
        config.strategy.take_profit_pct,
    );

    // 创建币安客户端
    let exchange = Arc::new(BinanceClient::from_config(&config.binance));

    // 创建策略
    let strategy = Arc::new(Mutex::new(
        DisciplineStrategy::from_app_config(&config.strategy)
    ));

    // 创建交易引擎
    let engine = Arc::new(TradingEngine::new(
        exchange.clone(),
        strategy.clone(),
        config.clone(),
    ));

    // 启动交易引擎（后台运行）
    if !config.binance.api_key.is_empty() {
        tracing::info!("🔑 检测到 API Key，启动交易引擎...");
        if let Err(e) = engine.start().await {
            tracing::error!("❌ 交易引擎启动失败: {}", e);
        }
    } else {
        tracing::warn!("⚠️ 未配置 API Key，仅启动 Web 服务（不交易）");
    }

    // 创建 Web 路由
    let app = router::create_router(engine.state.clone(), exchange.clone())
        .layer(TraceLayer::new_for_http());

    // 启动 HTTP 服务器
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🌐 Web 服务器: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
