//! 纪律型合约交易策略
//!
//! 核心规则:
//! - 10U 起步，每次用可用余额的 50% 开仓
//! - 100 倍杠杆，逐仓模式
//! - 止损 20%，止盈 100%
//! - 阶段升级: 80U 后固定 10U/单，200U 后可加大
//! - 入场信号: EMA 交叉 + RSI 过滤

use crate::exchange::types::*;
use crate::strategy::context::StrategyContext;
use crate::strategy::indicators::ma::ema;
use crate::strategy::indicators::rsi::rsi;
use crate::strategy::signal::{Signal, SignalDirection};
use crate::strategy::strategy_trait::Strategy;
use anyhow::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;
use std::collections::VecDeque;
use tracing::{info, warn};

/// 纪律策略配置
#[derive(Debug, Clone)]
pub struct DisciplineConfig {
    pub symbol: String,
    pub leverage: u32,
    pub margin_mode: MarginMode,
    pub stop_loss_pct: Decimal,
    pub take_profit_pct: Decimal,
    pub ema_short: usize,
    pub ema_long: usize,
    pub rsi_period: usize,
    pub rsi_overbought: Decimal,
    pub rsi_oversold: Decimal,
    pub position_ratio: Decimal,
}

impl Default for DisciplineConfig {
    fn default() -> Self {
        Self {
            symbol: "ETHUSDT".to_string(),
            leverage: 100,
            margin_mode: MarginMode::Isolated,
            stop_loss_pct: Decimal::from(20),
            take_profit_pct: Decimal::from(100),
            ema_short: 7,
            ema_long: 25,
            rsi_period: 14,
            rsi_overbought: Decimal::from(70),
            rsi_oversold: Decimal::from(30),
            position_ratio: Decimal::new(5, 1), // 0.5
        }
    }
}

/// 纪律型合约策略
pub struct DisciplineStrategy {
    config: DisciplineConfig,
    /// 历史收盘价 (用于计算指标)
    close_prices: VecDeque<Decimal>,
    /// 最大缓存 K 线数
    max_history: usize,
    /// 策略分配资金 (由引擎同步)
    allocated_funds: Decimal,
    /// 策略累计盈亏
    total_pnl: Decimal,
    /// 策略自己是否有持仓 (由引擎管理，只追踪策略开的仓)
    has_position: bool,
    /// 连续亏损次数 (由引擎在平仓时更新)
    #[allow(dead_code)]
    consecutive_losses: u32,
    /// 冷却时间 (毫秒时间戳)
    cooldown_until: i64,
    /// 指标预热是否完成
    is_warmed_up: bool,
    /// K线计数器 (调试用)
    kline_count: u64,
}

impl DisciplineStrategy {
    pub fn new(config: DisciplineConfig) -> Self {
        let max_history = config.ema_long.max(config.rsi_period) + 50;
        Self {
            config,
            close_prices: VecDeque::with_capacity(200),
            max_history,
            allocated_funds: Decimal::ZERO,
            total_pnl: Decimal::ZERO,
            has_position: false,
            consecutive_losses: 0,
            cooldown_until: 0,
            is_warmed_up: false,
            kline_count: 0,
        }
    }

    pub fn from_app_config(cfg: &crate::config::sys_config::StrategyConfig) -> Self {
        Self::new(DisciplineConfig {
            symbol: cfg.symbol.clone(),
            leverage: cfg.leverage,
            margin_mode: if cfg.margin_mode == "isolated" {
                MarginMode::Isolated
            } else {
                MarginMode::Cross
            },
            stop_loss_pct: Decimal::from_f64_retain(cfg.stop_loss_pct).unwrap_or(Decimal::from(20)),
            take_profit_pct: Decimal::from_f64_retain(cfg.take_profit_pct)
                .unwrap_or(Decimal::from(100)),
            ema_short: cfg.ema_short,
            ema_long: cfg.ema_long,
            rsi_period: cfg.rsi_period,
            rsi_overbought: Decimal::from_f64_retain(cfg.rsi_overbought)
                .unwrap_or(Decimal::from(70)),
            rsi_oversold: Decimal::from_f64_retain(cfg.rsi_oversold).unwrap_or(Decimal::from(30)),
            position_ratio: Decimal::from_f64_retain(cfg.position_ratio)
                .unwrap_or(Decimal::new(5, 1)),
        })
    }

