use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

/// Standard ISO-4217 Currency Code (e.g., "USD", "INR", "EUR")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct Currency(pub String);

impl Currency {
    pub fn usd() -> Self {
        Self("USD".to_string())
    }

    pub fn inr() -> Self {
        Self("INR".to_string())
    }

    pub fn eur() -> Self {
        Self("EUR".to_string())
    }

    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into().to_uppercase())
    }

    pub fn code(&self) -> &str {
        &self.0
    }
}

impl Default for Currency {
    fn default() -> Self {
        Self::usd()
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Precision-safe monetary value, preventing floating-point rounding errors.
///
/// # Example
/// ```
/// use obaf_core::money::Money;
/// use rust_decimal::Decimal;
///
/// let price = Money::from_i64(100);
/// assert_eq!(price.amount, Decimal::from(100));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Money {
    #[schema(value_type = String, example = "199.50")]
    pub amount: Decimal,
}

impl Money {
    pub fn new(amount: Decimal) -> Self {
        Self { amount }
    }

    pub fn zero() -> Self {
        Self {
            amount: Decimal::ZERO,
        }
    }

    pub fn from_i64(amount: i64) -> Self {
        Self {
            amount: Decimal::from(amount),
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.amount)
    }
}
