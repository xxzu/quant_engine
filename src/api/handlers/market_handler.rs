//! 行情数据处理器

use axum::{extract::Path, Json};

/// 热门合约交易对列表
const FUTURES_SYMBOLS: &[&str] = &[
    "BTCUSDT",
    "ETHUSDT",
    "BNBUSDT",
    "SOLUSDT",
    "XRPUSDT",
    "DOGEUSDT",
    "ADAUSDT",
    "AVAXUSDT",
    "DOTUSDT",
    "LINKUSDT",
    "MATICUSDT",
    "LTCUSDT",
];

/// 热门现货交易对列表
const SPOT_SYMBOLS: &[&str] = &[
    "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "DOGEUSDT", "ADAUSDT", "AVAXUSDT",
];

/// 获取支持的交易对列表
pub async fn get_symbols() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "futures": FUTURES_SYMBOLS,
        "spot": SPOT_SYMBOLS,
    }))
}

/// 获取多交易对实时行情 (通过币安公开API)
pub async fn get_market_prices() -> Json<serde_json::Value> {
    let client = reqwest::Client::new();

    // 合约价格
    let futures_url = "https://testnet.binancefuture.com/fapi/v1/ticker/price";
    let futures_prices = match client.get(futures_url).send().await {
        Ok(resp) => match resp.json::<Vec<serde_json::Value>>().await {
            Ok(data) => data
                .into_iter()
                .filter(|item| {
                    let symbol = item["symbol"].as_str().unwrap_or("");
                    FUTURES_SYMBOLS.contains(&symbol)
                })
                .collect::<Vec<_>>(),
            Err(_) => vec![],
        },
        Err(_) => vec![],
    };

    // 合约24hr变动
    let ticker_url = "https://testnet.binancefuture.com/fapi/v1/ticker/24hr";
    let tickers_24hr = match client.get(ticker_url).send().await {
        Ok(resp) => resp
            .json::<Vec<serde_json::Value>>()
            .await
            .unwrap_or_default(),
        Err(_) => vec![],
    };

    // 组合数据
    let mut markets = vec![];
    for fp in &futures_prices {
        let symbol = fp["symbol"].as_str().unwrap_or("");
        let price = fp["price"].as_str().unwrap_or("0");

        // 查找24hr变动
        let ticker = tickers_24hr
            .iter()
            .find(|t| t["symbol"].as_str() == Some(symbol));
        let change_pct = ticker
            .and_then(|t| t["priceChangePercent"].as_str())
            .unwrap_or("0");
        let volume = ticker
            .and_then(|t| t["quoteVolume"].as_str())
            .unwrap_or("0");
        let high = ticker.and_then(|t| t["highPrice"].as_str()).unwrap_or("0");
        let low = ticker.and_then(|t| t["lowPrice"].as_str()).unwrap_or("0");

        markets.push(serde_json::json!({
            "symbol": symbol,
            "price": price,
            "change_pct": change_pct,
            "volume": volume,
            "high_24h": high,
            "low_24h": low,
            "type": "futures",
        }));
    }

    Json(serde_json::json!({ "markets": markets }))
}

/// 获取K线数据 (供前端图表使用)
pub async fn get_klines(
    Path((symbol, interval)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://testnet.binancefuture.com/fapi/v1/klines?symbol={}&interval={}&limit=200",
        symbol, interval
    );

    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<Vec<Vec<serde_json::Value>>>().await {
            Ok(data) => {
                let klines: Vec<serde_json::Value> = data.iter().map(|k| {
                        serde_json::json!({
                            "time": k.first().and_then(|v| v.as_i64()).unwrap_or(0) / 1000,
                            "open": k.get(1).and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0),
                            "high": k.get(2).and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0),
                            "low": k.get(3).and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0),
                            "close": k.get(4).and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0),
                            "volume": k.get(5).and_then(|v| v.as_str()).unwrap_or("0").parse::<f64>().unwrap_or(0.0),
                        })
                    }).collect();
                Json(serde_json::json!({ "klines": klines }))
            }
            Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
        },
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}
