// ==================== QuantEngine Frontend App ====================

document.addEventListener('DOMContentLoaded', () => {
    // ==================== Navigation ====================
    const navItems = document.querySelectorAll('.nav-links li');
    const pages = document.querySelectorAll('.page');
    const pageHeading = document.getElementById('page-heading');
    const pageSubtitle = document.getElementById('page-subtitle');

    const pageMeta = {
        dashboard: { title: '仪表盘', subtitle: '系统概览与实时状态' },
        markets:   { title: '行情总览', subtitle: '合约 & 现货实时价格' },
        chart:     { title: 'K线图表', subtitle: '技术分析与走势研判' },
        strategy:  { title: '策略管理', subtitle: '交易阶段与参数配置' },
        positions: { title: '持仓订单', subtitle: '当前持仓与历史订单' },
    };

    function navigateTo(pageId) {
        // Update nav
        navItems.forEach(li => li.classList.toggle('active', li.dataset.page === pageId));
        // Update pages
        pages.forEach(p => p.classList.toggle('active', p.id === `page-${pageId}`));
        // Update header
        const meta = pageMeta[pageId] || {};
        pageHeading.textContent = meta.title || '';
        pageSubtitle.textContent = meta.subtitle || '';
        // Trigger page-specific load
        if (pageId === 'markets') loadMarkets();
        if (pageId === 'chart') loadChart();
        if (pageId === 'strategy') loadStages();
    }

    navItems.forEach(li => {
        li.addEventListener('click', (e) => {
            e.preventDefault();
            navigateTo(li.dataset.page);
        });
    });

    // Handle hash navigation
    function handleHash() {
        const hash = window.location.hash.replace('#', '') || 'dashboard';
        navigateTo(hash);
    }
    window.addEventListener('hashchange', handleHash);

    // ==================== Utility ====================
    function fmt(num, decimals = 2) {
        if (num === undefined || num === null || num === '--') return '--';
        const n = parseFloat(num);
        if (isNaN(n)) return '--';
        return n.toLocaleString('en-US', { minimumFractionDigits: decimals, maximumFractionDigits: decimals });
    }

    function fmtCompact(num) {
        const n = parseFloat(num);
        if (isNaN(n)) return '--';
        if (n >= 1e9) return (n/1e9).toFixed(2) + 'B';
        if (n >= 1e6) return (n/1e6).toFixed(2) + 'M';
        if (n >= 1e3) return (n/1e3).toFixed(1) + 'K';
        return n.toFixed(2);
    }

    // ==================== Dashboard ====================
    const statusBadge = document.getElementById('engine-status');
    const totalBalanceEl = document.getElementById('total-balance');
    const availableBalanceEl = document.getElementById('available-balance');
    const unrealizedPnlEl = document.getElementById('unrealized-pnl');
    const strategyStageEl = document.getElementById('strategy-stage');
    const dashPositions = document.getElementById('dash-positions');

    let dashChart = null;
    let dashCandleSeries = null;

    function initDashChart() {
        const container = document.getElementById('dash-chart');
        if (!container || dashChart) return;

        dashChart = LightweightCharts.createChart(container, {
            layout: { background: { type: 'solid', color: 'transparent' }, textColor: '#7a8599' },
            grid: { vertLines: { color: 'rgba(255,255,255,0.03)' }, horzLines: { color: 'rgba(255,255,255,0.03)' } },
            crosshair: { mode: LightweightCharts.CrosshairMode.Normal },
            rightPriceScale: { borderColor: 'rgba(255,255,255,0.06)' },
            timeScale: { borderColor: 'rgba(255,255,255,0.06)', timeVisible: true },
            handleScroll: false,
            handleScale: false,
        });

        dashCandleSeries = dashChart.addCandlestickSeries({
            upColor: '#22c55e', downColor: '#ef4444',
            borderUpColor: '#22c55e', borderDownColor: '#ef4444',
            wickUpColor: '#22c55e', wickDownColor: '#ef4444',
        });

        loadDashChart();

        document.getElementById('dash-chart-symbol').addEventListener('change', loadDashChart);

        new ResizeObserver(() => {
            dashChart.applyOptions({ width: container.clientWidth });
        }).observe(container);
    }

    async function loadDashChart() {
        const symbol = document.getElementById('dash-chart-symbol').value;
        try {
            const res = await fetch(`/api/v1/klines/${symbol}/15m`);
            const data = await res.json();
            if (data.klines && dashCandleSeries) {
                dashCandleSeries.setData(data.klines);
                dashChart.timeScale().fitContent();
            }
        } catch (e) { console.error('Dash chart error:', e); }
    }

    async function fetchStatus() {
        try {
            const res = await fetch('/api/v1/status');
            if (!res.ok) throw new Error('API Error');
            const data = await res.json();

            // Status badge
            if (data.is_running) {
                statusBadge.innerHTML = '<span class="pulse-dot"></span> 运行中';
                statusBadge.className = 'status-badge';
            } else {
                statusBadge.innerHTML = '<span class="pulse-dot"></span> 等待连接';
                statusBadge.className = 'status-badge error';
            }

            // Stats
            totalBalanceEl.textContent = fmt(data.total_balance);
            availableBalanceEl.textContent = fmt(data.available_balance);
            strategyStageEl.textContent = data.strategy_stage || '--';

            // PnL
            const pnl = parseFloat(data.unrealized_pnl);
            if (!isNaN(pnl)) {
                unrealizedPnlEl.textContent = (pnl >= 0 ? '+' : '') + fmt(pnl);
                unrealizedPnlEl.className = 'stat-value pnl-value ' + (pnl >= 0 ? 'positive' : 'negative');
            }

            // Positions
            if (data.positions && data.positions.length > 0) {
                dashPositions.innerHTML = data.positions.map(p => {
                    const isLong = p.position_side === 'Long' || (p.position_side === 'Both' && parseFloat(p.quantity) > 0);
                    const side = isLong ? 'long' : 'short';
                    const sideLabel = isLong ? '做多' : '做空';
                    const pnlClass = parseFloat(p.unrealized_pnl) >= 0 ? 'positive' : 'negative';
                    return `
                    <div class="pos-mini-card">
                        <div class="pos-mini-header">
                            <span class="pos-mini-symbol">${p.symbol}</span>
                            <span class="pos-mini-side ${side}">${sideLabel}</span>
                        </div>
                        <div class="pos-mini-row"><span>数量</span><span>${fmt(p.quantity, 4)}</span></div>
                        <div class="pos-mini-row"><span>开仓均价</span><span>${fmt(p.entry_price)}</span></div>
                        <div class="pos-mini-row"><span>盈亏</span><span class="${pnlClass}">${fmt(p.unrealized_pnl)}</span></div>
                    </div>`;
                }).join('');

                // Also update positions page table
                updatePositionsTable(data.positions, data.last_price);
            } else {
                dashPositions.innerHTML = '<div class="empty-state">暂无持仓</div>';
                updatePositionsTable([], data.last_price);
            }

            // Tracked Orders (独立订单追踪)
            updateTrackedOrdersTable(data.tracked_orders, data.last_price);
        } catch (e) {
            statusBadge.innerHTML = '<span class="pulse-dot"></span> 离线';
            statusBadge.className = 'status-badge error';
        }
    }

    function updatePositionsTable(positions, lastPrice) {
        const tbody = document.getElementById('positions-table-body');
        if (!positions || positions.length === 0) {
            tbody.innerHTML = '<tr><td colspan="8" class="empty-state">暂无持仓</td></tr>';
            return;
        }
        tbody.innerHTML = positions.map(p => {
            const isLong = p.position_side === 'Long' || (p.position_side === 'Both' && parseFloat(p.quantity) > 0);
            const pnlClass = parseFloat(p.unrealized_pnl) >= 0 ? 'positive' : 'negative';
            const sideLabel = isLong ? '🟢 做多' : '🔴 做空';
            return `
            <tr>
                <td>${p.symbol}</td>
                <td>${sideLabel}</td>
                <td>${fmt(p.quantity, 4)}</td>
                <td>${fmt(p.entry_price)}</td>
                <td>${fmt(p.mark_price || lastPrice)}</td>
                <td>${p.leverage}x</td>
                <td>${fmt(p.margin)}</td>
                <td class="${pnlClass}">${fmt(p.unrealized_pnl)}</td>
            </tr>`;
        }).join('');
    }

    function updateTrackedOrdersTable(orders, lastPrice) {
        const tbody = document.getElementById('tracked-orders-body');
        if (!tbody) return;
        
        const openOrders = (orders || []).filter(o => o.status === 'open');
        if (openOrders.length === 0) {
            tbody.innerHTML = '<tr><td colspan="11" class="empty-state">暂无独立订单</td></tr>';
            return;
        }
        
        const curPrice = parseFloat(lastPrice) || 0;
        
        tbody.innerHTML = openOrders.map(o => {
            const qty = parseFloat(o.quantity) || 0;
            const entry = parseFloat(o.entry_price) || 0;
            const lev = o.leverage || 1;
            const amountU = parseFloat(o.amount_usdt) || 0;
            const isLong = o.direction === 'long';
            
            // 计算盈亏: (当前价 - 开仓价) * 数量 (做多) 或 (开仓价 - 当前价) * 数量 (做空)
            const pnlU = isLong ? (curPrice - entry) * qty : (entry - curPrice) * qty;
            // 盈亏百分比 = 盈亏 / 投入保证金 * 100
            const pnlPct = amountU > 0 ? (pnlU / amountU * 100) : 0;
            
            const pnlClass = pnlU >= 0 ? 'positive' : 'negative';
            const sideLabel = isLong ? '🟢 做多' : '🔴 做空';
            const shortId = o.id.length > 8 ? o.id.slice(-8) : o.id;
            
            return `
            <tr>
                <td title="${o.id}">${shortId}</td>
                <td>${o.symbol}</td>
                <td>${sideLabel}</td>
                <td>${qty.toFixed(4)}</td>
                <td>${entry.toFixed(2)}</td>
                <td>${curPrice.toFixed(2)}</td>
                <td>${lev}x</td>
                <td>${amountU.toFixed(2)}</td>
                <td class="${pnlClass}">${pnlU >= 0 ? '+' : ''}${pnlU.toFixed(2)}</td>
                <td class="${pnlClass}">${pnlPct >= 0 ? '+' : ''}${pnlPct.toFixed(1)}%</td>
                <td><button class="btn btn-danger btn-sm" onclick="closeTrackedOrder('${o.id}')">平仓</button></td>
            </tr>`;
        }).join('');
    }

    window.closeTrackedOrder = async function(orderId) {
        if (!confirm('确定要平掉这笔订单吗？')) return;
        try {
            const res = await fetch('/api/v1/trade/close_order', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ order_id: orderId }),
            });
            const data = await res.json();
            if (data.success) {
                alert('✅ ' + data.message);
                setTimeout(fetchStatus, 500);
            } else {
                alert('❌ ' + (data.error || '平仓失败'));
            }
        } catch (e) {
            alert('网络错误: ' + e.message);
        }
    };

    // ==================== Markets ====================
    let marketsData = [];

    async function loadMarkets() {
        const grid = document.getElementById('markets-grid');
        grid.innerHTML = '<div class="empty-state">正在加载行情数据...</div>';

        try {
            const res = await fetch('/api/v1/markets');
            const data = await res.json();
            marketsData = data.markets || [];
            renderMarkets();
        } catch (e) {
            grid.innerHTML = '<div class="empty-state">加载失败，请稍后重试</div>';
        }
    }

    function renderMarkets(type = 'futures') {
        const grid = document.getElementById('markets-grid');
        const filtered = marketsData.filter(m => m.type === type);

        if (filtered.length === 0) {
            grid.innerHTML = '<div class="empty-state">暂无数据</div>';
            return;
        }

        grid.innerHTML = filtered.map(m => {
            const price = parseFloat(m.price);
            const change = parseFloat(m.change_pct);
            const changeClass = change >= 0 ? 'positive' : 'negative';
            const changeSign = change >= 0 ? '+' : '';
            const displaySymbol = m.symbol.replace('USDT', '/USDT');

            return `
            <div class="market-card" onclick="window.location.hash='chart'; setTimeout(()=>{document.getElementById('chart-symbol').value='${m.symbol}'; document.getElementById('chart-symbol').dispatchEvent(new Event('change'))}, 100)">
                <div class="market-card-header">
                    <span class="market-symbol">${displaySymbol}</span>
                    <span class="market-type">${type === 'futures' ? '合约' : '现货'}</span>
                </div>
                <div class="market-price">${fmt(price, price > 100 ? 2 : 4)}</div>
                <div class="market-meta">
                    <span class="market-change ${changeClass}">${changeSign}${fmt(change)}%</span>
                    <span>Vol: ${fmtCompact(m.volume)}</span>
                </div>
                <div class="market-meta" style="margin-top:4px">
                    <span>H: ${fmt(m.high_24h, price > 100 ? 2 : 4)}</span>
                    <span>L: ${fmt(m.low_24h, price > 100 ? 2 : 4)}</span>
                </div>
            </div>`;
        }).join('');
    }

    // Market tabs
    document.querySelectorAll('.tab-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            renderMarkets(btn.dataset.tab);
        });
    });

    // ==================== Chart ====================
    let mainChart = null;
    let mainCandleSeries = null;
    let mainVolumeSeries = null;
    let currentSymbol = 'ETHUSDT';
    let currentInterval = '15m';
    let chartCache = new Map(); // Simple cache: key -> {time, data}
    let isChartLoading = false;

    function initMainChart() {
        const container = document.getElementById('main-chart');
        if (!container) return;

        if (mainChart) {
            mainChart.remove();
            mainChart = null;
        }

        mainChart = LightweightCharts.createChart(container, {
            layout: { background: { type: 'solid', color: 'transparent' }, textColor: '#7a8599' },
            grid: { vertLines: { color: 'rgba(255,255,255,0.03)' }, horzLines: { color: 'rgba(255,255,255,0.03)' } },
            crosshair: { mode: LightweightCharts.CrosshairMode.Normal },
            rightPriceScale: { borderColor: 'rgba(255,255,255,0.06)' },
            timeScale: { borderColor: 'rgba(255,255,255,0.06)', timeVisible: true, secondsVisible: false },
        });

        mainCandleSeries = mainChart.addCandlestickSeries({
            upColor: '#22c55e', downColor: '#ef4444',
            borderUpColor: '#22c55e', borderDownColor: '#ef4444',
            wickUpColor: '#22c55e', wickDownColor: '#ef4444',
        });

        mainVolumeSeries = mainChart.addHistogramSeries({
            priceFormat: { type: 'volume' },
            priceScaleId: '',
        });

        mainVolumeSeries.priceScale().applyOptions({
            scaleMargins: { top: 0.8, bottom: 0 },
        });

        new ResizeObserver(() => {
            mainChart.applyOptions({ width: container.clientWidth, height: container.clientHeight });
        }).observe(container);
    }

    async function loadChart() {
        if (!mainChart) initMainChart();
        const symbol = document.getElementById('chart-symbol').value;
        currentSymbol = symbol;

        const cacheKey = `${symbol}_${currentInterval}`;
        const cached = chartCache.get(cacheKey);
        const now = Date.now();

        // 2秒内如果有缓存且不是强制刷新，直接用缓存
        if (cached && (now - cached.time < 2000)) {
            renderChartData(cached.data);
            return;
        }

        if (isChartLoading) return;
        isChartLoading = true;

        const priceEl = document.getElementById('chart-live-price');
        if (priceEl) priceEl.style.opacity = '0.5';

        try {
            const res = await fetch(`/api/v1/klines/${symbol}/${currentInterval}`);
            const data = await res.json();
            isChartLoading = false;
            if (priceEl) priceEl.style.opacity = '1';

            if (data.klines) {
                chartCache.set(cacheKey, { time: Date.now(), data: data.klines });
                renderChartData(data.klines);
            }
        } catch (e) { 
            isChartLoading = false;
            if (priceEl) priceEl.style.opacity = '1';
            console.error('Chart load error:', e); 
        }
    }

    function renderChartData(klines) {
        if (!klines || klines.length === 0) return;
        mainCandleSeries.setData(klines);

        const volumeData = klines.map(k => ({
            time: k.time,
            value: k.volume,
            color: k.close >= k.open ? 'rgba(34,197,94,0.3)' : 'rgba(239,68,68,0.3)',
        }));
        mainVolumeSeries.setData(volumeData);
        mainChart.timeScale().fitContent();

        // Update price display
        const last = klines[klines.length - 1];
        const first = klines[0];
        const change = ((last.close - first.open) / first.open * 100);
        const priceEl = document.getElementById('chart-live-price');
        const changeEl = document.getElementById('chart-change');

        if (priceEl) priceEl.textContent = fmt(last.close, last.close > 100 ? 2 : 4);
        if (changeEl) {
            changeEl.textContent = (change >= 0 ? '+' : '') + fmt(change) + '%';
            changeEl.className = 'chart-change ' + (change >= 0 ? 'positive' : 'negative');
        }
    }

    document.getElementById('chart-symbol').addEventListener('change', loadChart);

    document.querySelectorAll('.interval-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.interval-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            currentInterval = btn.dataset.interval;
            loadChart();
        });
    });

    // ==================== Strategy Stages ====================
    async function loadStages() {
        try {
            const res = await fetch('/api/v1/strategy/stages');
            const data = await res.json();
            renderStages(data);
        } catch (e) { console.error('Stages load error:', e); }
    }

    function renderStages(data) {
        const grid = document.getElementById('stages-grid');
        if (!data.strategies) return;
        const lastPrice = parseFloat(data.last_price) || 0;

        grid.innerHTML = data.strategies.map(s => {
            const allocated = parseFloat(s.allocated_funds) || 0;
            const used = parseFloat(s.used_funds) || 0;
            const avail = parseFloat(s.available_funds) || 0;
            const totalPnl = parseFloat(s.total_pnl) || 0;
            const unrealizedPnl = parseFloat(s.unrealized_pnl) || 0;
            const pnlClass = totalPnl >= 0 ? 'positive' : 'negative';
            const urPnlClass = unrealizedPnl >= 0 ? 'positive' : 'negative';
            const winRate = (s.win_count + s.loss_count) > 0 
                ? ((s.win_count / (s.win_count + s.loss_count)) * 100).toFixed(0) 
                : '--';

            // 逐仓纪律策略的阶段指示器
            let phaseHtml = '';
            if (s.id === 'discipline' && allocated > 0) {
                const totalFunds = allocated + totalPnl; // 当前总资金 = 分配 + 累计盈亏
                let phase, orderSize, phaseColor;
                if (totalFunds >= 200) {
                    phase = '🏆 稳健扩张期 (200U+)';
                    orderSize = Math.max(20, totalFunds * 0.1).toFixed(1) + 'U/单';
                    phaseColor = '#22c55e';
                } else if (totalFunds >= 80) {
                    phase = '📈 进阶成长期 (80-200U)';
                    orderSize = '固定10U/单 (留' + Math.floor(avail / 10) + '次试错)';
                    phaseColor = '#4f8ffc';
                } else {
                    phase = '🌱 新手起步期 (<80U)';
                    orderSize = (avail * 0.5).toFixed(1) + 'U/单 (余额50%)';
                    phaseColor = '#f59e0b';
                }
                phaseHtml = `
                <div style="background:rgba(79,143,252,0.08);border:1px solid rgba(79,143,252,0.15);border-radius:8px;padding:10px;margin:8px 0;">
                    <div style="display:flex;justify-content:space-between;align-items:center;">
                        <span style="color:${phaseColor};font-weight:600;">${phase}</span>
                        <span style="color:var(--text-muted);font-size:0.85rem;">每单: ${orderSize}</span>
                    </div>
                    <div style="display:flex;gap:16px;margin-top:6px;font-size:0.8rem;color:var(--text-dim);">
                        <span>止损: 20%</span><span>止盈: 100%</span><span>杠杆: 100x</span><span>逐仓模式</span>
                    </div>
                </div>`;
            }

            const ordersHtml = (s.open_orders && s.open_orders.length > 0) 
                ? s.open_orders.map(o => {
                    const urPnl = parseFloat(o.unrealized_pnl) || 0;
                    const oClass = urPnl >= 0 ? 'positive' : 'negative';
                    const dir = o.direction === 'long' ? '🟢多' : '🔴空';
                    return `<div class="strategy-order-row">
                        <span>${o.symbol} ${dir}</span>
                        <span>数量: ${parseFloat(o.quantity).toFixed(4)}</span>
                        <span>开仓: ${parseFloat(o.entry_price).toFixed(2)}</span>
                        <span class="${oClass}">${urPnl >= 0 ? '+' : ''}${urPnl.toFixed(2)}U</span>
                        <button class="btn btn-danger btn-sm" onclick="closeTrackedOrder('${o.id}')">平仓</button>
                    </div>`;
                }).join('')
                : '<div class="empty-state" style="padding:8px;font-size:0.85rem;">暂无持仓订单</div>';

            return `
            <div class="stage-card ${s.active ? 'active-stage' : ''}" style="width:100%;">
                <div class="stage-card-header">
                    <span class="stage-name">${s.name}</span>
                    <label class="toggle">
                        <input type="checkbox" ${s.active ? 'checked' : ''} onchange="toggleStage('${s.id}', this.checked)">
                        <span class="toggle-slider"></span>
                    </label>
                </div>
                <p class="stage-desc">${s.description}</p>
                ${phaseHtml}
                
                <div style="display:grid; grid-template-columns: repeat(4, 1fr); gap:10px; margin: 12px 0;">
                    <div style="text-align:center;">
                        <div style="color:var(--text-dim);font-size:0.75rem;">分配资金</div>
                        <div style="font-size:1.1rem;font-weight:600;color:var(--text);">${allocated.toFixed(1)}U</div>
                    </div>
                    <div style="text-align:center;">
                        <div style="color:var(--text-dim);font-size:0.75rem;">已用/可用</div>
                        <div style="font-size:1.1rem;font-weight:600;color:var(--accent);">${used.toFixed(1)}/${avail.toFixed(1)}U</div>
                    </div>
                    <div style="text-align:center;">
                        <div style="color:var(--text-dim);font-size:0.75rem;">累计盈亏</div>
                        <div class="${pnlClass}" style="font-size:1.1rem;font-weight:600;">${totalPnl >= 0 ? '+' : ''}${totalPnl.toFixed(2)}U</div>
                    </div>
                    <div style="text-align:center;">
                        <div style="color:var(--text-dim);font-size:0.75rem;">胜率</div>
                        <div style="font-size:1.1rem;font-weight:600;color:var(--text);">${winRate}% (${s.win_count}W/${s.loss_count}L)</div>
                    </div>
                </div>

                <div style="display:flex; gap:8px; align-items:center; margin-bottom:10px;">
                    <input type="number" id="alloc-${s.id}" class="text-input" value="${allocated}" min="0" step="10" style="width:120px;" placeholder="分配金额">
                    <button class="btn btn-primary btn-sm" onclick="allocateFunds('${s.id}')">💰 分配资金</button>
                </div>

                <div style="border-top: 1px solid rgba(255,255,255,0.06); padding-top: 10px;">
                    <div style="color:var(--text-dim);font-size:0.8rem;margin-bottom:6px;">
                        📋 当前持仓 <span class="${urPnlClass}">(浮动盈亏: ${unrealizedPnl >= 0 ? '+' : ''}${unrealizedPnl.toFixed(2)}U)</span>
                    </div>
                    ${ordersHtml}
                </div>
            </div>`;
        }).join('');
    }

    // 分配资金
    window.allocateFunds = async function(strategyId) {
        const input = document.getElementById('alloc-' + strategyId);
        const amount = parseFloat(input.value) || 0;
        try {
            const res = await fetch('/api/v1/strategy/allocate', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ strategy_id: strategyId, amount }),
            });
            const data = await res.json();
            if (data.success) {
                loadStages();
            } else {
                alert('分配失败: ' + data.error);
            }
        } catch (e) { alert('网络错误: ' + e.message); }
    };

    // 切换策略启用状态
    window.toggleStage = async function(stageId, active) {
        try {
            const res = await fetch('/api/v1/strategy/stages/update', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ stage_id: stageId, active }),
            });
            const result = await res.json();
            if (result.success) {
                loadStages();
            }
        } catch (e) { console.error('Toggle error:', e); }
    };

    // ==================== Manual Trading ====================
    const tradeResult = document.getElementById('trade-result');

    async function placeOrder(direction) {
        const symbol = document.getElementById('trade-symbol').value;
        const amount = parseFloat(document.getElementById('trade-amount').value);
        const priceStr = document.getElementById('trade-price').value;
        const price = priceStr ? parseFloat(priceStr) : null;
        const leverage = parseInt(document.getElementById('trade-leverage').value);
        const sl = parseFloat(document.getElementById('trade-sl').value);
        const tp = parseFloat(document.getElementById('trade-tp').value);
        const strategyId = document.getElementById('trade-strategy').value || null;

        if (!amount || amount < 5) {
            tradeResult.textContent = '⚠️ 最小下单金额 5U';
            tradeResult.className = 'trade-result error';
            return;
        }

        const side = direction.includes('long') ? 'buy' : 'sell';
        tradeResult.textContent = '⏳ 下单中...';
        tradeResult.className = 'trade-result';

        try {
            const res = await fetch('/api/v1/trade/order', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    symbol,
                    side,
                    direction,
                    amount_usdt: amount,
                    leverage,
                    price: isNaN(price) ? null : price,
                    stop_loss_pct: sl || null,
                    take_profit_pct: tp || null,
                    strategy_id: strategyId,
                }),
            });
            const data = await res.json();
            if (data.success) {
                tradeResult.textContent = '✅ ' + data.message;
                tradeResult.className = 'trade-result success';
                // 下单成功后立刻刷新状态以更新持仓列表
                setTimeout(fetchStatus, 500);
                setTimeout(loadStages, 600);
            } else {
                tradeResult.textContent = '❌ ' + (data.error || '下单失败');
                tradeResult.className = 'trade-result error';
            }
        } catch (e) {
            tradeResult.textContent = '❌ 网络错误: ' + e.message;
            tradeResult.className = 'trade-result error';
        }
    }

    document.getElementById('btn-open-long').addEventListener('click', () => placeOrder('open_long'));
    document.getElementById('btn-open-short').addEventListener('click', () => placeOrder('open_short'));
    document.getElementById('btn-close-long').addEventListener('click', () => placeOrder('close_long'));
    document.getElementById('btn-close-short').addEventListener('click', () => placeOrder('close_short'));

    window.closeAllPositions = async function() {
        if (!confirm('确定要平掉所有当前持仓吗？')) return;
        
        try {
            const res = await fetch('/api/v1/trade/close_all', { method: 'POST' });
            const data = await res.json();
            if (data.success) {
                alert('一键平仓指令已发送！\n' + data.message);
                setTimeout(fetchStatus, 500);
            } else {
                alert('一键平仓失败: ' + (data.error || '未知错误'));
            }
        } catch (e) {
            alert('网络错误: ' + e.message);
        }
    };

    const btnCloseAll = document.getElementById('btn-close-all');
    if (btnCloseAll) {
        btnCloseAll.addEventListener('click', closeAllPositions);
    }

    // ==================== Init ====================
    handleHash();
    initDashChart();
    fetchStatus();

    // Periodic updates
    setInterval(fetchStatus, 3000);
    setInterval(loadStages, 5000); // refresh strategy data every 5s
    setInterval(loadDashChart, 30000); // refresh dash chart every 30s
});