    /// 根据策略分配资金计算每单金额
    fn calculate_order_amount(&self) -> Decimal {
        // 使用策略自身的分配资金 + 累计盈亏，而不是总账户余额
        let balance = self.allocated_funds + self.total_pnl;

        // 阶段规则
        if balance >= Decimal::from(200) {
            // 200U+ : 可以加大，每单 20U 或余额的 10%
            let ratio_amount = balance * Decimal::new(1, 1); // 10%
            ratio_amount.max(Decimal::from(20))
        } else if balance >= Decimal::from(80) {
            // 80-200U : 固定 10U/单
            Decimal::from(10)
        } else {
            // 10-80U : 余额的 50%
            balance * self.config.position_ratio
        }
    }

    /// 外部同步策略资金信息（由引擎在每次信号检查前调用）
    pub fn sync_funds(&mut self, allocated_funds: Decimal, total_pnl: Decimal) {
        self.allocated_funds = allocated_funds;
        self.total_pnl = total_pnl;
    }

    /// 标记预热完成
    pub fn mark_warmed_up(&mut self) {
        self.is_warmed_up = true;
        info!("✅ 纪律策略指标预热完成，开始监听实时信号");
    }

    /// 计算技术指标，生成入场方向
    fn check_entry_signal(&self) -> Option<SignalDirection> {
        let prices: Vec<Decimal> = self.close_prices.iter().copied().collect();
        if prices.len() < self.config.ema_long + 2 {
            return None;
        }

        // 计算 EMA
        let ema_short = ema(&prices, self.config.ema_short);
        let ema_long_vals = ema(&prices, self.config.ema_long);

        // 计算 RSI
        let rsi_vals = rsi(&prices, self.config.rsi_period);

        let len = prices.len();
        let curr_idx = len - 1;
        let prev_idx = len - 2;

        // 获取当前和前一个值
        let (curr_short, prev_short) = match (ema_short.get(curr_idx), ema_short.get(prev_idx)) {
            (Some(Some(c)), Some(Some(p))) => (*c, *p),
            _ => return None,
        };
        let (curr_long, prev_long) =
            match (ema_long_vals.get(curr_idx), ema_long_vals.get(prev_idx)) {
                (Some(Some(c)), Some(Some(p))) => (*c, *p),
                _ => return None,
            };
        let curr_rsi = match rsi_vals.get(curr_idx) {
            Some(Some(v)) => *v,
            _ => return None,
        };

        // EMA 金叉 + RSI 未超买 → 开多
        if prev_short <= prev_long
            && curr_short > curr_long
            && curr_rsi < self.config.rsi_overbought
        {
            info!(
                "📈 EMA 金叉! 短EMA({})={}, 长EMA({})={}, RSI={}",
                self.config.ema_short,
                curr_short.round_dp(2),
                self.config.ema_long,
                curr_long.round_dp(2),
                curr_rsi.round_dp(2)
            );
            return Some(SignalDirection::OpenLong);
        }

        // EMA 死叉 + RSI 未超卖 → 开空
        if prev_short >= prev_long && curr_short < curr_long && curr_rsi > self.config.rsi_oversold
        {
            info!(
                "📉 EMA 死叉! 短EMA({})={}, 长EMA({})={}, RSI={}",
                self.config.ema_short,
                curr_short.round_dp(2),
                self.config.ema_long,
                curr_long.round_dp(2),
                curr_rsi.round_dp(2)
            );
            return Some(SignalDirection::OpenShort);
        }

        None
    }
}

#[async_trait]
impl Strategy for DisciplineStrategy {
    fn name(&self) -> &str {
        "Discipline"
    }

    fn description(&self) -> &str {
        "纪律型合约策略 - EMA交叉+RSI过滤, 严格止损止盈"
    }

    async fn init(&mut self, _ctx: &StrategyContext) -> Result<()> {
        // has_position 由引擎管理，初始为 false（只追踪策略自己开的仓）
        self.has_position = false;
        info!(
            "🎯 纪律策略初始化: 杠杆={}x, 止损={}%, 止盈={}%",
            self.config.leverage,
            self.config.stop_loss_pct,
            self.config.take_profit_pct
        );
        Ok(())
    }

