//! 量化交易引擎主程序

use quant_engine::api::router;
use quant_engine::config;
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

    tracing::info!("🚀 QuantEngine 量化交易引擎启动中...");

    // 加载配置
    let sys_config = config::sys_config::GLOBAL_CONFIG.lock().unwrap();
    let port = sys_config.port;
    let db_url = sys_config.database_url.clone();
    drop(sys_config);

    // 初始化数据库连接池
    config::db_config::init_global_pool(&db_url).await?;
    tracing::info!("✅ 数据库连接池初始化成功");

    // 创建路由
    let app = router::create_router().layer(TraceLayer::new_for_http());

    // 启动服务器
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("🌐 服务器监听地址: http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
