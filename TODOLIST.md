QuantEngine 生产级量化交易系统 — 实施路线图
当前状态评估
经过对全部源代码的审计，当前项目是一个 结构良好但功能未完成的框架模板：

✅ 已完成（骨架就绪）
模块	状态	说明
项目结构	✅ 完整	config / data / strategy / backtest / trading / api / utils 六大模块划分清晰
数据模型	✅ 基本完整	Kline, BarData, TickData, Stock, Order, Trade, Position, Account
数据库	✅ Schema 已定义	MySQL，6张核心表（stocks/klines/orders/trades/positions/backtests）
技术指标	✅ 3个	SMA/EMA、MACD、RSI，含金叉/死叉判断
回测引擎	✅ 基础版	单股回测流程完整，含撮合器(滑点/部分成交)和绩效计算(夏普/最大回撤/卡玛/索提诺)
风控系统	✅ 框架版	资金/持仓/频率/黑名单/比例检查
Paper Broker	✅ 可用	模拟交易含手续费(佣金/印花税/过户费)、T+1 限制
API 层	✅ 路由已定义	Axum Web 框架，健康检查/股票/策略路由
❌ 缺失（走向生产需要补齐）
缺失项	严重程度	说明
数据采集	🔴 致命	没有任何数据源对接，数据库是空的
实盘 Broker	🔴 致命	只有 PaperBroker，无法连接真实券商
实时行情	🔴 致命	TickData 只有结构体定义，没有 WebSocket/数据推送
API 实现	🟡 严重	所有 handler 返回硬编码/空数据，全是 TODO
具体策略	🟡 严重	Strategy trait 已定义但没有任何策略实现
事件驱动	🟡 严重	回测是简单 for 循环，不支持事件总线
调度系统	🟡 严重	没有定时任务、策略调度、数据更新调度
日志/监控	🟡 中等	只有基础 tracing，缺少结构化日志、指标采集
交易日历	🟡 中等	is_trading_day 只判断周末，没有节假日处理
配置管理	🟠 一般	config.yaml 字段过于简单，无环境隔离
错误处理	🟠 一般	API 层没有统一错误响应格式
测试覆盖	🟠 一般	只有3个单元测试，无集成测试
认证鉴权	🟠 一般	API 完全无保护
前端 UI	⚪ 可选	无可视化界面
实施阶段总览
05/03
05/10
05/17
05/24
05/31
06/07
06/14
06/21
06/28
07/05
07/12
07/19
07/26
配置体系重构
统一错误处理
结构化日志
交易日历完善
数据源适配器
历史数据爬取
实时行情推送
数据清洗存储
事件驱动架构
示例策略实现
回测引擎增强
策略参数优化
券商 API 对接
实盘风控强化
订单管理系统
Prometheus 指标
告警通知系统
API 认证与完善
多策略组合
Web 可视化面板
机器学习因子
第一阶段: 基础设施
第二阶段: 数据管线
第三阶段: 策略引擎
第四阶段: 实盘交易
第五阶段: 监控运维
第六阶段: 高级功能
QuantEngine 生产化路线图
第一阶段：基础设施加固（约 9 天）
目标：让框架具备生产级的配置、错误处理、日志、交易日历能力

