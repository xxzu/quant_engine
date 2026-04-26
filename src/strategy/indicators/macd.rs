//! MACD 指标

use super::ma::ema;
use rust_decimal::Decimal;

/// MACD 结果
#[derive(Debug, Clone)]
pub struct MacdResult {
    /// DIF 线 (快线 - 慢线)
    pub dif: Vec<Option<Decimal>>,
    /// DEA 线 (DIF 的 EMA)
    pub dea: Vec<Option<Decimal>>,
    /// MACD 柱状图 (DIF - DEA) * 2
    pub histogram: Vec<Option<Decimal>>,
}

/// 计算 MACD 指标
///
/// # 参数
/// - prices: 收盘价序列
/// - fast_period: 快线周期 (默认12)
/// - slow_period: 慢线周期 (默认26)
/// - signal_period: 信号线周期 (默认9)
pub fn macd(
    prices: &[Decimal],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> MacdResult {
    let fast_ema = ema(prices, fast_period);
    let slow_ema = ema(prices, slow_period);

    // 计算 DIF = 快线 - 慢线
    let mut dif: Vec<Option<Decimal>> = Vec::with_capacity(prices.len());
    let mut dif_values: Vec<Decimal> = Vec::new();

    for i in 0..prices.len() {
        match (
            fast_ema.get(i).copied().flatten(),
            slow_ema.get(i).copied().flatten(),
        ) {
            (Some(fast), Some(slow)) => {
                let diff = fast - slow;
                dif.push(Some(diff));
                dif_values.push(diff);
            }
            _ => {
                dif.push(None);
            }
        }
    }

    // 计算 DEA = DIF 的 EMA
    let dea_raw = ema(&dif_values, signal_period);
    let mut dea: Vec<Option<Decimal>> = vec![None; prices.len() - dif_values.len()];
    dea.extend(dea_raw);

    // 计算 MACD 柱状图 = (DIF - DEA) * 2
    let mut histogram: Vec<Option<Decimal>> = Vec::with_capacity(prices.len());
    for i in 0..prices.len() {
        match (dif.get(i).copied().flatten(), dea.get(i).copied().flatten()) {
            (Some(d), Some(e)) => {
                histogram.push(Some((d - e) * Decimal::from(2)));
            }
            _ => {
                histogram.push(None);
            }
        }
    }

    MacdResult {
        dif,
        dea,
        histogram,
    }
}

/// 判断 MACD 金叉
pub fn is_macd_golden_cross(result: &MacdResult, index: usize) -> bool {
    if index < 1 || index >= result.dif.len() {
        return false;
    }

    match (
        result.dif[index - 1],
        result.dif[index],
        result.dea[index - 1],
        result.dea[index],
    ) {
        (Some(prev_dif), Some(curr_dif), Some(prev_dea), Some(curr_dea)) => {
            prev_dif <= prev_dea && curr_dif > curr_dea
        }
        _ => false,
    }
}

/// 判断 MACD 死叉
pub fn is_macd_death_cross(result: &MacdResult, index: usize) -> bool {
    if index < 1 || index >= result.dif.len() {
        return false;
    }

    match (
        result.dif[index - 1],
        result.dif[index],
        result.dea[index - 1],
        result.dea[index],
    ) {
        (Some(prev_dif), Some(curr_dif), Some(prev_dea), Some(curr_dea)) => {
            prev_dif >= prev_dea && curr_dif < curr_dea
        }
        _ => false,
    }
}
