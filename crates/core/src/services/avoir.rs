//! The avoir, the credit note a shop writes against a facture it has already
//! issued (features.md §3).
//!
//! A facture is never edited and never deleted: it carries a number out of an
//! uninterrupted series (décret 05-468 art. 10), and the customer is holding a
//! copy of it. What a shop does instead is write a second numbered document
//! that carries the money back, out of its own series, naming the facture it
//! is written against.
//!
//! Three rules decide what one is worth. It never carries the droit de timbre:
//! the stamp is paid on money that changed hands (Code du timbre 2026
//! art. 100-I) and is not refunded with the goods. Its TVA is per rate on its
//! own lines, computed by the same money functions a sale goes through, so a
//! partial avoir is taxed as the part it credits rather than as a share of the
//! facture's tax. And the running total of avoirs on one facture never passes
//! what that facture asked for, which is the safety net under every other
//! rule here: a credit note for more than the paper it credits is money the
//! shop never took.
//!
//! What it does to the account is the ledger's half. The unpaid part of the
//! facture is reversed first, then whatever is left of the avoir goes over the
//! customer's other unpaid documents oldest first, and only what no paper can
//! take becomes credit the shop is holding. That last step is the only way a
//! customer's balance goes below zero (features.md §2).

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::{DebtKind, NewDebtEntry};
use crate::models::stock::{Movement, MovementKind};
use crate::money::{compute_totals, Line, Money, MoneyError, Totals, TotalsOptions, TvaLine};
use crate::repos::documents as repo;
use crate::services::documents::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, NewDocument,
    NewDocumentLine,
};
use crate::services::{audit, clock, debt, documents, optional_field, products, stock};

/// One line of a facture and how much of it is coming back.
///
/// The line is named by id and never by the product on it: a facture carries
/// one product on two lines as soon as a line discount or a second price is
/// involved, and what is left to credit is a fact about a line rather than
/// about a product.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvoirLine {
    pub document_line_id: i32,
    /// Thousandths of the unit, the way the sold quantity is.
    pub qty_milli: i64,
}

