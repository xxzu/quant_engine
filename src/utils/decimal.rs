//! Decimal 工具函数

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// 格式化金额显示 (保留2位小数，千分位分隔)
pub fn format_amount(amount: Decimal) -> String {
    let value = amount.round_dp(2).to_f64().unwrap_or(0.0);
    let parts: Vec<&str> = format!("{:.2}", value).split('.').collect();
    
    let integer_part = parts[0];
    let decimal_part = parts.get(1).unwrap_or(&"00");
    
    // 添加千分位
    let formatted: String = integer_part
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            if i > 0 && i % 3 == 0 && c != '-' {
                format!(",{}", c)
            } else {
                c.to_string()
            }
        })
        .collect::<Vec<_>>()
        .concat()
        .chars()
        .rev()
        .collect();

    format!("{}.{}", formatted, decimal_part)
}

/// 格式化百分比显示
pub fn format_percent(value: Decimal) -> String {
    format!("{:.2}%", value.round_dp(2))
}

/// 格式化价格 (股票价格保留2-3位小数)
pub fn format_price(price: Decimal) -> String {
    format!("{:.2}", price.round_dp(2))
}

/// 将金额转换为万元
pub fn to_wan(amount: Decimal) -> Decimal {
    amount / Decimal::from(10000)
}

/// 将金额转换为亿元
pub fn to_yi(amount: Decimal) -> Decimal {
    amount / Decimal::from(100_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount(dec!(1234567.89)), "1,234,567.89");
        assert_eq!(format_amount(dec!(100)), "100.00");
    }

    #[test]
    fn test_format_percent() {
        assert_eq!(format_percent(dec!(12.345)), "12.35%");
    }
}
