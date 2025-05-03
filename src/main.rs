use quant_engine::config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化连接池
    let sys_config = config::sys_config::GLOBAL_CONFIG.lock().unwrap();
    // 初始化连接池
    config::db_config::init_global_pool(&sys_config.database_url).await?;

    // 启动web服务...
    Ok(())
}
