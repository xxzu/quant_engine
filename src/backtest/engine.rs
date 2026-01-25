//! 回测引擎核心

use crate::data::kline::{BarData, Kline};
use crate::strategy::signal::Signal;
use crate::strategy::strategy_trait::Strategy;
use crate::strategy::context::StrategyContext;
use crate::trading::broker::paper::PaperBroker;
use crate::trading::order::{Order, OrderSide};
use crate::backtest::performance::PerformanceMetrics;
use anyhow::Result;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 回测配置
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    /// 初始资金
    pub initial_capital: Decimal,
    /// 开始日期
    pub start_date: NaiveDate,
    /// 结束日期
    pub end_date: NaiveDate,
    /// 股票代码列表
    pub codes: Vec<String>,
    /// 是否打印日志
    pub verbose: bool,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            initial_capital: Decimal::from(100_000),
            start_date: NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            codes: Vec::new(),
            verbose: false,
        }
    }
}

/// 回测结果
#[derive(Debug, Clone)]
pub struct BacktestResult {
    /// 策略名称
    pub strategy_name: String,
    /// 初始资金
    pub initial_capital: Decimal,
    /// 最终资金
    pub final_capital: Decimal,
    /// 总收益率 (%)
    pub total_return: Decimal,
    /// 交易次数
    pub trade_count: u32,
    /// 胜率 (%)
    pub win_rate: Decimal,
    /// 绩效指标
    pub metrics: PerformanceMetrics,
}

/// 回测引擎
pub struct BacktestEngine {
    config: BacktestConfig,
    broker: PaperBroker,
}

impl BacktestEngine {
    /// 创建回测引擎
    pub fn new(config: BacktestConfig) -> Self {
        let broker = PaperBroker::new(config.initial_capital);
        Self { config, broker }
    }

    /// 运行回测
    pub async fn run<S: Strategy>(
        &mut self,
        strategy: Arc<Mutex<S>>,
        klines: Vec<Kline>,
    ) -> Result<BacktestResult> {
        let mut strategy = strategy.lock().await;
        
        // 初始化策略
        let context = StrategyContext::default();
        strategy.init(&context).await?;

        let strategy_name = strategy.name().to_string();
        let initial_capital = self.config.initial_capital;

        // 按日期分组 K 线
        let mut win_count = 0u32;
        let mut total_trades = 0u32;
        let mut equity_curve: Vec<Decimal> = Vec::new();

        for kline in klines {
            // 新交易日
            self.broker.new_trading_day();

            // 更新价格
            self.broker.update_price(&kline.code, kline.close);

            // 转换为 BarData
            let bar: BarData = kline.into();

            // 生成信号
            let signals = strategy.on_bar(&bar).await?;

            // 执行信号
            for signal in signals {
                let order = self.signal_to_order(&signal);
                if let Ok(trade) = self.broker.execute_order(order, bar.datetime) {
                    strategy.on_trade(&trade).await?;
                    total_trades += 1;
                    
                    // 简单胜率计算
                    if trade.side == OrderSide::Sell {
                        let profit = self.broker.get_account().available_cash;
                        if profit > initial_capital {
                            win_count += 1;
                        }
                    }
                }
            }

            // 记录权益曲线
            equity_curve.push(self.broker.get_account().total_asset);
        }

        // 停止策略
        strategy.on_stop().await?;

        // 计算结果
        let final_capital = self.broker.get_account().total_asset;
        let total_return = if !initial_capital.is_zero() {
            ((final_capital - initial_capital) / initial_capital) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let win_rate = if total_trades > 0 {
            Decimal::from(win_count) / Decimal::from(total_trades) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let metrics = PerformanceMetrics::calculate(&equity_curve, initial_capital);

        Ok(BacktestResult {
            strategy_name,
            initial_capital,
            final_capital,
            total_return,
            trade_count: total_trades,
            win_rate,
            metrics,
        })
    }

    /// 信号转订单
    fn signal_to_order(&self, signal: &Signal) -> Order {
        match signal.direction {
            crate::strategy::signal::SignalDirection::Buy => {
                if let Some(price) = signal.price {
                    Order::limit_buy(&signal.code, price, signal.quantity)
                } else {
                    Order::market_buy(&signal.code, signal.quantity)
                }
            }
            crate::strategy::signal::SignalDirection::Sell => {
                if let Some(price) = signal.price {
                    Order::limit_sell(&signal.code, price, signal.quantity)
                } else {
                    Order::market_sell(&signal.code, signal.quantity)
                }
            }
        }
    }
}
