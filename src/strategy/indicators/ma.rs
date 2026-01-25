//! 移动平均线指标

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// 简单移动平均线 (SMA)
pub fn sma(prices: &[Decimal], period: usize) -> Vec<Option<Decimal>> {
    if prices.is_empty() || period == 0 {
        return vec![];
    }

    let mut result = Vec::with_capacity(prices.len());

    for i in 0..prices.len() {
        if i + 1 < period {
            result.push(None);
        } else {
            let sum: Decimal = prices[i + 1 - period..=i].iter().sum();
            result.push(Some(sum / Decimal::from(period)));
        }
    }

    result
}

/// 指数移动平均线 (EMA)
pub fn ema(prices: &[Decimal], period: usize) -> Vec<Option<Decimal>> {
    if prices.is_empty() || period == 0 {
        return vec![];
    }

    let mut result = Vec::with_capacity(prices.len());
    let multiplier = Decimal::from(2) / Decimal::from(period + 1);

    let mut prev_ema: Option<Decimal> = None;

    for (i, &price) in prices.iter().enumerate() {
        if i + 1 < period {
            result.push(None);
        } else if i + 1 == period {
            // 第一个 EMA 使用 SMA
            let sum: Decimal = prices[0..=i].iter().sum();
            let first_ema = sum / Decimal::from(period);
            prev_ema = Some(first_ema);
            result.push(prev_ema);
        } else {
            // EMA = Price * k + EMA_prev * (1 - k)
            let current_ema = price * multiplier + prev_ema.unwrap() * (Decimal::ONE - multiplier);
            prev_ema = Some(current_ema);
            result.push(prev_ema);
        }
    }

    result
}

/// 判断金叉 (短期均线上穿长期均线)
pub fn is_golden_cross(
    short_ma: &[Option<Decimal>],
    long_ma: &[Option<Decimal>],
    index: usize,
) -> bool {
    if index < 1 || index >= short_ma.len() || index >= long_ma.len() {
        return false;
    }

    match (
        short_ma[index - 1],
        short_ma[index],
        long_ma[index - 1],
        long_ma[index],
    ) {
        (Some(prev_short), Some(curr_short), Some(prev_long), Some(curr_long)) => {
            prev_short <= prev_long && curr_short > curr_long
        }
        _ => false,
    }
}

/// 判断死叉 (短期均线下穿长期均线)
pub fn is_death_cross(
    short_ma: &[Option<Decimal>],
    long_ma: &[Option<Decimal>],
    index: usize,
) -> bool {
    if index < 1 || index >= short_ma.len() || index >= long_ma.len() {
        return false;
    }

    match (
        short_ma[index - 1],
        short_ma[index],
        long_ma[index - 1],
        long_ma[index],
    ) {
        (Some(prev_short), Some(curr_short), Some(prev_long), Some(curr_long)) => {
            prev_short >= prev_long && curr_short < curr_long
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_sma() {
        let prices = vec![dec!(10), dec!(11), dec!(12), dec!(13), dec!(14)];
        let result = sma(&prices, 3);

        assert_eq!(result.len(), 5);
        assert_eq!(result[0], None);
        assert_eq!(result[1], None);
        assert_eq!(result[2], Some(dec!(11))); // (10+11+12)/3
        assert_eq!(result[3], Some(dec!(12))); // (11+12+13)/3
        assert_eq!(result[4], Some(dec!(13))); // (12+13+14)/3
    }
}
