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
//! own lines, rounded once on its own base the way a sale's is, so a partial
//! avoir is taxed as the part it credits rather than as a share of the
//! facture's tax. And the running total of avoirs on one facture never passes
//! what that facture asked for, which is the safety net under every other
//! rule here: a credit note for more than the paper it credits is money the
//! shop never took. That last rule is read per rate as well as on the total,
//! because a slice that rounds its tax up at one rate leaves the credit note
//! that closes the facture holding the centime it took too much.
//!
//! Every one of those rules is the same subtraction: the facture less the
//! avoirs already written against it. It is done once, into `Remaining`, and
//! every rule below reads that. A partial avoir is `min(the slice, what is
//! left)` per line and per rate; the one that closes the facture is what is
//! left, whole, field by field and line by line.
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
use crate::money::{Money, MoneyError};
use crate::services::avoir_remaining::{below_zero, Coming, Remaining, SliceLine};
use crate::services::avoir_slice::slice_totals;
use crate::services::cash_refunds::Refund;
use crate::services::documents::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, NewDocument,
    NewDocumentLine,
};
use crate::services::{cash_refunds, customers, debt, documents, stock};
use dzpos_kernel::services::{audit, clock, optional_field};

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
///
/// The money goes back on the customer's ledger. `issue_settling` beside it is
/// the same write with the other way of settling it, cash over the counter.
pub fn issue(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    facture_id: i32,
    lines: Option<Vec<AvoirLine>>,
    reason: Option<String>,
    at: Option<NaiveDateTime>,
) -> Result<Document, CoreError> {
    issue_settling(
        conn,
        shop_id,
        user_id,
        facture_id,
        lines,
        reason,
        at,
        Refund::None,
    )
}

