//! Decimal 工具函数

use rust_decimal::Decimal;

/// 格式化金额显示 (保留2位小数)
pub fn format_amount(amount: Decimal) -> String {
    format!("{:.2}", amount.round_dp(2))
}

/// 格式化百分比显示
pub fn format_percent(value: Decimal) -> String {
    format!("{:.2}%", value.round_dp(2))
}

/// 格式化价格 (保留对应精度)
pub fn format_price(price: Decimal, precision: u32) -> String {
    format!("{}", price.round_dp(precision))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount(dec!(1234.5678)), "1234.57");
    }

    #[test]
    fn test_format_percent() {
        assert_eq!(format_percent(dec!(12.345)), "12.34%");
    }
}
