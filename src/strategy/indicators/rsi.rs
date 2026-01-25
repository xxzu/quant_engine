//! RSI 相对强弱指标

use rust_decimal::Decimal;

/// 计算 RSI 指标
/// 
/// # 参数
/// - prices: 收盘价序列
/// - period: 计算周期 (默认14)
pub fn rsi(prices: &[Decimal], period: usize) -> Vec<Option<Decimal>> {
    if prices.len() < 2 || period == 0 {
        return vec![None; prices.len()];
    }

    let mut result = Vec::with_capacity(prices.len());
    result.push(None); // 第一个价格没有变化

    // 计算价格变化
    let mut gains = Vec::with_capacity(prices.len() - 1);
    let mut losses = Vec::with_capacity(prices.len() - 1);

    for i in 1..prices.len() {
        let change = prices[i] - prices[i - 1];
        if change > Decimal::ZERO {
            gains.push(change);
            losses.push(Decimal::ZERO);
        } else {
            gains.push(Decimal::ZERO);
            losses.push(change.abs());
        }
    }

    // 计算平均涨跌幅
    let mut avg_gain = Decimal::ZERO;
    let mut avg_loss = Decimal::ZERO;

    for i in 0..gains.len() {
        if i + 1 < period {
            result.push(None);
        } else if i + 1 == period {
            // 第一个 RSI：使用简单平均
            avg_gain = gains[0..period].iter().sum::<Decimal>() / Decimal::from(period);
            avg_loss = losses[0..period].iter().sum::<Decimal>() / Decimal::from(period);

            let rsi_value = calculate_rsi(avg_gain, avg_loss);
            result.push(Some(rsi_value));
        } else {
            // 后续 RSI：使用平滑平均
            avg_gain = (avg_gain * Decimal::from(period - 1) + gains[i]) / Decimal::from(period);
            avg_loss = (avg_loss * Decimal::from(period - 1) + losses[i]) / Decimal::from(period);

            let rsi_value = calculate_rsi(avg_gain, avg_loss);
            result.push(Some(rsi_value));
        }
    }

    result
}

fn calculate_rsi(avg_gain: Decimal, avg_loss: Decimal) -> Decimal {
    if avg_loss.is_zero() {
        return Decimal::from(100);
    }
    let rs = avg_gain / avg_loss;
    Decimal::from(100) - (Decimal::from(100) / (Decimal::ONE + rs))
}

/// 判断是否超买 (RSI > 70)
pub fn is_overbought(rsi_value: Option<Decimal>, threshold: Decimal) -> bool {
    rsi_value.map_or(false, |v| v > threshold)
}

/// 判断是否超卖 (RSI < 30)
pub fn is_oversold(rsi_value: Option<Decimal>, threshold: Decimal) -> bool {
    rsi_value.map_or(false, |v| v < threshold)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_rsi_basic() {
        let prices = vec![
            dec!(44), dec!(44.34), dec!(44.09), dec!(43.61), dec!(44.33),
            dec!(44.83), dec!(45.10), dec!(45.42), dec!(45.84), dec!(46.08),
            dec!(45.89), dec!(46.03), dec!(45.61), dec!(46.28), dec!(46.28),
        ];

        let result = rsi(&prices, 14);
        assert_eq!(result.len(), prices.len());
        
        // 前 14 个应该是 None
        for i in 0..14 {
            assert!(result[i].is_none() || i == 14);
        }
    }
}