/// The same credit note, saying how the money goes back (ruling 5 of the
/// 2026-09-20 loop).
///
/// `Refund::None` is the ledger credit every caller wrote before, and `issue`
/// above is that call with the argument spelled out, so nothing that does not
/// hand cash over has to say so. `Refund::Cash` writes a `cash_refunds` row
/// for the avoir's own `net_to_pay` and **no ledger credit at all**: the
/// customer is holding notes, not credit, and writing both would give the
/// money back twice.
///
/// **Cash is bounded by what came in, not by what is still owed.** The read is
/// `cash_refunds::still_to_hand_back` (its doc says why not `remaining_debt`), taken before
/// the avoir is written so a refusal burns no number: what was paid in against the facture
/// less what has gone back, so a half-paid credit facture hands back that half and no more.
/// `cancel_settling` bounds by what is left on the facture, and refuses a credit document.
///
/// A facture with no customer takes the cash path and writes its refund row.
/// That is the case ruling 5 exists for: the anonymous walk-in has no ledger
/// to credit, so before this row a shop that gave 3 000 DA back had no record
/// of it anywhere and the cashier was 3 000 short at every close.
///
/// The droit de timbre is never handed back, and nothing here has to arrange
/// that: an avoir carries no stamp at all, so its `net_to_pay` is its
/// `total_ttc` and the refund is the avoir's own figure
/// (`an_avoir_prints_no_stamp_and_one_that_carries_a_stamp_is_refused`).
#[allow(clippy::too_many_arguments)]
pub fn issue_settling(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    facture_id: i32,
    lines: Option<Vec<AvoirLine>>,
    reason: Option<String>,
    at: Option<NaiveDateTime>,
    refund: Refund,
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
        // The one subtraction, done once. Everything below reads it.
        let remaining = Remaining::of(conn, shop_id, &facture)?;
        let coming_back = chosen(&facture, &remaining, lines)?;

        // The one that takes the last quantity off the facture is not computed
        // from its own slice at all: it is what is left, field by field.
        // Rounding a slice's tax is what makes three credit notes of 60
        // against a facture of 179, and the third of them is where the centime
        // has to come back (`avoir_closing_carries_the_remainder`).
        let (totals, document_lines) = if remaining.closed_by(&coming_back) {
            let (totals, lines) = remaining.whole(&facture)?;
            // The closing avoir is a subtraction, so it cannot overrun the
            // facture by arithmetic and the running-total check below would
            // never fire on it. What it can meet is a file that already
            // disagrees with itself: avoirs coming to the whole of the facture
            // while its lines still hold goods, which is what a row written
            // straight into the table looks like. The subtraction then lands
            // on a field below zero, or on a total its own lines do not add up
            // to, and either one is that overrun read on the paper that would
            // have to carry it.
            //
            // A subtraction that lands on nothing at all is not an overrun:
            // the last milligrams of a line can be worth no centime, and the
            // credit note that takes them back is written for zero and puts
            // the goods on the shelf.
            let summed = lines
                .iter()
                .try_fold(Money::ZERO, |acc, l| acc.checked_add(l.line_total))?;
            let a_line_below_zero = lines.iter().any(|l| l.line_total.is_negative());
            if below_zero(&totals) || a_line_below_zero || summed != totals.total_ht {
                return Err(CoreError::validation(
                    "lines",
                    "the avoirs on this facture would come to more than it asked for",
                ));
            }
            (totals, lines)
        } else {
            // Every line is priced as it was sold, so a reprint of the two
            // papers side by side shows the same unit price and the same rate,
            // and never for more than the facture line still holds. Half a
            // unit at 0,01 costs a centime on its own paper, so two halves
            // credit two centimes of a line worth one, and the credit note
            // that closes the facture is left owing the difference.
            let mut slice: Vec<SliceLine> = Vec::new();
            let mut document_lines = Vec::new();
            for (line, qty_milli, line_discount) in &coming_back {
                let asked = line
                    .unit_price
                    .checked_mul_milli(*qty_milli)?
                    .checked_sub(*line_discount)?;
                let line_total = asked.min(remaining.money_on_line(line.id));
                slice.push(SliceLine {
                    rate: line.rate_bps,
                    ht: line_total,
                });
                document_lines.push(NewDocumentLine {
                    product_id: line.product_id,
                    name: line.name.clone(),
                    barcode: line.barcode.clone(),
                    qty_milli: *qty_milli,
                    unit_price: line.unit_price,
                    line_discount: *line_discount,
                    rate_bps: line.rate_bps,
                    line_total,
                    ref_line_id: Some(line.id),
                });
            }
            let totals = slice_totals(&facture, &remaining, &slice)?;

            // The safety net under the quantities: a line's remaining
            // quantity is checked above, and this is checked against the one
            // figure the whole facture asked for, which `Remaining::totals`
            // says why. A forged avoir carrying no line at all moves no
            // quantity and would slip past everything else.
            if totals.net_to_pay > remaining.totals.total_ttc {
                return Err(CoreError::validation(
                    "lines",
                    "the avoirs on this facture would come to more than it asked for",
                ));
            }

            (totals, document_lines)
        };

        // Before the number is drawn and the goods move, so a refusal costs
        // nothing.
        if refund.is_cash() {
            let left = cash_refunds::still_to_hand_back(conn, shop_id, &facture)?;
            if totals.net_to_pay > left {
                return Err(CoreError::validation(
                    "refund",
                    "more cash than was ever paid in on this facture; \
                     what the credit note is worth comes off the account",
                ));
            }
        }

        let customer = customers::prove_named(conn, shop_id, facture.customer_id)?;
        let old_balance = match &customer {
            Some(customer) => debt::balance(conn, shop_id, customer.id())?,
            None => Money::ZERO,
        };
        // The triple on an avoir is its whole effect on the account: what was
        // owed before it, the negative of its own net, and what is owed after.
        // The middle figure is never decremented by a later settlement, unlike
        // a facture's: the paper states what this credit note was worth on the
        // day it was written, and a payment against some other facture does
        // not change that.
        //
        // A credit note settled in notes has no effect on the account at all,
        // so its middle figure is zero and its `total_debt` is the balance it
        // found. Stamping it with `-net_to_pay` would print a paper saying the
        // account moved while `debt::balance` stays where it was, and the two
        // would disagree for the life of the document.
        let effect = if refund.is_cash() {
            Money::ZERO
        } else {
            Money::ZERO.checked_sub(totals.net_to_pay)?
        };
        let balance = match &customer {
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
                customer: customer.clone(),
                buyer: facture.buyer.clone(),
                ref_document_id: Some(facture_id),
                balance,
                totals: totals.clone(),
                tendered: None,
                change: None,
                lines: document_lines,
            },
        )?;

        // What the goods cost when they left on the facture, read once for
        // the whole loop. The credit note puts them back at that cost and
        // never at the fiche's cost today: a delivery between the sale and
        // the credit note moves the fiche, and a reversal that followed it
        // would move the month's margin with every purchase.
        let sold_at = stock::sale_costs(conn, shop_id, facture_id)?;

        // The goods come back after the document exists, so every movement
        // names the avoir that brought them back. A line whose product has
        // gone moves nothing: there is no count left to move.
        for (line, qty_milli, _) in &coming_back {
            let Some(product_id) = line.product_id else {
                continue;
            };
            // The fiche's cost is not a fallback. `unit_cost_centimes` has
            // been NOT NULL since the first documents migration and a sale is
            // the only writer of a `sale` movement, so a sold line without one
            // is a file that disagrees with itself; writing today's cost
            // instead would move a margin already earned with the next
            // delivery, which is the whole thing the ledger cost prevents.
            let Some(unit_cost) = sold_at.get(&product_id).copied() else {
                return Err(CoreError::UnpricedReversal {
                    document_id: facture_id,
                    product_id,
                    reason: "the line it credits has no sale movement",
                });
            };
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

        // Cash over the counter is the whole of the money half: the row that
        // says the drawer opened, and no ledger credit beside it. Above the
        // early return below, because the customer this avoir names may be
        // nobody at all and the anonymous facture is the case ruling 5 was
        // taken for.
        //
        // The amount is the avoir's own `net_to_pay`, which is its
        // `total_ttc`: an avoir carries no droit de timbre, so the stamp the
        // facture collected for the state is not in this figure and is never
        // handed back. `avoir.issued_at` and not `now`, so a credit note
        // backdated by a cancellation lands its cash on the same day the
        // paper is dated.
        if refund.is_cash() {
            cash_refunds::hand_over(
                conn,
                shop_id,
                user_id,
                avoir.id,
                totals.net_to_pay,
                avoir.issued_at,
            )?;
        }

        // An avoir for no money moves no debt. The closing one comes to nothing
        // when what is left of the facture is a quantity worth no centime: the
        // goods come back on it and the ledger is left alone, and it is
        // audited like any other because a numbered document was written.
        //
        // Nor does one that was settled in notes: the customer is holding the
        // money, so crediting the account as well would give it back twice.
        // The `cash_refunds` row above is the whole of that half.
        //
        // A facture naming nobody has no account for any of it: the goods
        // come back, the notes go back if that is how it was settled, and the
        // log below still records it. That row used to be skipped with the
        // ledger, which left the walk-in this feature exists for — the one
        // with no fiche — handing money over the counter with nothing in the
        // audit log at all.
        let customer_id = customer.as_ref().map(|c| c.id());
        if let (Some(customer_id), false, true) = (
            customer_id,
            refund.is_cash(),
            totals.net_to_pay != Money::ZERO,
        ) {
            // One credit movement for the whole of it, naming the avoir. What
            // it settles is said by the allocations beside it, not by this
            // row: an avoir can reach the facture it credits and then a
            // second one.
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

            // The facture this avoir was written against comes first, whatever
            // its age: the money is going back on that paper and nowhere else.
            // Only what it cannot take spreads over the customer's other
            // unpaid documents, oldest first, and what no paper can take at
            // all is left on the ledger as credit the shop is holding.
            let on_the_facture = totals.net_to_pay.min(unpaid_on(&facture));
            debt::settle_document(conn, shop_id, entry.id, facture_id, on_the_facture)?;
            debt::settle_oldest_first(
                conn,
                shop_id,
                customer_id,
                entry.id,
                totals.net_to_pay.checked_sub(on_the_facture)?,
            )?;
        }

        let after = match customer_id {
            Some(customer_id) => debt::balance(conn, shop_id, customer_id)?,
            None => Money::ZERO,
        };
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
                        // How the money went back. Without it a credit note
                        // that opened the drawer and one that credited an
                        // account read the same in the log, and the drawer is
                        // the one somebody has to answer for.
                        "settlement": refund.as_str(),
                        "cash_back_centimes": refund
                            .is_cash()
                            .then(|| totals.net_to_pay.as_centimes()),
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
    documents::avoirs_of(conn, shop_id, facture_id)
}

