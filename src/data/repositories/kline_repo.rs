//! K线数据仓库

use crate::config::db_config;
use crate::data::kline::Kline;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::Result;

/// K线仓库
pub struct KlineRepository;

impl KlineRepository {
    /// 批量插入K线数据
    pub async fn batch_insert(klines: &[Kline]) -> Result<u64> {
        if klines.is_empty() {
            return Ok(0);
        }

        let mut count = 0u64;
        for kline in klines {
            let result = sqlx::query(
                r#"
                INSERT INTO klines (code, date, open, high, low, close, volume, amount, change_pct, turnover)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                    open = VALUES(open),
                    high = VALUES(high),
                    low = VALUES(low),
                    close = VALUES(close),
                    volume = VALUES(volume),
                    amount = VALUES(amount),
                    change_pct = VALUES(change_pct),
                    turnover = VALUES(turnover)
                "#,
            )
            .bind(&kline.code)
            .bind(&kline.date)
            .bind(&kline.open)
            .bind(&kline.high)
            .bind(&kline.low)
            .bind(&kline.close)
            .bind(&kline.volume)
            .bind(&kline.amount)
            .bind(&kline.change_pct)
            .bind(&kline.turnover)
            .execute(db_config::get_global_pool())
            .await?;

            count += result.rows_affected();
        }

        Ok(count)
    }

    /// 查询指定股票的K线数据
    pub async fn find_by_code(
        code: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<Vec<Kline>> {
        let mut query = String::from("SELECT * FROM klines WHERE code = ?");

        if start_date.is_some() {
            query.push_str(" AND date >= ?");
        }
        if end_date.is_some() {
            query.push_str(" AND date <= ?");
        }
        query.push_str(" ORDER BY date ASC");

        let mut q = sqlx::query_as::<_, Kline>(&query).bind(code);

        if let Some(start) = start_date {
            q = q.bind(start);
        }
        if let Some(end) = end_date {
            q = q.bind(end);
        }

        let klines = q.fetch_all(db_config::get_global_pool()).await?;
        Ok(klines)
    }

    /// 获取最新一根K线
    pub async fn find_latest(code: &str) -> Result<Option<Kline>> {
        let kline = sqlx::query_as::<_, Kline>(
            "SELECT * FROM klines WHERE code = ? ORDER BY date DESC LIMIT 1",
        )
        .bind(code)
        .fetch_optional(db_config::get_global_pool())
        .await?;

        Ok(kline)
    }

    /// 获取指定日期的K线
    pub async fn find_by_date(code: &str, date: NaiveDate) -> Result<Option<Kline>> {
        let kline = sqlx::query_as::<_, Kline>(
            "SELECT * FROM klines WHERE code = ? AND date = ?",
        )
        .bind(code)
        .bind(date)
        .fetch_optional(db_config::get_global_pool())
        .await?;

        Ok(kline)
    }

    /// 统计K线数量
    pub async fn count_by_code(code: &str) -> Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM klines WHERE code = ?")
            .bind(code)
            .fetch_one(db_config::get_global_pool())
            .await?;

        Ok(count.0)
    }
}