    async fn on_kline(&mut self, kline: &Kline) -> Result<Vec<Signal>> {
        // 只处理已完结的 K 线
        if !kline.is_closed {
            return Ok(vec![]);
        }

        // 更新价格历史
        self.close_prices.push_back(kline.close);
        if self.close_prices.len() > self.max_history {
            self.close_prices.pop_front();
        }

        // 预热阶段：只更新价格历史，不产生信号
        if !self.is_warmed_up {
            return Ok(vec![]);
        }

        self.kline_count += 1;

        // 每10根K线输出一次状态日志
        if self.kline_count % 10 == 1 {
            info!(
                "📊 策略状态: K线#{} 价格={} 资金={}U 持仓={} 冷却={}",
                self.kline_count,
                kline.close.round_dp(2),
                (self.allocated_funds + self.total_pnl).round_dp(2),
                self.has_position,
                self.cooldown_until > chrono::Utc::now().timestamp_millis()
            );
        }

        // 如果已有持仓，不开新仓（单持仓模式）
        if self.has_position {
            return Ok(vec![]);
        }

        // 检查冷却期
        let now = chrono::Utc::now().timestamp_millis();
        if now < self.cooldown_until {
            return Ok(vec![]);
        }

        // 检查余额是否充足
        let order_amount = self.calculate_order_amount();
        if order_amount < Decimal::from(5) {
            warn!(
                "⚠️ 余额不足: {}U, 最小开仓 5U",
                self.allocated_funds + self.total_pnl
            );
            return Ok(vec![]);
        }

        // 检查入场信号
        if let Some(direction) = self.check_entry_signal() {
            let signal = match direction {
                SignalDirection::OpenLong => Signal::open_long(
                    &self.config.symbol,
                    order_amount,
                    self.config.leverage,
                    self.name(),
                )
                .with_sl_tp(self.config.stop_loss_pct, self.config.take_profit_pct),
                SignalDirection::OpenShort => Signal::open_short(
                    &self.config.symbol,
                    order_amount,
                    self.config.leverage,
                    self.name(),
                )
                .with_sl_tp(self.config.stop_loss_pct, self.config.take_profit_pct),
                _ => return Ok(vec![]),
            };

            info!(
                "🚀 生成信号: {:?} {} 金额={}U 杠杆={}x",
                direction, self.config.symbol, order_amount, self.config.leverage
            );

            return Ok(vec![signal]);
        }

        Ok(vec![])
    }

    async fn on_position_update(&mut self, position: &FuturesPosition) -> Result<Vec<Signal>> {
        // 注意：has_position 现在由引擎根据策略自己的交易来管理
        // 这里只做日志记录，不管理 has_position
        if position.symbol == self.config.symbol {
            info!(
                "📊 持仓更新通知: {} 数量={} 未实现盈亏={}",
                position.symbol, position.quantity, position.unrealized_pnl
            );
        }
        Ok(vec![])
    }

    async fn on_order_update(&mut self, order: &OrderResponse) -> Result<()> {
        info!(
            "📋 订单更新: {} {:?} {:?} qty={}",
            order.symbol, order.side, order.status, order.executed_qty
        );
        Ok(())
    }

    async fn on_stop(&mut self) -> Result<()> {
        info!("⏹️ 纪律策略已停止");
        Ok(())
    }

    fn update_balance(&mut self, _balance: Decimal) {
        // 余额同步改由 sync_funds() 方法处理
    }

    fn mark_warmed_up(&mut self) {
        self.is_warmed_up = true;
        info!("✅ 纪律策略指标预热完成，开始监听实时信号");
    }

    fn sync_funds(&mut self, allocated_funds: Decimal, total_pnl: Decimal) {
        self.allocated_funds = allocated_funds;
        self.total_pnl = total_pnl;
    }

    fn set_has_position(&mut self, has: bool) {
        info!("🔄 策略持仓状态变更: {} -> {}", self.has_position, has);
        self.has_position = has;
    }
}
