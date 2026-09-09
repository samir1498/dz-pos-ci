//! Document totals: the columns of the totals table in `docs/features.md`
//! §3. TVA is rounded once per rate group on the group's discounted HT
//! base, never per line and never again at the total. The global discount
//! is spread by the row `discount_spread_largest_remainder`. Fixture
//! `tva_rounding_once_per_rate.json`, arrays `totals_cases` and
//! `totals_error_cases`.

use super::{stamp::stamp, Bps, Money, MoneyError, PaymentMode};
use serde::{Deserialize, Serialize};

/// One document line. `qty` is a count, `unit_price` is HT under the réel
/// regime and the single price under the IFU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    pub qty: i64,
    pub unit_price: Money,
    pub line_discount: Money,
    #[serde(rename = "rate_bps")]
    pub rate: Bps,
}

/// The shop's régime fiscal at issue time. Under the IFU a document shows
/// no TVA at all (`regime_ifu_prints_no_tva`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Regime {
    Reel,
    Ifu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TotalsOptions {
    pub global_discount: Money,
    pub payment_mode: PaymentMode,
    /// The shop setting. The stamp still needs a cash payment to be due.
    pub stamp_enabled: bool,
    pub regime: Regime,
}

/// One row of the TVA recap: the rate, the HT base it applies to after the
/// global discount, and the tax rounded once on that base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TvaLine {
    #[serde(rename = "rate_bps")]
    pub rate: Bps,
    pub base: Money,
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Totals {
    pub total_ht: Money,
    pub discount: Money,
    pub subtotal_ht: Money,
    /// One row per rate present in the lines, by rising rate. Empty under
    /// the IFU regime.
    pub tva_by_rate: Vec<TvaLine>,
    pub tva: Money,
    pub total_ttc: Money,
    pub stamp: Money,
    pub net_to_pay: Money,
}

/// HT per rate group, by rising rate, and the sum of the groups.
fn group_by_rate(lines: &[Line]) -> Result<(Vec<(Bps, Money)>, Money), MoneyError> {
    let mut groups: Vec<(Bps, Money)> = Vec::new();
    let mut total_ht = Money::ZERO;
    for line in lines {
        if line.qty < 0 {
            return Err(MoneyError::NegativeQuantity);
        }
        if line.unit_price.is_negative() {
            return Err(MoneyError::NegativeUnitPrice);
        }
        if line.line_discount.is_negative() {
            return Err(MoneyError::NegativeDiscount);
        }
        let gross = line.unit_price.checked_mul(line.qty)?;
        if line.line_discount > gross {
            return Err(MoneyError::LineDiscountAboveLine);
        }
        let net = gross.checked_sub(line.line_discount)?;
        total_ht = total_ht.checked_add(net)?;
        match groups.iter_mut().find(|(rate, _)| *rate == line.rate) {
            Some((_, ht)) => *ht = ht.checked_add(net)?,
            None => groups.push((line.rate, net)),
        }
    }
    groups.sort_by_key(|(rate, _)| *rate);
    Ok((groups, total_ht))
}

/// The global discount each group carries: its proportional share rounded
/// down, with the leftover centimes all going to the group with the largest
/// HT subtotal, the lower rate winning a tie, and never past what the group
/// has left (`discount_spread_largest_remainder`, near-total case). Every
/// group HT is at or above zero here, so truncating division is a floor.
fn spread_discount(
    groups: &[(Bps, Money)],
    total_ht: Money,
    discount: Money,
) -> Result<Vec<Money>, MoneyError> {
    let mut shares = vec![Money::ZERO; groups.len()];
    if discount == Money::ZERO || groups.is_empty() {
        return Ok(shares);
    }
    // discount > 0 and discount <= total_ht, so total_ht is above zero.
    let total = i128::from(total_ht.as_centimes());
    let mut allocated = Money::ZERO;
    for (i, (_, ht)) in groups.iter().enumerate() {
        // i128 keeps the product exact; two i64 factors can overflow i64.
        let product = i128::from(discount.as_centimes())
            .checked_mul(i128::from(ht.as_centimes()))
            .ok_or(MoneyError::Overflow)?;
        let share = product.checked_div(total).ok_or(MoneyError::Overflow)?;
        let share = i64::try_from(share)
            .map(Money::centimes)
            .map_err(|_| MoneyError::Overflow)?;
        shares[i] = share;
        allocated = allocated.checked_add(share)?;
    }
    let mut remainder = discount.checked_sub(allocated)?;
    // The leftover centimes go to the largest HT group, but a share never
    // exceeds its group's HT: when the discount leaves fewer centimes than
    // there are groups the largest one may have no room, and what it cannot
    // take rolls to the next largest. Groups are sorted by rising rate and
    // the sort is stable, so the lower rate wins a tie.
    let mut by_size: Vec<usize> = (0..groups.len()).collect();
    by_size.sort_by(|a, b| groups[*b].1.cmp(&groups[*a].1));
    for i in by_size {
        if remainder == Money::ZERO {
            break;
        }
        let room = groups[i].1.checked_sub(shares[i])?;
        let taken = if room < remainder { room } else { remainder };
        shares[i] = shares[i].checked_add(taken)?;
        remainder = remainder.checked_sub(taken)?;
    }
    Ok(shares)
}

/// Every column of the totals table for one document.
pub fn compute_totals(lines: &[Line], opts: &TotalsOptions) -> Result<Totals, MoneyError> {
    let (groups, total_ht) = group_by_rate(lines)?;
    let discount = opts.global_discount;
    if discount.is_negative() {
        return Err(MoneyError::NegativeDiscount);
    }
    if discount > total_ht {
        return Err(MoneyError::GlobalDiscountAboveTotal);
    }
    let subtotal_ht = total_ht.checked_sub(discount)?;

    let mut tva_by_rate = Vec::new();
    let mut tva = Money::ZERO;
    if opts.regime == Regime::Reel {
        let shares = spread_discount(&groups, total_ht, discount)?;
        for ((rate, ht), share) in groups.iter().zip(shares) {
            let base = ht.checked_sub(share)?;
            let amount = base.pct(*rate)?;
            tva = tva.checked_add(amount)?;
            tva_by_rate.push(TvaLine {
                rate: *rate,
                base,
                amount,
            });
        }
    }

    let total_ttc = subtotal_ht.checked_add(tva)?;
    let stamp = if opts.stamp_enabled {
        stamp(total_ttc, opts.payment_mode)?
    } else {
        Money::ZERO
    };
    let net_to_pay = total_ttc.checked_add(stamp)?;
    Ok(Totals {
        total_ht,
        discount,
        subtotal_ht,
        tva_by_rate,
        tva,
        total_ttc,
        stamp,
        net_to_pay,
    })
}
