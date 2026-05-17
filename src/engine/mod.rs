//! 核心交易引擎 - 事件驱动

pub mod state;

use crate::config::sys_config::AppConfig;
use crate::exchange::types::*;
use crate::strategy::context::StrategyContext;
use crate::strategy::signal::{Signal, SignalDirection};
use crate::strategy::strategy_trait::Strategy;
use anyhow::Result;
use rust_decimal::Decimal;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

use state::{calc_strategy_stage, EngineState, SharedEngineState};

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
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let symbol = &self.config.strategy.symbol;
        let interval = &self.config.strategy.kline_interval;
        let leverage = self.config.strategy.leverage;

        info!("🚀 交易引擎启动: {} {}x {}", symbol, leverage, interval);

        // 1. 设置杠杆和保证金模式
        self.exchange.set_leverage(symbol, leverage).await?;
        self.exchange
            .set_margin_mode(symbol, MarginMode::Isolated)
            .await?;

        // 2. 获取初始状态
        let account = self.exchange.get_account().await?;
        let positions = self.exchange.get_positions(Some(symbol)).await?;
        let contract = self.exchange.get_contract_info(symbol).await?;

        info!(
            "💰 账户余额: {}U, 可用: {}U",
            account.total_balance, account.available_balance
        );

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

        // 3.5 拉取当前持仓 (启动时同步)
        let positions = self.exchange.get_positions(Some(symbol)).await?;
        let active_positions: Vec<_> = positions
            .into_iter()
            .filter(|p| !p.quantity.is_zero())
            .collect();
        if !active_positions.is_empty() {
            info!("📊 检测到 {} 个活跃持仓", active_positions.len());
            for p in &active_positions {
                info!(
                    "  └ {} qty={} 开仓价={} 保证金={}U",
                    p.symbol, p.quantity, p.entry_price, p.margin
                );
            }
        }
        {
            let mut st = self.state.write().await;
            st.positions = active_positions;
        }

        // 4. 加载历史 K 线 (预热指标)
        info!("📊 加载历史K线预热指标...");
        let history = self.exchange.get_klines(symbol, interval, 100).await?;
        for kline in &history {
            let _ = self.strategy.lock().await.on_kline(kline).await;
        }
        // 预热完成，标记策略可以开始产生信号
        self.strategy.lock().await.mark_warmed_up();
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

                        // ========== 引擎级强制止损/止盈监控 ==========
                        // 每次价格更新都检查（包括未收盘K线），不依赖交易所挂单
                        {
                            let st = state.read().await;
                            let positions_to_check: Vec<_> = st.positions.iter()
                                .filter(|p| !p.quantity.is_zero())
                                .cloned()
                                .collect();
                            drop(st); // 释放读锁

                            let current_price = kline.close;
                            for pos in &positions_to_check {
                                let margin = pos.margin;
                                if margin.is_zero() || pos.entry_price.is_zero() {
                                    continue;
                                }
                                // 用实时价格重新计算未实现盈亏
                                let realtime_pnl = if pos.quantity > Decimal::ZERO {
                                    // 多仓: (当前价 - 开仓价) * 数量
                                    (current_price - pos.entry_price) * pos.quantity.abs()
                                } else {
                                    // 空仓: (开仓价 - 当前价) * 数量
                                    (pos.entry_price - current_price) * pos.quantity.abs()
                                };
                                let pnl_pct = realtime_pnl / margin * Decimal::from(100);

                                // 止损: 亏损超过 -20%
                                if pnl_pct <= Decimal::from(-20) {
                                    error!(
                                        "🚨 引擎强制止损触发! {} 浮亏={:.2}% ({}U) 保证金={}U",
                                        pos.symbol, pnl_pct, pos.unrealized_pnl, margin
                                    );
                                    let close_side = if pos.quantity > Decimal::ZERO {
                                        OrderSide::Sell
                                    } else {
                                        OrderSide::Buy
                                    };
                                    // 先撤掉所有挂单
                                    let _ = exchange.cancel_all_orders(&pos.symbol).await;
                                    let close_req = OrderRequest {
                                        symbol: pos.symbol.clone(),
                                        side: close_side,
                                        order_type: OrderType::Market,
                                        quantity: Some(pos.quantity.abs()),
                                        price: None,
                                        stop_price: None,
                                        position_side: Some(PositionSide::Both),
                                        reduce_only: Some(true),
                                        time_in_force: None,
                                        close_position: None,
                                    };
                                    match exchange.place_order(&close_req).await {
                                        Ok(_) => {
                                            error!("🛑 强制止损平仓成功: {} qty={}", pos.symbol, pos.quantity.abs());
                                            strategy.lock().await.set_has_position(false);
                                        }
                                        Err(e) => error!("❌ 强制止损平仓失败: {}", e),
                                    }
                                }

                                // 止盈: 盈利超过 +100%
                                if pnl_pct >= Decimal::from(100) {
                                    info!(
                                        "🎉 引擎强制止盈触发! {} 浮盈={:.2}% ({}U)",
                                        pos.symbol, pnl_pct, pos.unrealized_pnl
                                    );
                                    let close_side = if pos.quantity > Decimal::ZERO {
                                        OrderSide::Sell
                                    } else {
                                        OrderSide::Buy
                                    };
                                    let _ = exchange.cancel_all_orders(&pos.symbol).await;
                                    let close_req = OrderRequest {
                                        symbol: pos.symbol.clone(),
                                        side: close_side,
                                        order_type: OrderType::Market,
                                        quantity: Some(pos.quantity.abs()),
                                        price: None,
                                        stop_price: None,
                                        position_side: Some(PositionSide::Both),
                                        reduce_only: Some(true),
                                        time_in_force: None,
                                        close_position: None,
                                    };
                                    match exchange.place_order(&close_req).await {
                                        Ok(_) => {
                                            info!("🎯 强制止盈平仓成功: {} qty={}", pos.symbol, pos.quantity.abs());
                                            strategy.lock().await.set_has_position(false);
                                        }
                                        Err(e) => error!("❌ 强制止盈平仓失败: {}", e),
                                    }
                                }
                            }
                        }
                        // ========== 强制监控结束 ==========

                        // 检查策略是否在 EngineState 中被启用且有资金
                        let (strategy_active, alloc_funds, total_pnl) = {
                            let st = state.read().await;
                            let info = st.strategies.iter().find(|s| s.id == "discipline");
                            match info {
                                Some(s) => (s.active && s.allocated_funds > Decimal::ZERO, s.allocated_funds, s.total_pnl),
                                None => (false, Decimal::ZERO, Decimal::ZERO),
                            }
                        };

                        let mut strat = strategy.lock().await;
                        // 同步策略资金信息
                        strat.sync_funds(alloc_funds, total_pnl);
                        match strat.on_kline(&kline).await {
                            Ok(signals) => {
                                if !strategy_active {
                                    // 策略未启用，跳过信号执行
                                    if !signals.is_empty() {
                                        info!("📋 纪律策略产生 {} 个信号，但策略未启用，跳过执行", signals.len());
                                    }
                                    continue;
                                }
                                for signal in signals {
                                    info!("🔥 策略信号触发! {:?} {} 金额={}U", signal.direction, signal.symbol, signal.amount_usdt);

                                    if let Err(e) = Self::execute_signal(
                                        &exchange, &signal, &contract_clone,
                                    ).await {
                                        error!("❌ 执行信号失败: {}", e);
                                    } else {
                                        // 执行成功后标记策略持仓状态
                                        match signal.direction {
                                            SignalDirection::OpenLong | SignalDirection::OpenShort => {
                                                strat.set_has_position(true);
                                                // 更新策略的 used_funds
                                                let mut st = state.write().await;
                                                if let Some(s) = st.strategies.iter_mut().find(|s| s.id == "discipline") {
                                                    s.used_funds += signal.amount_usdt;
                                                }
                                            }
                                            SignalDirection::CloseLong | SignalDirection::CloseShort => {
                                                strat.set_has_position(false);
                                            }
                                        }
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

                                // 如果是止损/止盈单被触发 (FILLED)，更新 tracked_orders 状态
                                if order.status == OrderStatus::Filled {
                                    let is_sl_tp = matches!(order.order_type,
                                        OrderType::StopMarket | OrderType::TakeProfitMarket |
                                        OrderType::Stop | OrderType::TakeProfit
                                    );
                                    if is_sl_tp {
                                        info!("🎯 止损/止盈单触发: {} {:?} {:?}", order.symbol, order.order_type, order.side);
                                        // 重置策略持仓状态
                                        strat.set_has_position(false);
                                        let mut st = state.write().await;
                                        // 先收集需要释放的资金信息
                                        let mut to_release: Vec<(String, Decimal)> = Vec::new();
                                        for tracked in st.tracked_orders.iter_mut() {
                                            if tracked.symbol == order.symbol && tracked.status == "open" {
                                                tracked.status = "closed".to_string();
                                                to_release.push((tracked.strategy_id.clone(), tracked.amount_usdt));
                                            }
                                        }
                                        // 再释放策略资金
                                        for (sid, amount) in to_release {
                                            if let Some(s) = st.strategies.iter_mut().find(|s| s.id == sid) {
                                                s.used_funds = (s.used_funds - amount).max(Decimal::ZERO);
                                            }
                                        }
                                    }
                                }
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
                                // 同步更新策略的余额（让 calculate_order_amount 使用最新余额）
                                strat.update_balance(account.available_balance);
                                info!("💰 账户更新: 余额={}U, 可用={}U", account.total_balance, account.available_balance);
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
        info!(
            "⚡ 执行信号: {:?} {} 金额={}U",
            signal.direction, signal.symbol, signal.amount_usdt
        );

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
                info!(
                    "✅ 开多成功: {} qty={} price={}",
                    signal.symbol, quantity, resp.avg_price
                );

                // 止损/止盈由引擎级强制监控处理 (币安测试网不支持条件单)
                if signal.stop_loss_pct.is_some() || signal.take_profit_pct.is_some() {
                    info!(
                        "🛡️ 止损{:?}%/止盈{:?}% 由引擎实时监控",
                        signal.stop_loss_pct, signal.take_profit_pct
                    );
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
                info!(
                    "✅ 开空成功: {} qty={} price={}",
                    signal.symbol, quantity, resp.avg_price
                );

                // 止损/止盈由引擎级强制监控处理
                if signal.stop_loss_pct.is_some() || signal.take_profit_pct.is_some() {
                    info!(
                        "🛡️ 止损{:?}%/止盈{:?}% 由引擎实时监控",
                        signal.stop_loss_pct, signal.take_profit_pct
                    );
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
                        quantity: Some(p.quantity.abs()),
                        price: None,
                        stop_price: None,
                        position_side: Some(PositionSide::Both),
                        reduce_only: Some(true),
                        time_in_force: None,
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
                        quantity: Some(p.quantity.abs()),
                        price: None,
                        stop_price: None,
                        position_side: Some(PositionSide::Both),
                        reduce_only: Some(true),
                        time_in_force: None,
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
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.strategy.lock().await.on_stop().await?;
        info!("⏹️ 交易引擎已停止");
        Ok(())
    }

    /// 是否运行中
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }
}
