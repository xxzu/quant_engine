//! 系统配置加载与管理

use lazy_static::lazy_static;
use serde::Deserialize;
use std::sync::Mutex;

/// 顶层配置
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub binance: BinanceConfig,
    pub strategy: StrategyConfig,
    pub risk: RiskConfig,
    pub notify: NotifyConfig,
    pub log: LogConfig,
}

/// 服务器配置
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
}

/// 数据库配置
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

/// 币安配置
#[derive(Debug, Deserialize, Clone)]
pub struct BinanceConfig {
    pub api_key: String,
    pub secret_key: String,
    pub testnet: bool,
    pub base_url: String,
    pub ws_url: String,
}

/// 策略配置
#[derive(Debug, Deserialize, Clone)]
pub struct StrategyConfig {
    pub name: String,
    pub symbol: String,
    pub leverage: u32,
    pub margin_mode: String,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub position_ratio: f64,
    pub ema_short: usize,
    pub ema_long: usize,
    pub rsi_period: usize,
    pub rsi_overbought: f64,
    pub rsi_oversold: f64,
    pub kline_interval: String,
}

/// 风控配置
#[derive(Debug, Deserialize, Clone)]
pub struct RiskConfig {
    pub max_daily_loss: f64,
    pub max_concurrent_positions: u32,
    pub cooldown_minutes: u64,
    pub force_isolated_below: f64,
}

/// 通知配置
#[derive(Debug, Deserialize, Clone)]
pub struct NotifyConfig {
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
}

/// 日志配置
#[derive(Debug, Deserialize, Clone)]
pub struct LogConfig {
    pub level: String,
}

/// 加载配置文件
pub fn load_config(file_path: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(file_path)?;
    let config: AppConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

lazy_static! {
    pub static ref GLOBAL_CONFIG: Mutex<AppConfig> = {
        let config = load_config("config.yaml").expect("Failed to load config.yaml");
        Mutex::new(config)
    };
}
