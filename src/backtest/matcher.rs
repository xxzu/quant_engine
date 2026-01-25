//! 撮合模拟器

use crate::trading::order::{Order, OrderSide, OrderStatus, OrderType};
use rust_decimal::Decimal;

/// 撮合结果
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// 是否成交
    pub matched: bool,
    /// 成交价格
    pub price: Decimal,
    /// 成交数量
    pub quantity: i64,
    /// 原因
    pub reason: String,
}

/// 撮合器
pub struct Matcher {
    /// 滑点 (%)
    pub slippage: Decimal,
    /// 成交比例 (模拟部分成交)
    pub fill_ratio: Decimal,
}

impl Default for Matcher {
    fn default() -> Self {
        Self {
            slippage: Decimal::ZERO,
            fill_ratio: Decimal::ONE,
        }
    }
}

impl Matcher {
    /// 创建撮合器
    pub fn new(slippage: Decimal) -> Self {
        Self {
            slippage,
            fill_ratio: Decimal::ONE,
        }
    }

    /// 撮合订单
    pub fn match_order(
        &self,
        order: &Order,
        current_price: Decimal,
        high: Decimal,
        low: Decimal,
    ) -> MatchResult {
        if order.status != OrderStatus::Pending && order.status != OrderStatus::Submitted {
            return MatchResult {
                matched: false,
                price: Decimal::ZERO,
                quantity: 0,
                reason: "订单状态不可撮合".to_string(),
            };
        }

        match order.order_type {
            OrderType::Market => {
                // 市价单：按当前价 + 滑点成交
                let price = self.apply_slippage(current_price, order.side);
                MatchResult {
                    matched: true,
                    price,
                    quantity: self.calculate_fill_quantity(order.quantity),
                    reason: "市价成交".to_string(),
                }
            }
            OrderType::Limit => {
                // 限价单：判断是否触及限价
                if let Some(limit_price) = order.price {
                    match order.side {
                        OrderSide::Buy => {
                            if low <= limit_price {
                                MatchResult {
                                    matched: true,
                                    price: limit_price,
                                    quantity: self.calculate_fill_quantity(order.quantity),
                                    reason: "限价买入成交".to_string(),
                                }
                            } else {
                                MatchResult {
                                    matched: false,
                                    price: Decimal::ZERO,
                                    quantity: 0,
                                    reason: format!("未触及限价 {} (最低 {})", limit_price, low),
                                }
                            }
                        }
                        OrderSide::Sell => {
                            if high >= limit_price {
                                MatchResult {
                                    matched: true,
                                    price: limit_price,
                                    quantity: self.calculate_fill_quantity(order.quantity),
                                    reason: "限价卖出成交".to_string(),
                                }
                            } else {
                                MatchResult {
                                    matched: false,
                                    price: Decimal::ZERO,
                                    quantity: 0,
                                    reason: format!("未触及限价 {} (最高 {})", limit_price, high),
                                }
                            }
                        }
                    }
                } else {
                    MatchResult {
                        matched: false,
                        price: Decimal::ZERO,
                        quantity: 0,
                        reason: "限价单缺少价格".to_string(),
                    }
                }
            }
            OrderType::Stop => {
                // 止损单
                if let Some(stop_price) = order.price {
                    match order.side {
                        OrderSide::Sell => {
                            if low <= stop_price {
                                MatchResult {
                                    matched: true,
                                    price: self.apply_slippage(stop_price, order.side),
                                    quantity: self.calculate_fill_quantity(order.quantity),
                                    reason: "止损卖出成交".to_string(),
                                }
                            } else {
                                MatchResult {
                                    matched: false,
                                    price: Decimal::ZERO,
                                    quantity: 0,
                                    reason: "未触发止损".to_string(),
                                }
                            }
                        }
                        OrderSide::Buy => {
                            if high >= stop_price {
                                MatchResult {
                                    matched: true,
                                    price: self.apply_slippage(stop_price, order.side),
                                    quantity: self.calculate_fill_quantity(order.quantity),
                                    reason: "止损买入成交".to_string(),
                                }
                            } else {
                                MatchResult {
                                    matched: false,
                                    price: Decimal::ZERO,
                                    quantity: 0,
                                    reason: "未触发止损".to_string(),
                                }
                            }
                        }
                    }
                } else {
                    MatchResult {
                        matched: false,
                        price: Decimal::ZERO,
                        quantity: 0,
                        reason: "止损单缺少价格".to_string(),
                    }
                }
            }
            OrderType::TakeProfit => {
                // 止盈单
                if let Some(tp_price) = order.price {
                    match order.side {
                        OrderSide::Sell => {
                            if high >= tp_price {
                                MatchResult {
                                    matched: true,
                                    price: tp_price,
                                    quantity: self.calculate_fill_quantity(order.quantity),
                                    reason: "止盈卖出成交".to_string(),
                                }
                            } else {
                                MatchResult {
                                    matched: false,
                                    price: Decimal::ZERO,
                                    quantity: 0,
                                    reason: "未触发止盈".to_string(),
                                }
                            }
                        }
                        OrderSide::Buy => {
                            if low <= tp_price {
                                MatchResult {
                                    matched: true,
                                    price: tp_price,
                                    quantity: self.calculate_fill_quantity(order.quantity),
                                    reason: "止盈买入成交".to_string(),
                                }
                            } else {
                                MatchResult {
                                    matched: false,
                                    price: Decimal::ZERO,
                                    quantity: 0,
                                    reason: "未触发止盈".to_string(),
                                }
                            }
                        }
                    }
                } else {
                    MatchResult {
                        matched: false,
                        price: Decimal::ZERO,
                        quantity: 0,
                        reason: "止盈单缺少价格".to_string(),
                    }
                }
            }
        }
    }

    /// 应用滑点
    fn apply_slippage(&self, price: Decimal, side: OrderSide) -> Decimal {
        match side {
            OrderSide::Buy => price * (Decimal::ONE + self.slippage / Decimal::from(100)),
            OrderSide::Sell => price * (Decimal::ONE - self.slippage / Decimal::from(100)),
        }
    }

    /// 计算成交数量
    fn calculate_fill_quantity(&self, quantity: i64) -> i64 {
        (Decimal::from(quantity) * self.fill_ratio)
            .floor()
            .to_string()
            .parse()
            .unwrap_or(quantity)
    }
}