/// What each line of this document still has on it, line id to quantity in
/// thousandths.
///
/// The same subtraction `issue` is written from, handed to
/// `services::cancellation` so the two agree about what is left. Without it
/// the cancellation of a part-credited facture puts the credited quantity
/// back on the shelf a second time and the shop reads stock it does not hold.
///
/// A ticket carries no avoirs — `issue` is written against a facture and
/// refuses anything else — so every line of one comes back whole, which is
/// what the caller did before this existed.
pub(crate) fn quantities_left(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document: &Document,
) -> Result<std::collections::HashMap<i32, i64>, CoreError> {
    let remaining = Remaining::of(conn, shop_id, document)?;
    Ok(remaining
        .lines
        .iter()
        .map(|line| (line.id, line.qty_milli))
        .collect())
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
    let remaining = Remaining::of(conn, shop_id, facture)?;
    Ok(remaining.lines.iter().any(|line| line.qty_milli > 0))
}

/// What a whole avoir on this facture would come to, without writing one.
///
/// The figure a cancellation's confirm shows, and it has to be this
/// subtraction rather than the totals of the lines still on the facture: those
/// are what the closing avoir refuses to be computed from, so a confirm
/// re-deriving them would promise 60 where the avoir will be written for 59.
pub fn what_is_left(
    conn: &mut SqliteConnection,
    shop_id: i32,
    facture: &Document,
) -> Result<Money, CoreError> {
    Ok(Remaining::of(conn, shop_id, facture)?
        .totals
        .total_ttc
        .max(Money::ZERO))
}

/// What is still unpaid on a facture. A facture with no customer carries no
/// triple at all, and nothing was ever owed on it.
fn unpaid_on(facture: &Document) -> Money {
    facture.balance.map_or(Money::ZERO, |b| b.remaining_debt)
}

fn chosen<'a>(
    facture: &'a Document,
    remaining: &Remaining,
    asked: Option<Vec<AvoirLine>>,
) -> Result<Coming<'a>, CoreError> {
    let mut coming: Coming<'a> = Vec::new();
    match asked {
        None => {
            for line in &facture.lines {
                let qty = remaining.quantity_on_line(line.id);
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
                if want.qty_milli > remaining.quantity_on_line(line.id) {
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
