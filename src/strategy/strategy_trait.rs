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

    /// 更新策略使用的余额（由引擎在账户变更时调用）
    fn update_balance(&mut self, _balance: rust_decimal::Decimal) {}

    /// 标记指标预热完成（历史K线加载完毕后调用）
    fn mark_warmed_up(&mut self) {}

    /// 同步策略分配资金和累计盈亏
    fn sync_funds(
        &mut self,
        _allocated_funds: rust_decimal::Decimal,
        _total_pnl: rust_decimal::Decimal,
    ) {
    }

    /// 设置策略的持仓状态（由引擎在策略交易成功后调用）
    fn set_has_position(&mut self, _has: bool) {}
}
