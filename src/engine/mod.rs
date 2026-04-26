//! 核心交易引擎 - 事件驱动

pub mod state;

use crate::config::sys_config::AppConfig;
use crate::exchange::types::*;
use crate::strategy::signal::{Signal, SignalDirection};
use crate::strategy::strategy_trait::Strategy;
use crate::strategy::context::StrategyContext;
use anyhow::Result;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, error, warn};

use state::{SharedEngineState, EngineState, calc_strategy_stage};

/// 交易引擎
pub struct TradingEngine {
    exchange: Arc<dyn ExchangeApi>,
    strategy: Arc<Mutex<dyn Strategy>>,
    config: AppConfig,
    running: Arc<std::sync::atomic::AtomicBool>,
    pub state: SharedEngineState,
}

impl TradingEngine {
    pub fn new(
        exchange: Arc<dyn ExchangeApi>,
        strategy: Arc<Mutex<dyn Strategy>>,
        config: AppConfig,
    ) -> Self {
        Self {
            exchange,
            strategy,
            config,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            state: Arc::new(RwLock::new(EngineState::default())),
        }
    }

    /// 启动引擎
    pub async fn start(&self) -> Result<()> {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);

        let symbol = &self.config.strategy.symbol;
        let interval = &self.config.strategy.kline_interval;
        let leverage = self.config.strategy.leverage;

        info!("🚀 交易引擎启动: {} {}x {}", symbol, leverage, interval);

        // 1. 设置杠杆和保证金模式
        self.exchange.set_leverage(symbol, leverage).await?;
        self.exchange.set_margin_mode(symbol, MarginMode::Isolated).await?;

        // 2. 获取初始状态
        let account = self.exchange.get_account().await?;
        let positions = self.exchange.get_positions(Some(symbol)).await?;
        let contract = self.exchange.get_contract_info(symbol).await?;

        info!("💰 账户余额: {}U, 可用: {}U", account.total_balance, account.available_balance);

        // 3. 初始化策略
        let ctx = StrategyContext {
            available_balance: account.available_balance,
            total_balance: account.total_balance,
            positions: positions.clone(),
            contract_info: Some(contract.clone()),
        };
        self.strategy.lock().await.init(&ctx).await?;

        // 初始化状态
        {
            let mut st = self.state.write().await;
            st.symbol = symbol.clone();
            st.total_balance = account.total_balance;
            st.available_balance = account.available_balance;
            st.positions = positions.clone();
            st.strategy_stage = calc_strategy_stage(account.total_balance);
            st.is_running = true;
        }

        // 4. 加载历史 K 线 (预热指标)
        info!("📊 加载历史K线预热指标...");
        let history = self.exchange.get_klines(symbol, interval, 100).await?;
        for kline in &history {
            let _ = self.strategy.lock().await.on_kline(kline).await;
        }
        info!("✅ 已加载 {} 根历史K线", history.len());

        // 5. 订阅 WebSocket
        let mut kline_rx = self.exchange.subscribe_kline(symbol, interval).await?;
        let mut user_rx = self.exchange.subscribe_user_data().await?;

        info!("🔌 WebSocket 已连接，开始监听行情...");

        // 6. 事件循环
        let exchange = self.exchange.clone();
        let strategy = self.strategy.clone();
        let _config = self.config.clone();
        let running = self.running.clone();
        let state = self.state.clone();
        let contract_clone = contract.clone();

