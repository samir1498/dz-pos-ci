//! What an order's lines are worth: the landed cost `save` fixes once, and
//! the value a delivery or a return moves on the supplier's account. Lifted
//! out of `services::purchases` whole on 2026-09-24, when that file was at
//! the length `scripts/file-sizes.json` pins it to and the purchase's own
//! number and account had to go somewhere.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::purchase::PurchaseLine;
use crate::money::Money;
use crate::services::products;

/// One product on an order, as a caller hands it over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLine {
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    /// What the supplier charges for the unit, before the extra costs are
    /// spread over the order.
    pub unit_cost: Money,
}

/// Every line's landed unit cost, and what the order is worth once they are
/// applied.
pub(crate) struct Landed {
    pub(crate) per_line: Vec<Money>,
    /// The sum of the landed line totals: what the whole order will put on the
    /// supplier's account if all of it arrives.
    pub(crate) total: Money,
}

/// Spreads the transport and the extra costs over the lines by value and
/// divides each share per unit.
///
/// By value, because a share by quantity would divide kilos and pieces by
/// each other. The shares are floored and the remainder goes to the last
/// line, so the shares add up to the extra costs exactly; the per-unit
/// division is floored too, which is the one place the order loses centimes:
/// a line's landed total can come out under its value plus its share by fewer
/// centimes than the line has units, plus one. Nothing is ever gained, so the sum of the landed
/// line totals is never above the lines' value plus the extra costs, which is
/// what `purchase_prop` pins.
pub(crate) fn spread(
    conn: &mut SqliteConnection,
    shop_id: i32,
    lines: &[NewLine],
    transport: Money,
    extra_costs: Money,
) -> Result<Landed, CoreError> {
    let mut values = Vec::with_capacity(lines.len());
    let mut seen: Vec<i32> = Vec::with_capacity(lines.len());
    let mut order_value = Money::ZERO;
    for line in lines {
        if line.qty_ordered_milli <= 0 {
            return Err(CoreError::validation(
                "qty_ordered_milli",
                "a line ordering nothing orders nothing",
            ));
        }
        if line.unit_cost.is_negative() {
            return Err(CoreError::validation(
                "unit_cost_centimes",
                "a unit cost cannot be negative",
            ));
        }
        // The product has to be this shop's: a `NotFound` here rather than the
        // foreign key's failure further down (rule 3).
        products::get(conn, shop_id, line.product_id)?;
        if seen.contains(&line.product_id) {
            // "The cost of the last receipt" has to name one line, and two
            // lines of one product on one order leave it asking which.
            return Err(CoreError::validation(
                "product_id",
                "a product is named once on an order",
            ));
        }
        seen.push(line.product_id);
        let value = line.unit_cost.checked_mul_milli(line.qty_ordered_milli)?;
        order_value = order_value.checked_add(value)?;
        values.push(value);
    }

    let extra = transport.checked_add(extra_costs)?;
    if order_value == Money::ZERO {
        if extra != Money::ZERO {
            // A share of a line worth nothing is nothing, so there is no
            // honest place to put the amount. Refused on the field that
            // carries most of it rather than spread by quantity, which would
            // add kilos to pieces.
            return Err(CoreError::validation(
                "transport_centimes",
                "costs cannot be spread over lines that are worth nothing",
            ));
        }
        // Free samples, and nothing to spread over them. Answered here rather
        // than through the loop below, which would divide by the order's
        // value: an order of one line never reached that division because the
        // single line takes the remainder, and an order of two did.
        return Ok(Landed {
            per_line: vec![Money::ZERO; lines.len()],
            total: Money::ZERO,
        });
    }

    let mut per_line = Vec::with_capacity(lines.len());
    let mut given = Money::ZERO;
    let mut total = Money::ZERO;
    let last = lines.len() - 1;
    for (index, line) in lines.iter().enumerate() {
        let share = if index == last {
            // The remainder, so the shares add up to the extra costs exactly
            // whatever the division left behind.
            extra.checked_sub(given)?
        } else {
            let value = values.get(index).copied().unwrap_or(Money::ZERO);
            let by_value = share_of(extra, value, order_value)?;
            given = given.checked_add(by_value)?;
            by_value
        };
        // The share divided per unit, floored: the line's landed total can
        // come out under its value plus its share, never above it.
        let per_unit = Money::centimes(
            i64::try_from(
                i128::from(share.as_centimes())
                    .checked_mul(i128::from(crate::money::MILLI_PER_UNIT))
                    .ok_or(crate::money::MoneyError::Overflow)?
                    / i128::from(line.qty_ordered_milli),
            )
            .map_err(|_| crate::money::MoneyError::Overflow)?,
        );
        let landed = line.unit_cost.checked_add(per_unit)?;
        total = total.checked_add(landed.checked_mul_milli(line.qty_ordered_milli)?)?;
        per_line.push(landed);
    }
    Ok(Landed { per_line, total })
}

/// What one movement of `qty_milli` is worth, given that `before` of the line
/// had already moved the same way.
///
/// The difference the movement makes to a running total, never the movement
/// priced on its own. A delivery priced on its own is rounded half-up, and
/// two halves of a line each rounded up come to a centime more than the line
/// is worth: an order paid in full up front is then left asking for that
/// centime for ever, and one whose halves round down is quietly forgiven one.
///
/// The same function prices a return, against the quantity that has already
/// gone back, so a line received whole and returned whole leaves nothing on
/// the account whatever the rounding did on the way.
pub(crate) fn step_value(
    line: &PurchaseLine,
    before: i64,
    qty_milli: i64,
) -> Result<Money, CoreError> {
    let after = before
        .checked_add(qty_milli)
        .ok_or(crate::money::MoneyError::Overflow)?;
    let was = running_value(line, before)?;
    let now = running_value(line, after)?;
    Ok(now.checked_sub(was)?)
}

/// What the first `qty_milli` of a line are worth on the ledger: the landed
/// cost times the quantity, rounded down, except at the whole ordered
/// quantity where it is the line's landed total rounded once the way `spread`
/// rounded it. Monotone, so a movement is never worth a negative amount, and
/// exact at the end, so the deliveries of a line add up to what the line is
/// worth and no more.
pub(crate) fn running_value(line: &PurchaseLine, qty_milli: i64) -> Result<Money, CoreError> {
    if qty_milli >= line.qty_ordered_milli {
        return Ok(line
            .landed_unit_cost
            .checked_mul_milli(line.qty_ordered_milli)?);
    }
    // Both factors are at or above zero here (the file and `spread` both
    // refuse a negative), so the truncation this division does is a floor.
    let raw = i128::from(line.landed_unit_cost.as_centimes())
        .checked_mul(i128::from(qty_milli))
        .ok_or(crate::money::MoneyError::Overflow)?
        / i128::from(crate::money::MILLI_PER_UNIT);
    Ok(Money::centimes(
        i64::try_from(raw).map_err(|_| crate::money::MoneyError::Overflow)?,
    ))
}

/// `extra × value / order_value`, floored, in i128 so two i64 factors cannot
/// overflow on the way.
fn share_of(extra: Money, value: Money, order_value: Money) -> Result<Money, CoreError> {
    let raw = i128::from(extra.as_centimes())
        .checked_mul(i128::from(value.as_centimes()))
        .ok_or(crate::money::MoneyError::Overflow)?
        / i128::from(order_value.as_centimes());
    Ok(Money::centimes(
        i64::try_from(raw).map_err(|_| crate::money::MoneyError::Overflow)?,
    ))
}