/// Writes the avoir: its number, its document with the lines and the TVA
/// recap, the stock coming back, the ledger movement and what it settled, and
/// the audit entry. One transaction, so a credit note that is refused anywhere
/// leaves no number burned and no goods back on the shelf.
///
/// `lines` of `None` is the whole of what is left on the facture, not the whole
/// of what it was: a facture already credited in part is finished off by one
/// whole avoir rather than refused for asking twice.
pub fn issue(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    facture_id: i32,
    lines: Option<Vec<AvoirLine>>,
    reason: Option<String>,
    at: Option<NaiveDateTime>,
) -> Result<Document, CoreError> {
    let reason = optional_field("reason", reason.as_deref())?;
    conn.transaction(|conn| {
        // Only a facture, and only one that still stands. `get_of_kind`
        // answers "no such facture" for a ticket, an avoir or another shop's
        // document alike (rule 3), which is the honest answer in all three.
        let facture = documents::get_of_kind(conn, shop_id, facture_id, DocumentKind::Facture)?;
        if facture.status == DocumentStatus::Cancelled {
            return Err(CoreError::validation(
                "document_id",
                "this facture is annulée, and a document that asks for nothing is credited by nothing",
            ));
        }

        let earlier = repo::avoirs_of(conn, shop_id, facture_id)?;
        let credited = credited_by_line(conn, shop_id, &facture)?;
        let coming_back = chosen(&facture, &credited, lines)?;

        // The one that takes the last quantity off the facture is not computed
        // from its own slice at all: it is the facture less every avoir before
        // it, field by field. Rounding a slice's tax is what makes three
        // credit notes of 60 against a facture of 179, and the third of them
        // is where the centime has to come back
        // (`avoir_closing_carries_the_remainder`).
        let already = credited_amount(&earlier)?;
        let (totals, document_lines) = if closes_the_facture(&facture, &credited, &coming_back) {
            // The closing avoir is a subtraction, so it cannot overrun the
            // facture by arithmetic and the running-total check below would
            // never fire on it. What it can meet is a file that already
            // disagrees with itself: avoirs coming to the whole of the facture
            // while its lines still hold goods, which is what a row written
            // straight into the table looks like. A facture that asked for
            // nothing is left alone, because there is nothing there to overrun.
            if facture.totals.total_ttc != Money::ZERO && already >= facture.totals.total_ttc {
                return Err(CoreError::validation(
                    "lines",
                    "the avoirs on this facture would come to more than it asked for",
                ));
            }
            remainder(&facture, &earlier)?
        } else {
            // Every line is priced as it was sold, so a reprint of the two
            // papers side by side shows the same unit price and the same rate.
            let money_lines: Vec<Line> = coming_back
                .iter()
                .map(|(line, qty_milli, line_discount)| Line {
                    qty_milli: *qty_milli,
                    unit_price: line.unit_price,
                    line_discount: *line_discount,
                    rate: line.rate_bps,
                })
                .collect();
            let totals = compute_totals(
                &money_lines,
                &TotalsOptions {
                    global_discount: global_discount(&facture, &money_lines)?,
                    payment_mode: facture.payment_mode,
                    // An avoir never carries the droit de timbre, whatever the
                    // facture was paid in. Passed as off rather than by handing
                    // over a payment mode the document is not: the stored mode
                    // is the facture's, and the paper says so.
                    stamp_enabled: false,
                    regime: facture.regime,
                },
            )
            .map_err(too_large("lines"))?;

            // The safety net under the quantities: a line's remaining quantity
            // is checked above, and this is checked against the one figure the
            // whole facture asked for. A forged avoir carrying no line at all
            // moves no quantity and would slip past everything else.
            //
            // Against `total_ttc` and not against `net_to_pay`, because the
            // droit de timbre is the one part of a facture no avoir ever gives
            // back: capping at the net would leave the stamp's worth of room
            // for the partials to eat, and the closing avoir would then have
            // to be written for a negative amount.
            if already.checked_add(totals.net_to_pay)? > facture.totals.total_ttc {
                return Err(CoreError::validation(
                    "lines",
                    "the avoirs on this facture would come to more than it asked for",
                ));
            }

            let lines = coming_back
                .iter()
                .zip(&money_lines)
                .map(|((line, qty_milli, line_discount), money)| {
                    Ok(NewDocumentLine {
                        product_id: line.product_id,
                        name: line.name.clone(),
                        barcode: line.barcode.clone(),
                        qty_milli: *qty_milli,
                        unit_price: line.unit_price,
                        line_discount: *line_discount,
                        rate_bps: line.rate_bps,
                        line_total: money
                            .unit_price
                            .checked_mul_milli(*qty_milli)?
                            .checked_sub(*line_discount)?,
                        ref_line_id: Some(line.id),
                    })
                })
                .collect::<Result<Vec<_>, MoneyError>>()?;
            (totals, lines)
        };

        let customer_id = facture.customer_id;
        let old_balance = match customer_id {
            Some(customer_id) => debt::balance(conn, shop_id, customer_id)?,
            None => Money::ZERO,
        };
        // The triple on an avoir is its whole effect on the account: what was
        // owed before it, the negative of its own net, and what is owed after.
        // The middle figure is never decremented by a later settlement, unlike
        // a facture's: the paper states what this credit note was worth on the
        // day it was written, and a payment against some other facture does
        // not change that.
        let effect = Money::ZERO.checked_sub(totals.net_to_pay)?;
        let balance = match customer_id {
            Some(_) => Some(BalanceTriple {
                old_balance,
                remaining_debt: effect,
                total_debt: old_balance.checked_add(effect)?,
            }),
            None => None,
        };

        let avoir = documents::issue(
            conn,
            shop_id,
            NewDocument {
                kind: DocumentKind::Avoir,
                issued_at: at.unwrap_or_else(clock::now),
                user_id,
                regime: facture.regime,
                payment_mode: facture.payment_mode,
                // Both party blocks are the facture's, copied rather than read
                // live: an avoir is read beside the facture it credits, and
                // the two have to name the same seller and the same buyer even
                // after the shop's settings or the fiche have been edited.
                seller: facture.seller.clone(),
                customer_id,
                buyer: facture.buyer.clone(),
                ref_document_id: Some(facture_id),
                balance,
                totals: totals.clone(),
                tendered: None,
                change: None,
                lines: document_lines,
            },
        )?;

        // The goods come back after the document exists, so every movement
        // names the avoir that brought them back. A line whose product has
        // gone moves nothing: there is no count left to move.
        for (line, qty_milli, _) in &coming_back {
            let Some(product_id) = line.product_id else {
                continue;
            };
            // The cost the product carries now, which is what the goods are
            // worth back on the shelf. Read before the movement so the two
            // borrows do not overlap.
            let unit_cost = products::get(conn, shop_id, product_id)?.cost;
            stock::record(
                conn,
                shop_id,
                &Movement {
                    product_id,
                    kind: MovementKind::Return,
                    qty_milli: *qty_milli,
                    unit_cost,
                    document_id: Some(avoir.id),
                    user_id,
                },
            )?;
        }

        let Some(customer_id) = customer_id else {
            return Ok(avoir);
        };

        // One credit movement for the whole of it, naming the avoir. What it
        // settles is said by the allocations beside it, not by this row: an
        // avoir can reach the facture it credits and then a second one.
        let entry = debt::append_at(
            conn,
            shop_id,
            NewDebtEntry {
                customer_id,
                document_id: Some(avoir.id),
                kind: DebtKind::Avoir,
                debit: Money::ZERO,
                credit: totals.net_to_pay,
                user_id,
                note: reason.clone(),
            },
            Some(avoir.issued_at),
        )?;

        // The facture this avoir was written against comes first, whatever its
        // age: the money is going back on that paper and nowhere else. Only
        // what it cannot take spreads over the customer's other unpaid
        // documents, oldest first, and what no paper can take at all is left
        // on the ledger as credit the shop is holding.
        let on_the_facture = totals.net_to_pay.min(unpaid_on(&facture));
        debt::settle_document(conn, shop_id, entry.id, facture_id, on_the_facture)?;
        debt::settle_oldest_first(
            conn,
            shop_id,
            customer_id,
            entry.id,
            totals.net_to_pay.checked_sub(on_the_facture)?,
        )?;

        let after = debt::balance(conn, shop_id, customer_id)?;
        let left_on_facture = documents::get(conn, shop_id, facture_id)?
            .balance
            .map_or(Money::ZERO, |b| b.remaining_debt);
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_AVOIR,
                // The facture is the entity that changed: it is the paper a
                // reader is holding when they ask why it stopped asking for
                // its amount. The avoir is named in `after` as what the
                // decision produced.
                entity: "document",
                entity_id: Some(facture_id),
                before: Some(
                    serde_json::json!({
                        "remaining_debt_centimes": unpaid_on(&facture).as_centimes(),
                        "balance_centimes": old_balance.as_centimes(),
                    })
                    .to_string(),
                ),
                after: Some(
                    serde_json::json!({
                        "avoir_document_id": avoir.id,
                        "avoir_number": avoir.number,
                        "amount_centimes": totals.net_to_pay.as_centimes(),
                        "remaining_debt_centimes": left_on_facture.as_centimes(),
                        "balance_centimes": after.as_centimes(),
                        "reason": reason,
                    })
                    .to_string(),
                ),
            },
        )?;

        Ok(avoir)
    })
}

