use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;

static POOL: OnceCell<MySqlPool> = OnceCell::new();
static INIT_LOCK: Mutex<()> = Mutex::const_new(());

pub async fn init_global_pool(db_url: &str) -> Result<(), sqlx::Error> {
    let _guard = INIT_LOCK.lock().await;
    if POOL.get().is_none() {
        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .connect(db_url)
            .await?;
        POOL.set(pool).unwrap();
    }
    Ok(())
}

pub fn get_global_pool() -> &'static MySqlPool {
    POOL.get().expect("Database pool not initialized")
}