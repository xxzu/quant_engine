//! 绩效分析

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

/// 绩效指标
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// 年化收益率 (%)
    pub annual_return: Decimal,
    /// 夏普比率
    pub sharpe_ratio: Decimal,
    /// 最大回撤 (%)
    pub max_drawdown: Decimal,
    /// 最大回撤持续天数
    pub max_drawdown_days: u32,
    /// 卡玛比率 (年化收益/最大回撤)
    pub calmar_ratio: Decimal,
    /// 索提诺比率
    pub sortino_ratio: Decimal,
    /// 胜率 (%)
    pub win_rate: Decimal,
    /// 盈亏比
    pub profit_factor: Decimal,
}

impl PerformanceMetrics {
    /// 计算绩效指标
    pub fn calculate(equity_curve: &[Decimal], initial_capital: Decimal) -> Self {
        if equity_curve.is_empty() {
            return Self::default();
        }

        let final_equity = *equity_curve.last().unwrap_or(&initial_capital);
        let trading_days = equity_curve.len() as f64;

        // 总收益率
        let total_return = if !initial_capital.is_zero() {
            ((final_equity - initial_capital) / initial_capital).to_f64().unwrap_or(0.0)
        } else {
            0.0
        };

        // 年化收益率 (假设一年 252 个交易日)
        let annual_return = if trading_days > 0.0 {
            let years = trading_days / 252.0;
            ((1.0 + total_return).powf(1.0 / years) - 1.0) * 100.0
        } else {
            0.0
        };

        // 计算日收益率
        let mut daily_returns: Vec<f64> = Vec::new();
        for i in 1..equity_curve.len() {
            let prev = equity_curve[i - 1].to_f64().unwrap_or(1.0);
            let curr = equity_curve[i].to_f64().unwrap_or(1.0);
            if prev > 0.0 {
                daily_returns.push((curr - prev) / prev);
            }
        }

        // 计算标准差
        let mean_return = if !daily_returns.is_empty() {
            daily_returns.iter().sum::<f64>() / daily_returns.len() as f64
        } else {
            0.0
        };

        let variance = if daily_returns.len() > 1 {
            daily_returns
                .iter()
                .map(|r| (r - mean_return).powi(2))
                .sum::<f64>()
                / (daily_returns.len() - 1) as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();

        // 夏普比率 (假设无风险利率 3%)
        let risk_free_rate = 0.03 / 252.0;
        let sharpe = if std_dev > 0.0 {
            ((mean_return - risk_free_rate) / std_dev) * (252.0_f64).sqrt()
        } else {
            0.0
        };

        // 最大回撤
        let (max_drawdown, max_drawdown_days) = calculate_max_drawdown(equity_curve);

        // 卡玛比率
        let calmar = if max_drawdown > 0.0 {
            annual_return / max_drawdown
        } else {
            0.0
        };

        // 索提诺比率 (只考虑下行波动)
        let downside_returns: Vec<f64> = daily_returns
            .iter()
            .filter(|&&r| r < 0.0)
            .copied()
            .collect();

        let downside_variance = if downside_returns.len() > 1 {
            downside_returns
                .iter()
                .map(|r| r.powi(2))
                .sum::<f64>()
                / downside_returns.len() as f64
        } else {
            0.0
        };
        let downside_std = downside_variance.sqrt();

        let sortino = if downside_std > 0.0 {
            ((mean_return - risk_free_rate) / downside_std) * (252.0_f64).sqrt()
        } else {
            0.0
        };

        Self {
            annual_return: Decimal::from_f64_retain(annual_return)
                .unwrap_or(Decimal::ZERO)
                .round_dp(2),
            sharpe_ratio: Decimal::from_f64_retain(sharpe)
                .unwrap_or(Decimal::ZERO)
                .round_dp(2),
            max_drawdown: Decimal::from_f64_retain(max_drawdown)
                .unwrap_or(Decimal::ZERO)
                .round_dp(2),
            max_drawdown_days,
            calmar_ratio: Decimal::from_f64_retain(calmar)
                .unwrap_or(Decimal::ZERO)
                .round_dp(2),
            sortino_ratio: Decimal::from_f64_retain(sortino)
                .unwrap_or(Decimal::ZERO)
                .round_dp(2),
            win_rate: Decimal::ZERO,
            profit_factor: Decimal::ZERO,
        }
    }
}

/// 计算最大回撤
fn calculate_max_drawdown(equity_curve: &[Decimal]) -> (f64, u32) {
    let mut max_drawdown = 0.0;
    let mut peak = equity_curve.first().copied().unwrap_or(Decimal::ONE);
    let mut max_days = 0u32;
    let mut current_days = 0u32;

    for &equity in equity_curve {
        if equity > peak {
            peak = equity;
            current_days = 0;
        } else {
            current_days += 1;
            let drawdown = ((peak - equity) / peak).to_f64().unwrap_or(0.0) * 100.0;
            if drawdown > max_drawdown {
                max_drawdown = drawdown;
                max_days = current_days;
            }
        }
    }

    (max_drawdown, max_days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_drawdown() {
        let curve = vec![
            Decimal::from(100),
            Decimal::from(110),
            Decimal::from(105),
            Decimal::from(90),
            Decimal::from(95),
            Decimal::from(120),
        ];

        let (dd, _days) = calculate_max_drawdown(&curve);
        // 最大回撤从 110 到 90 = (110-90)/110 = 18.18%
        assert!(dd > 18.0 && dd < 19.0);
    }
}