1.1 配置体系重构
[MODIFY] 
config.yaml
拆分为多环境配置：config.dev.yaml / config.prod.yaml
增加字段：redis_url, data_source, broker_config, risk_config, log_level
[MODIFY] 
sys_config.rs
用 config crate 替代 serde_yaml 手动加载，支持环境变量覆盖
配置结构体分层：AppConfig { server, database, redis, data_source, trading, risk, log }
[NEW] src/config/trading_config.rs
交易相关配置：佣金费率、滑点设置、最大持仓、策略白名单
1.2 统一错误处理
[NEW] src/api/error.rs
定义 AppError 枚举，实现 IntoResponse
统一 JSON 错误响应格式：{ "code": 40001, "message": "...", "data": null }
[MODIFY] 所有 api/handlers/*.rs
handler 返回 Result<Json<T>, AppError> 替代裸 Json<T>
1.3 结构化日志
[MODIFY] 
main.rs
添加 tracing-appender 文件日志输出
JSON 格式日志 + 按日期滚动
关键交易操作添加 tracing::instrument 追踪
1.4 交易日历完善
[MODIFY] 
datetime.rs
加载 A 股节假日数据（JSON 文件或数据库表）
is_trading_day() 支持节假日判断
添加 get_trading_days(start, end) -> Vec<NaiveDate> 方法
[NEW] data/holidays.json
A 股历年节假日数据文件
第二阶段：数据采集管线（约 18 天）
目标：实现从外部数据源获取历史和实时行情，填充数据库

2.1 数据源适配器
[NEW] src/data/source/mod.rs
rust
#[async_trait]
pub trait DataSource: Send + Sync {
    async fn fetch_stock_list(&self) -> Result<Vec<Stock>>;
    async fn fetch_daily_klines(&self, code: &str, start: NaiveDate, end: NaiveDate) -> Result<Vec<Kline>>;
    async fn subscribe_realtime(&self, codes: &[String]) -> Result<broadcast::Receiver<TickData>>;
}
[NEW] src/data/source/tushare.rs
对接 Tushare Pro API（最常用的A股数据源）
实现股票列表、日K线、分钟K线获取
[NEW] src/data/source/akshare.rs（备选）
对接 AKShare HTTP 接口作为免费替代
[NEW] src/data/source/sina.rs
新浪实时行情接口（免费实时数据）
2.2 历史数据采集
[NEW] src/data/collector/mod.rs
数据采集调度器：支持增量更新和全量回补
断点续爬、速率限制、重试机制
[NEW] src/data/collector/kline_collector.rs
历史 K 线数据采集任务
并发控制（避免被数据源封禁）
2.3 实时行情推送
[NEW] src/data/realtime/mod.rs
实时行情管理器：订阅/取消订阅
tokio::broadcast 分发 TickData
[NEW] src/data/realtime/websocket.rs
WebSocket 推送给前端客户端
行情快照缓存（Redis）
2.4 数据清洗与存储
[MODIFY] 
kline_repo.rs
添加批量 UPSERT 优化（当前是逐条插入）
添加数据完整性检查方法
[NEW] src/data/cleaner.rs
除权除息复权处理（前复权/后复权）
缺失数据检测和填充
第三阶段：策略引擎增强（约 18 天）
目标：事件驱动架构 + 多策略 + 增强回测

3.1 事件驱动架构
[NEW] src/engine/event.rs
rust
pub enum Event {
    Tick(TickData),
    Bar(BarData),
    Order(OrderEvent),
    Trade(TradeEvent),
    Timer(TimerEvent),
    Signal(Signal),
    Risk(RiskEvent),
}
[NEW] src/engine/event_bus.rs
基于 tokio::mpsc 的事件总线
支持事件订阅和发布
[NEW] src/engine/engine.rs
核心引擎：事件循环 + 模块编排
统一管理 DataFeed → Strategy → RiskCheck → Broker 的流程
3.2 示例策略实现
[NEW] src/strategy/strategies/ma_cross.rs
双均线交叉策略（完整实现 Strategy trait）
支持参数配置：短期/长期均线周期、仓位比例
[NEW] src/strategy/strategies/macd_strategy.rs
MACD 金叉死叉策略
[NEW] src/strategy/strategies/rsi_strategy.rs
RSI 超买超卖策略
[NEW] src/strategy/strategies/turtle.rs
海龟交易法则（经典趋势跟踪策略）
3.3 回测引擎增强
[MODIFY] 
engine.rs
支持多股票同时回测
支持分钟级/Tick级回测
改进胜率计算（当前逻辑有 Bug：按总资产而非单笔交易计算）
添加逐笔交易记录和分析
[MODIFY] 
performance.rs
新增指标：信息比率、Omega 比率、月度收益统计
添加基准对比（vs 沪深300）
导出回测报告（HTML/JSON）
[NEW] src/backtest/report.rs
回测报告生成器
权益曲线数据导出
3.4 策略参数优化
[NEW] src/backtest/optimizer.rs
网格搜索参数优化
并行回测加速（rayon / tokio::spawn）
Walk-forward 滚动优化
第四阶段：实盘交易对接（约 17 天）
目标：连接真实券商，实现实盘下单

4.1 券商 API 对接
[NEW] src/trading/broker/broker_trait.rs
rust
#[async_trait]
pub trait Broker: Send + Sync {
    async fn submit_order(&self, order: &Order) -> Result<String>;
    async fn cancel_order(&self, order_id: &str) -> Result<()>;
    async fn query_orders(&self, filter: &OrderFilter) -> Result<Vec<Order>>;
    async fn query_positions(&self) -> Result<Vec<Position>>;
    async fn query_account(&self) -> Result<Account>;
}
[NEW] src/trading/broker/ctp.rs
CTP 柜台接口（期货）— 通过 FFI 调用 C++ SDK
[NEW] src/trading/broker/xtp.rs
XTP 中泰证券接口（股票）
[NEW] src/trading/broker/simulated.rs
增强版模拟盘（基于实时行情撮合，替代当前 PaperBroker）
IMPORTANT

券商对接是最复杂的部分，不同券商有不同的 SDK。建议优先对接一家券商验证流程，再扩展。 另一个选择是通过 QMT（迅投）、掘金量化等中间件间接对接，降低复杂度。

4.2 实盘风控强化
[MODIFY] 
risk.rs
添加熔断机制：单日亏损超阈值自动停止交易
添加异常检测：价格突变、成交量异常
实时仓位同步（对比本地和券商仓位）
[NEW] src/trading/risk/circuit_breaker.rs
熔断器实现：按日/按周/按月亏损限制
[NEW] src/trading/risk/position_sync.rs
定时和券商同步仓位，发现不一致立即告警
4.3 订单管理系统 (OMS)
[NEW] src/trading/oms.rs
订单生命周期管理
订单路由：根据标的自动选择 Broker
订单状态追踪和持久化
[NEW] src/data/repositories/order_repo.rs
订单/成交记录持久化到数据库
第五阶段：监控运维体系（约 11 天）
目标：具备生产级的可观测性和安全性

5.1 指标采集
[NEW] src/monitoring/metrics.rs
集成 metrics + metrics-exporter-prometheus
核心指标：订单延迟、策略信号数、账户净值、持仓集中度
5.2 告警通知
[NEW] src/monitoring/alert.rs
异常告警：交易失败、风控触发、系统错误
通知渠道：企业微信 / 钉钉 / 邮件
5.3 API 认证与完善
[NEW] src/api/middleware/auth.rs
JWT Token 认证
API Key 鉴权
[MODIFY] 
router.rs
完善所有 API 端点，连接真实数据
添加：回测结果查询、持仓查询、订单操作、策略管理、系统状态
第六阶段：高级功能（可选，持续迭代）
6.1 多策略组合
策略注册中心 + 资金分配器
策略之间信号汇总和冲突解决
6.2 Web 可视化面板
前端技术栈：Vite + React/Vue
功能：行情看板、回测报表、持仓监控、订单管理
6.3 机器学习因子
集成 linfa（Rust ML库）或通过 Python 子进程调用
多因子选股模型
推荐依赖新增
toml
# 新增到 Cargo.toml
# 定时任务
tokio-cron-scheduler = "0.10"
# WebSocket
tokio-tungstenite = "0.21"
# 指标监控
metrics = "0.22"
metrics-exporter-prometheus = "0.13"
# JWT 认证
jsonwebtoken = "9.2"
# 并行计算（参数优化）
rayon = "1.8"
# 更好的配置管理
config = "0.14"
# 通知（HTTP 回调）
# reqwest 已有
# 数据序列化增强
csv = "1.3"
验证方案
每阶段验收标准
阶段	验收标准
第一阶段	cargo build --release 无警告；多环境配置加载正确；交易日历节假日测试通过
第二阶段	能自动采集并存储 ≥100 只股票的历史日K线；实时行情延迟 < 500ms
第三阶段	双均线策略回测结果与手工计算一致；多股票回测 ≥ 1000 日完成 < 5s
第四阶段	模拟盘连续运行 7 天无异常；订单状态全生命周期正确
第五阶段	Prometheus 指标可采集；异常告警 < 30s 到达
第六阶段	Web 面板可展示回测曲线和实时持仓
持续测试
bash
# 单元测试
cargo test --workspace
# 集成测试（需要数据库）
cargo test --test '*' -- --ignored
# 基准测试
cargo bench
开放问题（需要你的决策）
IMPORTANT

1. 目标市场是什么？ 当前代码明显面向 A 股（SH/SZ/BJ交易所、T+1、印花税）。是否也需要支持期货/数字货币？这会显著影响 Broker 和数据源的选择。

IMPORTANT

2. 数据源选择？

Tushare Pro（需要积分，数据质量好）
AKShare（免费，但稳定性一般）
自有数据库（已有历史数据？）
这决定了第二阶段的实现优先级。

IMPORTANT

3. 券商对接方式？

直接对接券商 SDK（CTP/XTP）— 复杂但灵活
通过量化中间件（QMT/掘金/恒生PTrade）— 简单但受限
先只做模拟盘 + 手动执行信号 — 最简单的起步方式
建议：先做好模拟盘 + 信号通知，后期再接券商。

WARNING

4. 是否需要 Web 前端？ 如果只是个人使用，命令行 + 日志 + 通知可能就够了。Web 面板开发量较大（约 2-3 周额外工作）。

建议执行顺序（最小可行路径）
如果你希望尽快跑通一个端到端的量化交易流程，建议按以下精简路径：

1. 数据采集 → 接入一个免费数据源，灌入历史K线
2. 策略实现 → 实现一个双均线策略
3. 回测验证 → 完善回测引擎，验证策略表现
4. 模拟盘   → 用实时行情 + PaperBroker 跑模拟
5. 信号通知 → 发信号到手机，手动跟单
6. 再考虑   → 实盘对接、Web UI 等
这条路径大约 3-4 周可以走通，后续再逐步补齐其他能力。