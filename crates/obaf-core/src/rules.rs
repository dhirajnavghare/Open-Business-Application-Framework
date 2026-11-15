use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The condition determining whether a pricing rule triggers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "type", content = "value")]
pub enum RuleCondition {
    /// Always applies unconditionally.
    Always,
    /// Triggers if order subtotal is at least this amount.
    MinSubtotal(Decimal),
    /// Triggers if the provided promo coupon code matches.
    PromoCode(String),
}

/// The economic action performed when a rule triggers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "type", content = "value")]
pub enum RuleAction {
    /// Percentage discount applied to subtotal (e.g. 0.10 for 10% off).
    PercentageDiscount(Decimal),
    /// Flat fixed currency discount deducted from subtotal.
    FlatDiscount(Decimal),
    /// Percentage surcharge added to subtotal (e.g. 0.05 for 5% service fee).
    PercentageSurcharge(Decimal),
    /// Flat fixed surcharge added to subtotal.
    FlatSurcharge(Decimal),
}

/// Declarative business rule for pricing, promotions, and dynamic adjustments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PricingRule {
    pub id: String,
    pub name: String,
    pub condition: RuleCondition,
    pub action: RuleAction,
}

/// A line-item breakdown adjustment resulting from an evaluated rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct RuleAdjustment {
    pub rule_id: String,
    pub rule_name: String,
    /// Negative for discounts, positive for surcharges.
    pub amount: Decimal,
}

/// Final summary outcome of the pricing rules engine evaluation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct RuleEvaluationResult {
    pub original_subtotal: Decimal,
    pub total_adjustments: Decimal,
    pub adjusted_subtotal: Decimal,
    pub applied_rules: Vec<RuleAdjustment>,
}

/// Evaluates a list of business rules against a subtotal and optional promo code.
///
/// # Example
/// ```
/// use obaf_core::rules::{evaluate_pricing_rules, PricingRule, RuleCondition, RuleAction};
/// use rust_decimal::Decimal;
///
/// let rules = vec![
///     PricingRule {
///         id: "save10".into(),
///         name: "10% off".into(),
///         condition: RuleCondition::MinSubtotal(Decimal::from(50)),
///         action: RuleAction::PercentageDiscount(Decimal::new(1, 1)), // 0.10
///     }
/// ];
/// let result = evaluate_pricing_rules(&rules, Decimal::from(100), None);
/// assert_eq!(result.adjusted_subtotal, Decimal::from(90));
/// ```
pub fn evaluate_pricing_rules(
    rules: &[PricingRule],
    subtotal: Decimal,
    promo_code: Option<&str>,
) -> RuleEvaluationResult {
    let mut total_adjustments = Decimal::ZERO;
    let mut applied_rules = Vec::new();

    for rule in rules {
        let condition_met = match &rule.condition {
            RuleCondition::Always => true,
            RuleCondition::MinSubtotal(min_val) => subtotal >= *min_val,
            RuleCondition::PromoCode(expected_code) => {
                if let Some(code) = promo_code {
                    code.trim().eq_ignore_ascii_case(expected_code.trim())
                } else {
                    false
                }
            }
        };

        if condition_met {
            let amount = match &rule.action {
                RuleAction::PercentageDiscount(pct) => {
                    let discount = (subtotal * *pct).round_dp(2);
                    -discount
                }
                RuleAction::FlatDiscount(flat) => {
                    let discount = (*flat).min(subtotal).round_dp(2);
                    -discount
                }
                RuleAction::PercentageSurcharge(pct) => (subtotal * *pct).round_dp(2),
                RuleAction::FlatSurcharge(flat) => flat.round_dp(2),
            };

            total_adjustments += amount;
            applied_rules.push(RuleAdjustment {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                amount,
            });
        }
    }

    let adjusted_subtotal = (subtotal + total_adjustments).max(Decimal::ZERO);

    RuleEvaluationResult {
        original_subtotal: subtotal,
        total_adjustments,
        adjusted_subtotal,
        applied_rules,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_rules_engine_discount_and_promo() {
        let rules = vec![
            PricingRule {
                id: "min_100_tier".into(),
                name: "10% off orders over 100".into(),
                condition: RuleCondition::MinSubtotal(Decimal::from(100)),
                action: RuleAction::PercentageDiscount(Decimal::from_str("0.10").unwrap()),
            },
            PricingRule {
                id: "vip_code".into(),
                name: "Flat 25 off VIP".into(),
                condition: RuleCondition::PromoCode("VIP25".into()),
                action: RuleAction::FlatDiscount(Decimal::from(25)),
            },
        ];

        let result = evaluate_pricing_rules(&rules, Decimal::from(200), Some("vip25"));
        assert_eq!(result.applied_rules.len(), 2);
        // 10% of 200 = -20, VIP25 = -25 -> total adjustments = -45
        assert_eq!(result.total_adjustments, Decimal::from(-45));
        assert_eq!(result.adjusted_subtotal, Decimal::from(155));
    }
}
