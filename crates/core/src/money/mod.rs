//! Money in integer centimes. Every fiscal rule in `docs/features.md`
//! has a constant here, a fixture under `fixtures/money/` and a doc row;
//! the three move together. No `f64` on any path that reaches a total.
#![deny(clippy::arithmetic_side_effects)]

use serde::{Deserialize, Serialize};

pub mod stamp;
pub mod totals;

pub use stamp::{stamp, PaymentMode};
pub use totals::{compute_totals, Line, Regime, Totals, TotalsOptions, TvaLine};

/// Basis points in one whole: 10 000 bps = 100 %.
pub const BPS_PER_WHOLE: i64 = 10_000;

/// A whole amount of centimes. `Money::centimes(1250)` is 12,50 DA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(i64);

/// A rate in basis points: `Bps::new(1900)` is 19 %.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Bps(u32);

impl From<Bps> for u32 {
    fn from(b: Bps) -> u32 {
        b.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    #[error("amount overflows i64 centimes")]
    Overflow,
    #[error("rate above 10 000 basis points (100 %)")]
    RateOutOfRange,
    #[error("a line quantity is below zero")]
    NegativeQuantity,
    #[error("a line unit price is below zero")]
    NegativeUnitPrice,
    #[error("a discount is below zero")]
    NegativeDiscount,
    #[error("a line discount is above its line total")]
    LineDiscountAboveLine,
    #[error("the global discount is above the total HT")]
    GlobalDiscountAboveTotal,
}

impl Money {
    pub const ZERO: Money = Money(0);

    pub const fn centimes(v: i64) -> Self {
        Money(v)
    }

    pub const fn as_centimes(self) -> i64 {
        self.0
    }

    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    pub fn checked_add(self, other: Money) -> Result<Money, MoneyError> {
        self.0
            .checked_add(other.0)
            .map(Money)
            .ok_or(MoneyError::Overflow)
    }

    pub fn checked_sub(self, other: Money) -> Result<Money, MoneyError> {
        self.0
            .checked_sub(other.0)
            .map(Money)
            .ok_or(MoneyError::Overflow)
    }

    /// Unit amount times a quantity (a count, never a percentage).
    pub fn checked_mul(self, qty: i64) -> Result<Money, MoneyError> {
        self.0
            .checked_mul(qty)
            .map(Money)
            .ok_or(MoneyError::Overflow)
    }

    /// `self × rate`, rounded once to the centime, half away from zero.
    /// Fixture `tva_rounding_once_per_rate`.
    pub fn pct(self, rate: Bps) -> Result<Money, MoneyError> {
        let raw = self
            .0
            .checked_mul(i64::from(rate.0))
            .ok_or(MoneyError::Overflow)?;
        let magnitude = raw.checked_abs().ok_or(MoneyError::Overflow)?;
        // Division by a non-zero constant cannot panic; the add is checked.
        let half_up = magnitude
            .checked_add(BPS_PER_WHOLE / 2)
            .ok_or(MoneyError::Overflow)?
            / BPS_PER_WHOLE;
        let signed = if raw < 0 {
            half_up.checked_neg().ok_or(MoneyError::Overflow)?
        } else {
            half_up
        };
        Ok(Money(signed))
    }
}

impl Bps {
    /// A rate is at most one whole; 19 typed as 190 000 must not become
    /// 1 900 % TVA (fixture `tva_rounding_once_per_rate`, invalid rates).
    pub fn new(v: u32) -> Result<Self, MoneyError> {
        Bps::try_from(v)
    }

    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for Bps {
    type Error = MoneyError;

    fn try_from(v: u32) -> Result<Self, MoneyError> {
        if i64::from(v) > BPS_PER_WHOLE {
            Err(MoneyError::RateOutOfRange)
        } else {
            Ok(Bps(v))
        }
    }
}
