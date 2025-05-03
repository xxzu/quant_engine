use super::super::config::db_config;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub age: u8,
}

impl User {
    // 插入用户（返回自增ID）
    pub async fn insert(name: &str, age: u8) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("INSERT INTO users (name, age) VALUES (?, ?)")
            .bind(name)
            .bind(age)
            .execute(db_config::get_global_pool())
            .await?;
        Ok(result.last_insert_id())
    }

    // 查询所有用户（编译时SQL检查）
    pub async fn get_all() -> Result<Vec<User>, sqlx::Error> {
        let users = sqlx::query_as::<_, User>("SELECT id, name, age FROM users")
            .fetch_all(db_config::get_global_pool())
            .await?;
        Ok(users)
    }

    // 根据ID查询用户
    pub async fn get_by_id(id: u64) -> Result<Option<User>, sqlx::Error> {
        let mut users = sqlx::query_as::<_, User>("SELECT id, name, age FROM users WHERE id = ?")
            .bind(id)
            .fetch_all(db_config::get_global_pool())
            .await?;
        if users.is_empty() {
            return Ok(None);
        }
        Ok(users.pop())
    }

    // 更新用户信息
    pub async fn update(id: u64, new_name: &str, new_age: u8) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET name = ?, age = ? WHERE id = ?")
            .bind(new_name)
            .bind(new_age)
            .bind(id)
            .execute(db_config::get_global_pool())
            .await?;
        Ok(())
    }

    // 删除用户
    pub async fn delete(id: u64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(db_config::get_global_pool())
            .await?;
        Ok(())
    }
}
