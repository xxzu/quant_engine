//! 日期时间工具函数

use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

/// 判断是否交易日 (简单判断：非周末)
pub fn is_trading_day(date: NaiveDate) -> bool {
    let weekday = date.weekday();
    weekday != Weekday::Sat && weekday != Weekday::Sun
}

/// 判断是否在交易时段
/// A股交易时段: 9:30-11:30, 13:00-15:00
pub fn is_trading_hours(datetime: NaiveDateTime) -> bool {
    let time = datetime.time();
    
    let morning_start = NaiveTime::from_hms_opt(9, 30, 0).unwrap();
    let morning_end = NaiveTime::from_hms_opt(11, 30, 0).unwrap();
    let afternoon_start = NaiveTime::from_hms_opt(13, 0, 0).unwrap();
    let afternoon_end = NaiveTime::from_hms_opt(15, 0, 0).unwrap();

    (time >= morning_start && time <= morning_end)
        || (time >= afternoon_start && time <= afternoon_end)
}

/// 获取当前交易日
pub fn current_trading_day() -> NaiveDate {
    let today = Local::now().date_naive();
    if is_trading_day(today) {
        today
    } else {
        next_trading_day(today)
    }
}

/// 获取下一个交易日
pub fn next_trading_day(date: NaiveDate) -> NaiveDate {
    let mut next = date.succ_opt().unwrap();
    while !is_trading_day(next) {
        next = next.succ_opt().unwrap();
    }
    next
}

/// 获取上一个交易日
pub fn prev_trading_day(date: NaiveDate) -> NaiveDate {
    let mut prev = date.pred_opt().unwrap();
    while !is_trading_day(prev) {
        prev = prev.pred_opt().unwrap();
    }
    prev
}

/// 解析日期字符串
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    // 支持多种格式
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%Y%m%d"))
        .ok()
}

/// 格式化日期
pub fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_trading_day() {
        // 2024-01-15 是周一
        let monday = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        assert!(is_trading_day(monday));

        // 2024-01-14 是周日
        let sunday = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap();
        assert!(!is_trading_day(sunday));
    }

    #[test]
    fn test_parse_date() {
        assert_eq!(
            parse_date("2024-01-15"),
            Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
        );
        assert_eq!(
            parse_date("20240115"),
            Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
        );
    }
}
