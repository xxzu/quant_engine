//! 策略处理器

use axum::Json;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 策略信息
#[derive(Debug, Serialize)]
pub struct StrategyInfo {
    pub name: String,
    pub description: String,
}

/// 回测请求
#[derive(Debug, Deserialize)]
pub struct BacktestRequest {
    pub strategy_name: String,
    pub codes: Vec<String>,
    pub start_date: String,
    pub end_date: String,
    pub initial_capital: Option<f64>,
}

/// 回测响应
#[derive(Debug, Serialize)]
pub struct BacktestResponse {
    pub strategy_name: String,
    pub initial_capital: f64,
    pub final_capital: f64,
    pub total_return: f64,
    pub annual_return: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub trade_count: u32,
    pub win_rate: f64,
}

/// 获取策略列表
pub async fn list_strategies() -> Json<Vec<StrategyInfo>> {
    // TODO: 返回已注册的策略列表
    Json(vec![
        StrategyInfo {
            name: "MA_Cross".to_string(),
            description: "双均线交叉策略".to_string(),
        },
        StrategyInfo {
            name: "MACD_Strategy".to_string(),
            description: "MACD 金叉死叉策略".to_string(),
        },
    ])
}

/// 运行回测
pub async fn run_backtest(
    Json(request): Json<BacktestRequest>,
) -> Json<BacktestResponse> {
    // TODO: 实现回测逻辑
    let initial = request.initial_capital.unwrap_or(100_000.0);

    Json(BacktestResponse {
        strategy_name: request.strategy_name,
        initial_capital: initial,
        final_capital: initial,
        total_return: 0.0,
        annual_return: 0.0,
        sharpe_ratio: 0.0,
        max_drawdown: 0.0,
        trade_count: 0,
        win_rate: 0.0,
    })
}
