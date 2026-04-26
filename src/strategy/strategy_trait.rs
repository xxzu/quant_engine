//! 策略接口定义 - 合约交易版

use crate::exchange::types::*;
use crate::strategy::signal::Signal;
use anyhow::Result;
use async_trait::async_trait;

/// 策略上下文（由引擎注入）
pub use crate::strategy::context::StrategyContext;

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
    async fn init(&mut self, ctx: &StrategyContext) -> Result<()>;

    /// 处理 K 线数据，生成交易信号
    async fn on_kline(&mut self, kline: &Kline) -> Result<Vec<Signal>>;

    /// 持仓更新回调
    async fn on_position_update(&mut self, _position: &FuturesPosition) -> Result<Vec<Signal>> {
        Ok(vec![])
    }

    /// 订单更新回调
    async fn on_order_update(&mut self, _order: &OrderResponse) -> Result<()> {
        Ok(())
    }

    /// 策略停止
    async fn on_stop(&mut self) -> Result<()> {
        Ok(())
    }
}