/// Every avoir written against one facture, oldest first. What the screen
/// showing a facture lists under it, and what the running total is summed
/// from.
pub fn list_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    facture_id: i32,
) -> Result<Vec<Document>, CoreError> {
    // Through the service, so a ticket or another shop's document is answered
    // as no such facture rather than as an empty list (rule 3).
    documents::get_of_kind(conn, shop_id, facture_id, DocumentKind::Facture)?;
    repo::avoirs_of(conn, shop_id, facture_id)
}

/// Whether any line of the facture still has something on it to credit.
///
/// What a cancellation asks before writing an avoir: a facture whose goods
/// have all come back already is annulled without a second credit note, and
/// calling `issue` to find that out would mean reading a refusal as an answer.
pub fn anything_left(
    conn: &mut SqliteConnection,
    shop_id: i32,
    facture: &Document,
) -> Result<bool, CoreError> {
    let credited = credited_by_line(conn, shop_id, facture)?;
    Ok(facture.lines.iter().any(|line| {
        let taken = credited
            .iter()
            .find(|(id, _)| *id == line.id)
            .map_or(0, |(_, qty)| *qty);
        line.qty_milli.saturating_sub(taken) > 0
    }))
}

/// What is still unpaid on a facture. A facture with no customer carries no
/// triple at all, and nothing was ever owed on it.
fn unpaid_on(facture: &Document) -> Money {
    facture.balance.map_or(Money::ZERO, |b| b.remaining_debt)
}

