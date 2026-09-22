//! What one slice of a facture is worth: the totals of a partial avoir and
//! the shares of remise that come back with it.
//!
//! Split off `avoir` because it is a different question. That module decides
//! which lines are coming back and what happens to the account; this one is
//! the arithmetic on the lines once they are chosen, and it never reads or
//! writes a row. Every figure here is a `Money` in centimes and every step is
//! checked (rule 1).
//!
//! It reads `Remaining` and never produces one, so the edge points one way:
//! `avoir -> avoir_slice -> avoir_remaining`.

use crate::error::CoreError;
use crate::money::{Bps, Money, MoneyError, Regime, Totals, TvaLine};
use crate::services::avoir_remaining::{Remaining, SliceLine};
use crate::services::documents::Document;

/// The totals of a partial avoir: its own lines, taxed as the part they are,
/// and held at every rate to what the facture has left there.
///
/// The slice's own arithmetic is the rule (features.md §3): a partial avoir is
/// taxed as the goods it credits and not as a share of the facture's tax, so
/// its base is its own HT and its tax is rounded once on that base. The cap is
/// the other rule, the one the running total on `total_ttc` already states,
/// read one column further in: no avoir gives back more base, more TVA or more
/// remise at a rate than the facture still has at that rate.
///
/// Without the cap the closing avoir, which is what is left of the facture,
/// goes negative. Two partials of a facture at two rates can round their tax
/// to a centime more than the facture charged at one of those rates, or take a
/// rate group's whole HT while the centime of remise the facture put on that
/// group stays behind, and the credit note that closes the facture is then
/// asked for a base or a tax below zero at a rate whose goods have all come
/// back. The cap spends the difference on the partial that caused it, where it
/// is one centime of rounding on a paper that is already rounding, rather than
/// leaving it for a document that cannot carry it at all (`avoir_prop`).
///
/// The droit de timbre is never on an avoir, so the stamp is zero and the net
/// is the TTC.
pub(crate) fn slice_totals(
    facture: &Document,
    remaining: &Remaining,
    lines: &[SliceLine],
) -> Result<Totals, CoreError> {
    let (groups, total_ht) = grouped(lines)?;

    let mut tva_by_rate = Vec::new();
    let mut discount = Money::ZERO;
    let mut tva = Money::ZERO;
    // Under the IFU a document shows no TVA at all, so there is no recap to
    // read a rate's remise off and nothing to hold at a rate either. The
    // remise is taken on the whole slice instead, the way it is spread by
    // `compute_totals`.
    if facture.regime != Regime::Reel {
        let share = global_discount(facture, remaining, total_ht)?.min(total_ht);
        let subtotal_ht = total_ht.checked_sub(share)?;
        return Ok(Totals {
            total_ht,
            discount: share,
            subtotal_ht,
            tva_by_rate,
            tva,
            total_ttc: subtotal_ht,
            stamp: Money::ZERO,
            net_to_pay: subtotal_ht,
        });
    }
    for (rate, ht) in &groups {
        // Every rate of the slice is a rate one of the facture's lines is at,
        // and `Remaining` carries a row for each of those, so a rate with no
        // row is a rate the facture never charged and has nothing left at.
        let (left_ht, left_share, left_tva) = match remaining.at(*rate) {
            Some(left) => (left.ht, left.ht.checked_sub(left.base)?, left.amount),
            None => (Money::ZERO, Money::ZERO, Money::ZERO),
        };
        // What this slice gives back of the facture's remise at this rate: its
        // proportional share rounded up, so a slice never leaves the rest of
        // the facture holding a remise it cannot place, and never more than
        // what is left.
        let share = if *ht >= left_ht {
            left_share.min(*ht)
        } else {
            up(left_share, *ht, left_ht)?.min(left_share)
        };
        let base = ht.checked_sub(share)?;
        let amount = base.pct(*rate)?.min(left_tva);
        discount = discount.checked_add(share)?;
        tva = tva.checked_add(amount)?;
        tva_by_rate.push(TvaLine {
            rate: *rate,
            base,
            amount,
        });
    }

    let subtotal_ht = total_ht.checked_sub(discount)?;
    let total_ttc = subtotal_ht.checked_add(tva)?;
    Ok(Totals {
        total_ht,
        discount,
        subtotal_ht,
        tva_by_rate,
        tva,
        total_ttc,
        stamp: Money::ZERO,
        net_to_pay: total_ttc,
    })
}

/// The share of the facture's global remise that comes back with these lines,
/// for a document with no TVA recap to spread it over: the credited HT against
/// what the facture has left, rounded up, and never more than the remise the
/// earlier avoirs have not taken.
///
/// Against what is left rather than against the whole, so the shares of the
/// successive avoirs come to the remise exactly and the last one takes
/// whatever the divisions left. Rounded up so that a slice never leaves the
/// rest of the facture holding a remise it has no HT left to put it on.
fn global_discount(
    facture: &Document,
    remaining: &Remaining,
    credited_ht: Money,
) -> Result<Money, CoreError> {
    if facture.totals.discount == Money::ZERO || facture.totals.total_ht == Money::ZERO {
        return Ok(Money::ZERO);
    }
    let left_discount = remaining.totals.discount;
    let left_ht = remaining.totals.total_ht;
    if left_discount <= Money::ZERO || left_ht <= Money::ZERO {
        return Ok(Money::ZERO);
    }
    if credited_ht >= left_ht {
        return Ok(left_discount);
    }
    Ok(up(left_discount, credited_ht, left_ht)?.min(left_discount))
}

/// HT per rate group of the lines coming back, by rising rate, and their sum:
/// the grouping the money functions do, on the slice's own lines.
fn grouped(lines: &[SliceLine]) -> Result<(Vec<(Bps, Money)>, Money), CoreError> {
    let mut groups: Vec<(Bps, Money)> = Vec::new();
    let mut total_ht = Money::ZERO;
    for line in lines {
        total_ht = total_ht.checked_add(line.ht)?;
        match groups.iter_mut().find(|(rate, _)| *rate == line.rate) {
            Some((_, ht)) => *ht = ht.checked_add(line.ht)?,
            None => groups.push((line.rate, line.ht)),
        }
    }
    groups.sort_by_key(|(rate, _)| *rate);
    Ok((groups, total_ht))
}

/// `part` of `whole` of an amount, rounded up. Every figure is at or above
/// zero here, so adding the divisor less one before dividing is the ceiling.
/// i128 keeps the product exact; two i64 factors can overflow i64.
fn up(amount: Money, part: Money, whole: Money) -> Result<Money, CoreError> {
    if amount == Money::ZERO || whole <= Money::ZERO {
        return Ok(Money::ZERO);
    }
    let divisor = i128::from(whole.as_centimes());
    let product = i128::from(amount.as_centimes())
        .checked_mul(i128::from(part.as_centimes()))
        .ok_or(MoneyError::Overflow)?
        .checked_add(divisor.checked_sub(1).ok_or(MoneyError::Overflow)?)
        .ok_or(MoneyError::Overflow)?;
    let share = product.checked_div(divisor).ok_or(MoneyError::Overflow)?;
    Ok(i64::try_from(share)
        .map(Money::centimes)
        .map_err(|_| MoneyError::Overflow)?)
}
