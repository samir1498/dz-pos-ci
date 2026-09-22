//! Droit de timbre, the row `stamp_progressive_tranches` in
//! `docs/features.md`. Cash only; a floor at 300,00 DA; the amount is cut
//! into 100,00 DA tranches, rounded up, and every tranche is charged at the
//! band of the whole amount. Fixture `stamp_progressive_tranches.json`.

use super::{Money, MoneyError};
use serde::{Deserialize, Serialize};

/// Nothing is due at or below this amount.
pub const STAMP_FLOOR: Money = Money::centimes(30_000);
/// One tranche, 100,00 DA. The count is rounded up.
pub const STAMP_TRANCHE: i64 = 10_000;
/// Highest amount charged at 1,00 DA per tranche: 30 000,00 DA.
pub const STAMP_BAND_LOW: Money = Money::centimes(3_000_000);
/// Highest amount charged at 1,50 DA per tranche: 100 000,00 DA.
pub const STAMP_BAND_MID: Money = Money::centimes(10_000_000);
/// Per-tranche rate in the low band, 1,00 DA.
pub const STAMP_RATE_LOW: i64 = 100;
/// Per-tranche rate in the middle band, 1,50 DA.
pub const STAMP_RATE_MID: i64 = 150;
/// Per-tranche rate above the middle band, 2,00 DA.
pub const STAMP_RATE_HIGH: i64 = 200;
/// Nothing due is charged below this once anything is due, 5,00 DA.
pub const STAMP_MIN: Money = Money::centimes(500);

/// How the document is paid. Only cash carries the stamp; card is the
/// electronic exemption and credit carries no stamp at issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentMode {
    Cash,
    Card,
    Credit,
}

impl PaymentMode {
    pub const fn carries_stamp(self) -> bool {
        matches!(self, PaymentMode::Cash)
    }
}

/// Droit de timbre on `total_ttc`. Zero for card and credit, zero at or
/// under 300,00 DA, otherwise `ceil(total_ttc / 100 DA)` tranches at the
/// band rate of the whole amount, never below 5,00 DA, never capped.
pub fn stamp(total_ttc: Money, mode: PaymentMode) -> Result<Money, MoneyError> {
    if !mode.carries_stamp() || total_ttc <= STAMP_FLOOR {
        return Ok(Money::ZERO);
    }
    let amount = total_ttc.as_centimes();
    // ceil(amount / STAMP_TRANCHE) on a strictly positive amount. Division
    // by a non-zero constant cannot panic; the add is checked.
    let tranches = amount
        .checked_add(STAMP_TRANCHE.saturating_sub(1))
        .ok_or(MoneyError::Overflow)?
        / STAMP_TRANCHE;
    let rate = if total_ttc <= STAMP_BAND_LOW {
        STAMP_RATE_LOW
    } else if total_ttc <= STAMP_BAND_MID {
        STAMP_RATE_MID
    } else {
        STAMP_RATE_HIGH
    };
    let due = tranches
        .checked_mul(rate)
        .map(Money::centimes)
        .ok_or(MoneyError::Overflow)?;
    Ok(due.max(STAMP_MIN))
}