        tokio::spawn(async move {
            loop {
                if !running.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    // K线事件
                    Ok(kline) = kline_rx.recv() => {
                        // 更新最新价格
                        {
                            let mut st = state.write().await;
                            st.last_price = kline.close;
                        }
                        
                        let mut strat = strategy.lock().await;
                        match strat.on_kline(&kline).await {
                            Ok(signals) => {
                                for signal in signals {
                                    if let Err(e) = Self::execute_signal(
                                        &exchange, &signal, &contract_clone,
                                    ).await {
                                        error!("❌ 执行信号失败: {}", e);
                                    }
                                }
                            }
                            Err(e) => error!("策略处理 K 线失败: {}", e),
                        }
                    }

                    // 用户数据事件
                    Ok(event) = user_rx.recv() => {
                        let mut strat = strategy.lock().await;
                        match event {
                            UserDataEvent::OrderUpdate(order) => {
                                let _ = strat.on_order_update(&order).await;
                            }
                            UserDataEvent::PositionUpdate(positions) => {
                                // 更新状态中的持仓
                                {
                                    let mut st = state.write().await;
                                    st.positions = positions.clone();
                                    st.unrealized_pnl = positions.iter().map(|p| p.unrealized_pnl).sum();
                                }
                                for pos in &positions {
                                    let _ = strat.on_position_update(pos).await;
                                }
                            }
                            UserDataEvent::AccountUpdate(account) => {
                                {
                                    let mut st = state.write().await;
                                    st.total_balance = account.total_balance;
                                    st.available_balance = account.available_balance;
                                    st.strategy_stage = calc_strategy_stage(account.total_balance);
                                }
                                info!("💰 账户更新: 余额={}U", account.total_balance);
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// 执行交易信号
    async fn execute_signal(
        exchange: &Arc<dyn ExchangeApi>,
        signal: &Signal,
        contract: &ContractInfo,
    ) -> Result<()> {
        info!("⚡ 执行信号: {:?} {} 金额={}U",
            signal.direction, signal.symbol, signal.amount_usdt);

        match signal.direction {
            SignalDirection::OpenLong => {
                // 获取当前价格
                let ticker = exchange.get_ticker(&signal.symbol).await?;
                let price = ticker.last_price;

                // 计算数量: 金额 * 杠杆 / 价格
                let notional = signal.amount_usdt * Decimal::from(signal.leverage);
                let quantity = (notional / price).round_dp(contract.quantity_precision as u32);

                // 市价开多
                let order_req = OrderRequest {
                    symbol: signal.symbol.clone(),
                    side: OrderSide::Buy,
                    order_type: OrderType::Market,
                    quantity: Some(quantity),
                    price: None,
                    stop_price: None,
                    position_side: Some(PositionSide::Both),
                    reduce_only: None,
                    time_in_force: None,
                    close_position: None,
                };
                let resp = exchange.place_order(&order_req).await?;
                info!("✅ 开多成功: {} qty={} price={}", signal.symbol, quantity, resp.avg_price);

                // 设置止损单
                if let Some(sl_pct) = &signal.stop_loss_pct {
                    let sl_price = price * (Decimal::ONE - *sl_pct / Decimal::from(100));
                    let sl_price = sl_price.round_dp(contract.price_precision as u32);
                    let sl_req = OrderRequest {
                        symbol: signal.symbol.clone(),
                        side: OrderSide::Sell,
                        order_type: OrderType::StopMarket,
                        quantity: None,
                        price: None,
                        stop_price: Some(sl_price),
                        position_side: Some(PositionSide::Both),
                        reduce_only: None,
                        time_in_force: None,
                        close_position: Some(true),
                    };
                    exchange.place_order(&sl_req).await?;
                    info!("🛑 止损已设置: {}", sl_price);
                }

                // 设置止盈单
                if let Some(tp_pct) = &signal.take_profit_pct {
                    let tp_price = price * (Decimal::ONE + *tp_pct / Decimal::from(100));
                    let tp_price = tp_price.round_dp(contract.price_precision as u32);
                    let tp_req = OrderRequest {
                        symbol: signal.symbol.clone(),
                        side: OrderSide::Sell,
                        order_type: OrderType::TakeProfitMarket,
                        quantity: None,
                        price: None,
                        stop_price: Some(tp_price),
                        position_side: Some(PositionSide::Both),
                        reduce_only: None,
                        time_in_force: None,
                        close_position: Some(true),
                    };
                    exchange.place_order(&tp_req).await?;
                    info!("🎯 止盈已设置: {}", tp_price);
                }
            }

            SignalDirection::OpenShort => {
                let ticker = exchange.get_ticker(&signal.symbol).await?;
                let price = ticker.last_price;
                let notional = signal.amount_usdt * Decimal::from(signal.leverage);
                let quantity = (notional / price).round_dp(contract.quantity_precision as u32);

                let order_req = OrderRequest {
                    symbol: signal.symbol.clone(),
                    side: OrderSide::Sell,
                    order_type: OrderType::Market,
                    quantity: Some(quantity),
                    price: None,
                    stop_price: None,
                    position_side: Some(PositionSide::Both),
                    reduce_only: None,
                    time_in_force: None,
                    close_position: None,
                };
                let resp = exchange.place_order(&order_req).await?;
                info!("✅ 开空成功: {} qty={} price={}", signal.symbol, quantity, resp.avg_price);

                // 止损 (做空止损在上方)
                if let Some(sl_pct) = &signal.stop_loss_pct {
                    let sl_price = price * (Decimal::ONE + *sl_pct / Decimal::from(100));
                    let sl_price = sl_price.round_dp(contract.price_precision as u32);
                    let sl_req = OrderRequest {
                        symbol: signal.symbol.clone(),
                        side: OrderSide::Buy,
                        order_type: OrderType::StopMarket,
                        quantity: None, price: None,
                        stop_price: Some(sl_price),
                        position_side: Some(PositionSide::Both),
                        reduce_only: None, time_in_force: None,
                        close_position: Some(true),
                    };
                    exchange.place_order(&sl_req).await?;
                    info!("🛑 空单止损已设置: {}", sl_price);
                }

                // 止盈 (做空止盈在下方)
                if let Some(tp_pct) = &signal.take_profit_pct {
                    let tp_price = price * (Decimal::ONE - *tp_pct / Decimal::from(100));
                    let tp_price = tp_price.round_dp(contract.price_precision as u32);
                    let tp_req = OrderRequest {
                        symbol: signal.symbol.clone(),
                        side: OrderSide::Buy,
                        order_type: OrderType::TakeProfitMarket,
                        quantity: None, price: None,
                        stop_price: Some(tp_price),
                        position_side: Some(PositionSide::Both),
                        reduce_only: None, time_in_force: None,
                        close_position: Some(true),
                    };
                    exchange.place_order(&tp_req).await?;
                    info!("🎯 止盈已设置: {}", tp_price);
                }
            }

            SignalDirection::CloseLong => {
                exchange.cancel_all_orders(&signal.symbol).await?;
                // 获取当前持仓数量
                let positions = exchange.get_positions(Some(&signal.symbol)).await?;
                let pos = positions.iter().find(|p| !p.quantity.is_zero());
                if let Some(p) = pos {
                    let req = OrderRequest {
                        symbol: signal.symbol.clone(),
                        side: OrderSide::Sell,
                        order_type: OrderType::Market,
                        quantity: Some(p.quantity.abs()), price: None, stop_price: None,
                        position_side: Some(PositionSide::Both),
                        reduce_only: Some(true), time_in_force: None,
                        close_position: None,
                    };
                    exchange.place_order(&req).await?;
                    info!("✅ 平多完成: {}", signal.symbol);
                }
            }
            SignalDirection::CloseShort => {
                exchange.cancel_all_orders(&signal.symbol).await?;
                let positions = exchange.get_positions(Some(&signal.symbol)).await?;
                let pos = positions.iter().find(|p| !p.quantity.is_zero());
                if let Some(p) = pos {
                    let req = OrderRequest {
                        symbol: signal.symbol.clone(),
                        side: OrderSide::Buy,
                        order_type: OrderType::Market,
                        quantity: Some(p.quantity.abs()), price: None, stop_price: None,
                        position_side: Some(PositionSide::Both),
                        reduce_only: Some(true), time_in_force: None,
                        close_position: None,
                    };
                    exchange.place_order(&req).await?;
                    info!("✅ 平空完成: {}", signal.symbol);
                }
            }
        }

        Ok(())
    }

    /// 停止引擎
    pub async fn stop(&self) -> Result<()> {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        self.strategy.lock().await.on_stop().await?;
        info!("⏹️ 交易引擎已停止");
        Ok(())
    }

    /// 是否运行中
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }
}
