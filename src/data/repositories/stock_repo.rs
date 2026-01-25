//! 股票数据仓库

use crate::config::db_config;
use crate::data::stock::{CreateStockRequest, Stock};
use sqlx::Result;

/// 股票仓库
pub struct StockRepository;

impl StockRepository {
    /// 插入股票
    pub async fn insert(req: &CreateStockRequest) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO stocks (code, name, market, industry, list_date, status)
            VALUES (?, ?, ?, ?, ?, 'Normal')
            "#,
        )
        .bind(&req.code)
        .bind(&req.name)
        .bind(&req.market)
        .bind(&req.industry)
        .bind(&req.list_date)
        .execute(db_config::get_global_pool())
        .await?;

        Ok(result.last_insert_id() as i64)
    }

    /// 根据代码查询股票
    pub async fn find_by_code(code: &str) -> Result<Option<Stock>> {
        let stock = sqlx::query_as::<_, Stock>(
            "SELECT * FROM stocks WHERE code = ?",
        )
        .bind(code)
        .fetch_optional(db_config::get_global_pool())
        .await?;

        Ok(stock)
    }

    /// 查询所有股票
    pub async fn find_all() -> Result<Vec<Stock>> {
        let stocks = sqlx::query_as::<_, Stock>("SELECT * FROM stocks")
            .fetch_all(db_config::get_global_pool())
            .await?;

        Ok(stocks)
    }

    /// 根据市场查询股票
    pub async fn find_by_market(market: &str) -> Result<Vec<Stock>> {
        let stocks = sqlx::query_as::<_, Stock>(
            "SELECT * FROM stocks WHERE market = ?",
        )
        .bind(market)
        .fetch_all(db_config::get_global_pool())
        .await?;

        Ok(stocks)
    }

    /// 更新股票状态
    pub async fn update_status(code: &str, status: &str) -> Result<()> {
        sqlx::query("UPDATE stocks SET status = ?, updated_at = NOW() WHERE code = ?")
            .bind(status)
            .bind(code)
            .execute(db_config::get_global_pool())
            .await?;

        Ok(())
    }

    /// 删除股票
    pub async fn delete(code: &str) -> Result<()> {
        sqlx::query("DELETE FROM stocks WHERE code = ?")
            .bind(code)
            .execute(db_config::get_global_pool())
            .await?;

        Ok(())
    }
}
