use quant_engine::config;
use quant_engine::data::user_repository::User;

#[tokio::test]
pub async fn test_insert_user() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化连接池
    let sys_config = config::sys_config::GLOBAL_CONFIG.lock().unwrap();
    // 初始化连接池
    config::db_config::init_global_pool(&sys_config.database_url).await?;
    // 插入示例数据
    User::insert("Alice", 30).await?;
    let _bob_id = User::insert("Bob", 25).await?;

    // 查询所有用户
    let users = User::get_all().await?;
    println!("All users: {:#?}", users);
    Ok(())
}
