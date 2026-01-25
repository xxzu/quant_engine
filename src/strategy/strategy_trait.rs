//! 策略接口定义

use crate::data::kline::BarData;
use crate::data::market_data::TickData;
use crate::strategy::signal::Signal;
use crate::trading::order::Trade;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// 策略上下文
pub struct StrategyContext {
    /// 初始资金
    pub initial_capital: f64,
    /// 当前持仓
    pub positions: HashMap<String, i64>,
    /// 策略参数
    pub params: HashMap<String, String>,
}

impl Default for StrategyContext {
    fn default() -> Self {
        Self {
            initial_capital: 100_000.0,
            positions: HashMap::new(),
            params: HashMap::new(),
        }
    }
}

/// 策略 Trait
#[async_trait]
pub trait Strategy: Send + Sync {
    /// 策略名称
    fn name(&self) -> &str;

    /// 策略描述
    fn description(&self) -> &str {
        ""
    }

    /// 初始化策略
    async fn init(&mut self, context: &StrategyContext) -> Result<()>;

    /// 处理 Bar 数据，生成交易信号
    async fn on_bar(&mut self, bar: &BarData) -> Result<Vec<Signal>>;

    /// 处理 Tick 数据 (可选)
    async fn on_tick(&mut self, _tick: &TickData) -> Result<Vec<Signal>> {
        Ok(vec![])
    }

    /// 订单成交回调
    async fn on_trade(&mut self, _trade: &Trade) -> Result<()> {
        Ok(())
    }

    /// 策略停止时调用
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }
}