/// What earlier avoirs have already asked for against this facture.
fn credited_amount(earlier: &[Document]) -> Result<Money, CoreError> {
    let mut sum = Money::ZERO;
    for avoir in earlier {
        sum = sum.checked_add(avoir.totals.net_to_pay)?;
    }
    Ok(sum)
}

/// Whether this avoir takes the last quantity off the facture, which is what
/// decides how its money is computed.
///
/// Every line of the facture has to end at nothing: a line still holding one
/// unit is a facture that can be credited again, and the avoir being written
/// is one more partial.
fn closes_the_facture(facture: &Document, credited: &[(i32, i64)], coming: &Coming) -> bool {
    facture.lines.iter().all(|line| {
        let taken = credited
            .iter()
            .find(|(id, _)| *id == line.id)
            .map_or(0, |(_, qty)| *qty);
        let now = coming
            .iter()
            .find(|(l, _, _)| l.id == line.id)
            .map_or(0, |(_, qty, _)| *qty);
        line.qty_milli.saturating_sub(taken).saturating_sub(now) <= 0
    })
}

/// The facture less every avoir already written against it: the totals field
/// by field, and the lines line by line.
///
/// This is the closing avoir, and it exists because a slice of a facture is
/// not a fraction of it. The tax on each slice is rounded once, on that
/// slice's own base, so three slices of a 179 facture come to 180 and three
/// slices of a 286 one come to 285. Neither is wrong on its own paper and both
/// are wrong added up, and what a shop and a comptable read is the sum: the
/// avoirs on a facture have to reproduce it. So the last one is the
/// difference, and it carries whatever the rounding left over.
///
/// The droit de timbre is the one thing not in the difference. It is paid on
/// money that changed hands (Code du timbre 2026 art. 100-I) and never given
/// back, so the figure being reproduced is the facture's `total_ttc` and the
/// avoir's own stamp stays at zero.
///
/// A line that comes to nothing at all is dropped; a line whose quantity is
/// spent but whose centimes are not is kept, because that centime is the whole
/// reason this function exists.
fn remainder(
    facture: &Document,
    earlier: &[Document],
) -> Result<(Totals, Vec<NewDocumentLine>), CoreError> {
    let mut totals = Totals {
        total_ht: facture.totals.total_ht,
        discount: facture.totals.discount,
        subtotal_ht: facture.totals.subtotal_ht,
        tva_by_rate: facture.totals.tva_by_rate.clone(),
        tva: facture.totals.tva,
        total_ttc: facture.totals.total_ttc,
        stamp: Money::ZERO,
        net_to_pay: facture.totals.total_ttc,
    };
    for avoir in earlier {
        totals.total_ht = totals.total_ht.checked_sub(avoir.totals.total_ht)?;
        totals.discount = totals.discount.checked_sub(avoir.totals.discount)?;
        totals.subtotal_ht = totals.subtotal_ht.checked_sub(avoir.totals.subtotal_ht)?;
        totals.tva = totals.tva.checked_sub(avoir.totals.tva)?;
        totals.total_ttc = totals.total_ttc.checked_sub(avoir.totals.total_ttc)?;
        for row in &avoir.totals.tva_by_rate {
            // A rate an earlier avoir carries is a rate the facture carries:
            // an avoir line is a facture line and takes its rate from it.
            let Some(mine) = totals.tva_by_rate.iter_mut().find(|r| r.rate == row.rate) else {
                return Err(CoreError::validation(
                    "lines",
                    "an avoir on this facture carries a rate the facture does not",
                ));
            };
            mine.base = mine.base.checked_sub(row.base)?;
            mine.amount = mine.amount.checked_sub(row.amount)?;
        }
    }
    totals.net_to_pay = totals.total_ttc;
    totals
        .tva_by_rate
        .retain(|r: &TvaLine| r.base != Money::ZERO || r.amount != Money::ZERO);

    let mut lines = Vec::new();
    for line in &facture.lines {
        let mut qty_milli = line.qty_milli;
        let mut line_discount = line.line_discount;
        let mut line_total = line.line_total;
        for avoir in earlier {
            for taken in avoir
                .lines
                .iter()
                .filter(|l| l.ref_line_id == Some(line.id))
            {
                qty_milli = qty_milli.saturating_sub(taken.qty_milli);
                line_discount = line_discount.checked_sub(taken.line_discount)?;
                line_total = line_total.checked_sub(taken.line_total)?;
            }
        }
        if qty_milli == 0 && line_total == Money::ZERO && line_discount == Money::ZERO {
            continue;
        }
        lines.push(NewDocumentLine {
            product_id: line.product_id,
            name: line.name.clone(),
            barcode: line.barcode.clone(),
            qty_milli,
            unit_price: line.unit_price,
            line_discount,
            rate_bps: line.rate_bps,
            line_total,
            ref_line_id: Some(line.id),
        });
    }
    Ok((totals, lines))
}

/// How much of each facture line earlier avoirs have already taken back, by
/// line id.
fn credited_by_line(
    conn: &mut SqliteConnection,
    shop_id: i32,
    facture: &Document,
) -> Result<Vec<(i32, i64)>, CoreError> {
    let ids: Vec<i32> = facture.lines.iter().map(|l| l.id).collect();
    repo::credited_by_line(conn, shop_id, &ids)
}

/// The facture lines that are coming back, each with the quantity and the
/// share of its line discount that comes with it.
///
/// `None` is every line that still has something left on it, which is what
/// finishes off a facture already credited in part. A named list is checked
/// line by line: a line of some other facture is not found, a quantity at or
/// below zero credits nothing, and a quantity past what the line has left is
/// goods that were never sold.
type Coming<'a> = Vec<(&'a DocumentLine, i64, Money)>;

fn chosen<'a>(
    facture: &'a Document,
    credited: &[(i32, i64)],
    asked: Option<Vec<AvoirLine>>,
) -> Result<Coming<'a>, CoreError> {
    let left = |line: &DocumentLine| -> i64 {
        let taken = credited
            .iter()
            .find(|(id, _)| *id == line.id)
            .map_or(0, |(_, qty)| *qty);
        line.qty_milli.saturating_sub(taken)
    };

    let mut coming: Coming<'a> = Vec::new();
    match asked {
        None => {
            for line in &facture.lines {
                let qty = left(line);
                if qty > 0 {
                    coming.push((line, qty, prorated_discount(line, qty)?));
                }
            }
        }
        Some(asked) => {
            for want in asked {
                let Some(line) = facture.lines.iter().find(|l| l.id == want.document_line_id)
                else {
                    return Err(CoreError::NotFound {
                        entity: "document_line",
                        id: want.document_line_id,
                    });
                };
                if coming.iter().any(|(l, _, _)| l.id == line.id) {
                    return Err(CoreError::validation(
                        "document_line_id",
                        "the same line is credited twice on one avoir",
                    ));
                }
                if want.qty_milli <= 0 {
                    return Err(CoreError::validation(
                        "qty_milli",
                        "a credited quantity is above zero",
                    ));
                }
                if want.qty_milli > left(line) {
                    return Err(CoreError::validation(
                        "qty_milli",
                        "more of this line is being credited than was sold and not yet credited",
                    ));
                }
                coming.push((
                    line,
                    want.qty_milli,
                    prorated_discount(line, want.qty_milli)?,
                ));
            }
        }
    }
    if coming.is_empty() {
        return Err(CoreError::validation(
            "lines",
            "there is nothing left to credit on this facture",
        ));
    }
    Ok(coming)
}

/// The share of a line's discount that comes back with part of that line
/// (`avoir_partial_prorates_discounts_floor`).
///
/// A line discount belongs to the whole line, so crediting a third of the line
/// credits a third of the discount. Rounded down, because the discount is what
/// the customer was not charged: rounding it up would take a centime off the
/// credit note that the customer never paid in the first place. A whole line
/// comes back with the whole of its discount and no rounding at all.
///
/// i128 keeps the product exact; two i64 factors can overflow i64.
fn prorated_discount(line: &DocumentLine, qty_milli: i64) -> Result<Money, CoreError> {
    if qty_milli >= line.qty_milli || line.line_discount == Money::ZERO {
        return Ok(line.line_discount);
    }
    let product = i128::from(line.line_discount.as_centimes())
        .checked_mul(i128::from(qty_milli))
        .ok_or(MoneyError::Overflow)?;
    let share = product
        .checked_div(i128::from(line.qty_milli))
        .ok_or(MoneyError::Overflow)?;
    Ok(i64::try_from(share)
        .map(Money::centimes)
        .map_err(|_| MoneyError::Overflow)?)
}

/// The share of the facture's global discount that comes back with these
/// lines, by the same rule and rounded the same way: the credited HT against
/// the whole HT of the facture.
///
/// An avoir for every line of an uncredited facture carries the whole global
/// discount and reproduces the facture's total to the centime, because the two
/// HT figures are then equal and no rounding happens.
fn global_discount(facture: &Document, lines: &[Line]) -> Result<Money, CoreError> {
    let discount = facture.totals.discount;
    let whole = facture.totals.total_ht;
    if discount == Money::ZERO || whole == Money::ZERO {
        return Ok(Money::ZERO);
    }
    let mut credited_ht = Money::ZERO;
    for line in lines {
        let gross = line.unit_price.checked_mul_milli(line.qty_milli)?;
        credited_ht = credited_ht.checked_add(gross.checked_sub(line.line_discount)?)?;
    }
    if credited_ht >= whole {
        return Ok(discount);
    }
    let product = i128::from(discount.as_centimes())
        .checked_mul(i128::from(credited_ht.as_centimes()))
        .ok_or(MoneyError::Overflow)?;
    let share = product
        .checked_div(i128::from(whole.as_centimes()))
        .ok_or(MoneyError::Overflow)?;
    Ok(i64::try_from(share)
        .map(Money::centimes)
        .map_err(|_| MoneyError::Overflow)?)
}

/// An amount that does not fit is the caller's arithmetic, named on the field
/// it came from, the way `services::sales` names it.
fn too_large(field: &'static str) -> impl Fn(MoneyError) -> CoreError {
    move |e| match e {
        MoneyError::Overflow => CoreError::validation(
            field,
            "this amount is past what the till can hold in centimes",
        ),
        other => CoreError::from(other),
    }
}
