//! What the shop ordered from a supplier, what arrived of it, and what that
//! left on the supplier's account (features.md §1, Purchase).
//!
//! Two rules hold the rest up.
//!
//! **The landed unit cost is fixed when the order is saved.** Transport and
//! the other extra costs are agreed once for the whole order, so they are
//! spread over the lines by value and divided per unit at that moment. A
//! share recomputed at each receipt would move a cost a sale has already been
//! measured against, and the margin a report reads would change without
//! anybody selling anything.
//!
//! **Stock and debt move on receipt and never on save** (plan lens,
//! 2026-09-10). A purchase is a piece of paper until goods are handed over: an
//! order closed short owes nothing for what never came, and a shop's shelf
//! count does not rise because somebody wrote an order.
//!
//! A `bon_de_reception` is a `purchase_receipts` row and not a document: the
//! `documents` table's régime, its payment mode and its customer key have no
//! honest value for something the shop buys, and its series are the ones the
//! tax code hands out for what the shop sells. The receipt takes its number
//! from the `reception:<year>` counter instead, which resets on 1 January the
//! way every other series does.

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::purchase::{
    PurchaseLineRowWrite, PurchaseReceiptLineRowWrite, PurchaseReceiptRowWrite, PurchaseRowWrite,
};
use crate::models::stock::Movement;
use crate::models::supplier_debt::SupplierDebtRowWrite;
use crate::money::Money;
use crate::repos::{
    counters, products as products_repo, purchases as repo, supplier_debt as debt_repo,
};
use crate::services::{audit, bounded_field, clock, optional_field, stock, supplier_debt};

pub use crate::models::purchase::{
    Purchase, PurchaseLine, PurchaseReceipt, PurchaseReceiptLine, PurchaseStatus,
};
pub use crate::models::stock::MovementKind;
pub use crate::models::supplier_debt::PaymentMethod;

/// The counter a bon de réception takes its number from, in a given year.
/// One series per year, so the first delivery of January is number 1
/// (`repos::counters::take_next` creates the row on first use).
fn reception_series(year: i32) -> String {
    format!("reception:{year}")
}

/// One product on an order, as a caller hands it over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLine {
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    /// What the supplier charges for the unit, before the extra costs are
    /// spread over the order.
    pub unit_cost: Money,
}

/// Money handed to the supplier at the moment the order is saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paid {
    pub amount: Money,
    pub mode: PaymentMethod,
}

/// An order as the screen sends it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPurchase {
    pub supplier_id: i32,
    pub supplier_document_number: Option<String>,
    /// A day on the shop's calendar, `YYYY-MM-DD`.
    pub purchase_date: String,
    pub due_date: Option<String>,
    pub transport: Money,
    pub extra_costs: Money,
    pub note: Option<String>,
    pub lines: Vec<NewLine>,
    /// Money handed over now, which becomes a `payment` row on the supplier's
    /// ledger inside this transaction.
    pub paid_now: Option<Paid>,
    /// The common case of features.md §1: the goods came with the paper, so
    /// the whole receipt is written in the same transaction as the order.
    pub receive_now: bool,
}

/// One line of a delivery or of a return: how much of one ordered line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiveLine {
    pub purchase_line_id: i32,
    pub qty_milli: i64,
}

/// One delivery with what arrived on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptView {
    pub receipt: PurchaseReceipt,
    pub lines: Vec<PurchaseReceiptLine>,
}

/// A whole order as a screen reads it: the paper, what was ordered with what
/// has arrived and gone back on each line, and every delivery against it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseView {
    pub purchase: Purchase,
    pub lines: Vec<PurchaseLine>,
    /// Newest delivery first, the way the repo answers them.
    pub receipts: Vec<ReceiptView>,
}

/// The shop's orders, newest first, optionally narrowed to one state or one
/// supplier. Both filters are applied here rather than in SQL because the two
/// indexes the file carries open on the shop and the date, and a shop's list
/// of orders is a screen's page rather than a report over years.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    status: Option<PurchaseStatus>,
    supplier_id: Option<i32>,
) -> Result<Vec<Purchase>, CoreError> {
    Ok(repo::list(conn, shop_id)?
        .into_iter()
        .filter(|p| status.is_none_or(|s| p.status == s))
        .filter(|p| supplier_id.is_none_or(|s| p.supplier_id == s))
        .collect())
}

/// One order with its lines and its deliveries.
pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<PurchaseView, CoreError> {
    let purchase = repo::get(conn, shop_id, id)?;
    let lines = repo::lines(conn, shop_id, id)?;
    let mut receipts = Vec::new();
    for receipt in repo::receipts(conn, shop_id, id)? {
        let lines = repo::receipt_lines(conn, shop_id, receipt.id)?;
        receipts.push(ReceiptView { receipt, lines });
    }
    Ok(PurchaseView {
        purchase,
        lines,
        receipts,
    })
}

/// Writes the order, its lines and, when the goods came with the paper, the
/// whole receipt. One transaction: an order whose stock landed and whose
/// ledger row did not would be a shop that owes nothing for goods on its
/// shelf.
///
/// The order of the writes inside it is not free. The `payment` row goes in
/// last, after the receipt has given the order its value on the ledger, so
/// the money lands on paper rather than sitting on the balance while the
/// order it paid for goes on asking to be paid.
pub fn save(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewPurchase,
) -> Result<PurchaseView, CoreError> {
    let day = parse_day("purchase_date", &new.purchase_date)?;
    if let Some(due) = new.due_date.as_deref() {
        let due = parse_day("due_date", due)?;
        if due < day {
            return Err(CoreError::validation(
                "due_date",
                "an order is not due before the day it was placed",
            ));
        }
    }
    if new.lines.is_empty() {
        return Err(CoreError::validation(
            "lines",
            "an order with no line orders nothing",
        ));
    }
    if new.transport.is_negative() {
        return Err(CoreError::validation(
            "transport_centimes",
            "a cost cannot be negative",
        ));
    }
    if new.extra_costs.is_negative() {
        return Err(CoreError::validation(
            "extra_costs_centimes",
            "a cost cannot be negative",
        ));
    }
    let note = optional_field("note", new.note.as_deref())?;
    let supplier_document_number = optional_field(
        "supplier_document_number",
        new.supplier_document_number.as_deref(),
    )?;
    bounded_field("purchase_date", &new.purchase_date)?;

    conn.transaction(|conn| {
        // The fiche has to exist, be this shop's and still be one the shop
        // buys from. A closed fiche refuses an order and goes on taking
        // payments (features.md §1).
        supplier_debt::ensure_active(conn, shop_id, new.supplier_id)?;
        let landed = spread(conn, shop_id, &new)?;

        if let Some(paid) = new.paid_now {
            if paid.amount.as_centimes() <= 0 {
                return Err(CoreError::validation(
                    "paid_now_centimes",
                    "money handed over is more than nothing",
                ));
            }
            // The order is the paper the money is against, so the order is
            // what bounds it. `supplier_debt::pay` measures against the whole
            // balance instead, which would refuse the ordinary case of paying
            // for goods that have not arrived yet.
            if paid.amount > landed.total {
                return Err(CoreError::validation(
                    "paid_now_centimes",
                    "more was handed over than the order is worth",
                ));
            }
        }

        let purchase = repo::insert(
            conn,
            &PurchaseRowWrite {
                shop_id,
                supplier_id: new.supplier_id,
                supplier_document_number,
                purchase_date: new.purchase_date.clone(),
                due_date: new.due_date.clone(),
                transport_centimes: new.transport.as_centimes(),
                extra_costs_centimes: new.extra_costs.as_centimes(),
                status: PurchaseStatus::Ordered,
                user_id,
                note,
            },
        )?;
        let mut written = Vec::with_capacity(new.lines.len());
        for (line, landed_unit_cost) in new.lines.iter().zip(&landed.per_line) {
            written.push(repo::insert_line(
                conn,
                &PurchaseLineRowWrite {
                    shop_id,
                    purchase_id: purchase.id,
                    product_id: line.product_id,
                    qty_ordered_milli: line.qty_ordered_milli,
                    unit_cost_centimes: line.unit_cost.as_centimes(),
                    landed_unit_cost_centimes: landed_unit_cost.as_centimes(),
                    qty_received_milli: 0,
                    qty_returned_milli: 0,
                },
            )?);
        }

        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_CREATE_PURCHASE,
                entity: "purchase",
                entity_id: Some(purchase.id),
                before: None,
                after: Some(
                    serde_json::json!({
                        "supplier_id": new.supplier_id,
                        "purchase_date": new.purchase_date,
                        "lines": written.len(),
                        "landed_total_centimes": landed.total.as_centimes(),
                        "transport_centimes": new.transport.as_centimes(),
                        "extra_costs_centimes": new.extra_costs.as_centimes(),
                        "received_now": new.receive_now,
                    })
                    .to_string(),
                ),
            },
        )?;

        if new.receive_now {
            let whole: Vec<ReceiveLine> = written
                .iter()
                .map(|l| ReceiveLine {
                    purchase_line_id: l.id,
                    qty_milli: l.qty_ordered_milli,
                })
                .collect();
            receive_inside(conn, shop_id, user_id, purchase.id, &whole, None, None)?;
        }

        if let Some(paid) = new.paid_now {
            hand_over(conn, shop_id, user_id, new.supplier_id, purchase.id, paid)?;
        }

        get(conn, shop_id, purchase.id)
    })
}

/// A delivery against an order: what arrived, the bon de réception it is
/// written on, the stock it put on the shelf and the debt it left behind.
/// One transaction for all of it.
///
/// `received_at` is the moment the goods were taken in; `None` is now on the
/// shop's clock. It decides the year the number comes out of and the day the
/// ledger row lands on, so the two cannot disagree.
pub fn receive(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    purchase_id: i32,
    lines: Vec<ReceiveLine>,
    note: Option<String>,
) -> Result<PurchaseView, CoreError> {
    let note = optional_field("note", note.as_deref())?;
    conn.transaction(|conn| {
        receive_inside(conn, shop_id, user_id, purchase_id, &lines, note, None)?;
        get(conn, shop_id, purchase_id)
    })
}

/// Goods handed back to the supplier. The stock leaves the shelf again and
/// the ledger takes the value off what the shop owes; a return writes no
/// document, because the movement and the ledger row are the record
/// (plan decision, 2026-09-10).
///
/// An order already settled goes below zero on the balance, which is credit
/// the supplier is holding, exactly as it is on the customer side.
pub fn return_to_supplier(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    purchase_id: i32,
    lines: Vec<ReceiveLine>,
    note: Option<String>,
) -> Result<PurchaseView, CoreError> {
    let note = optional_field("note", note.as_deref())?;
    if lines.is_empty() {
        return Err(CoreError::validation(
            "lines",
            "a return with no line sends nothing back",
        ));
    }
    conn.transaction(|conn| {
        let purchase = repo::get(conn, shop_id, purchase_id)?;
        let at = clock::now();
        let ordered = repo::lines(conn, shop_id, purchase_id)?;
        no_line_twice(&lines)?;
        let mut value = Money::ZERO;
        for back in &lines {
            let line = one_of(&ordered, back.purchase_line_id)?;
            let outstanding = line
                .qty_received_milli
                .checked_sub(line.qty_returned_milli)
                .ok_or(crate::money::MoneyError::Overflow)?;
            if back.qty_milli <= 0 {
                return Err(CoreError::validation(
                    "qty_milli",
                    "a return sends back more than nothing",
                ));
            }
            if back.qty_milli > outstanding {
                return Err(CoreError::validation(
                    "qty_milli",
                    "goods the shop never took in are goods it cannot send back",
                ));
            }
            repo::add_returned(conn, shop_id, line.id, back.qty_milli)?;
            let out = back
                .qty_milli
                .checked_neg()
                .ok_or(crate::money::MoneyError::Overflow)?;
            // The same word the customer side uses for goods coming back, and
            // the sign is what says which way they went.
            stock::record(
                conn,
                shop_id,
                &Movement {
                    product_id: line.product_id,
                    kind: MovementKind::Return,
                    qty_milli: out,
                    unit_cost: line.landed_unit_cost,
                    document_id: None,
                    user_id,
                },
            )?;
            value = value.checked_add(line.landed_unit_cost.checked_mul_milli(back.qty_milli)?)?;
        }
        // Goods worth nothing take nothing off the account, and a movement of
        // zero would sit in every statement the shop ever reads.
        if value != Money::ZERO {
            supplier_debt::append_at(
                conn,
                shop_id,
                supplier_debt::NewSupplierEntry {
                    supplier_id: purchase.supplier_id,
                    purchase_id: Some(purchase_id),
                    kind: supplier_debt::SupplierDebtKind::Return,
                    debit: Money::ZERO,
                    credit: value,
                    user_id,
                    note: note.clone(),
                },
                Some(at),
            )?;
        }
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_RETURN_PURCHASE,
                entity: "purchase",
                entity_id: Some(purchase_id),
                before: None,
                after: Some(
                    serde_json::json!({
                        "supplier_id": purchase.supplier_id,
                        "value_centimes": value.as_centimes(),
                        "lines": lines.len(),
                        "note": note,
                    })
                    .to_string(),
                ),
            },
        )?;
        get(conn, shop_id, purchase_id)
    })
}

/// An order that never happened. Only while nothing has arrived: goods on the
/// shelf say the order did happen, and what closes an order the rest of which
/// will never come is `close_short`.
pub fn cancel(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    purchase_id: i32,
    reason: String,
) -> Result<PurchaseView, CoreError> {
    let reason = required_reason(&reason)?;
    conn.transaction(|conn| {
        let purchase = repo::get(conn, shop_id, purchase_id)?;
        if purchase.status != PurchaseStatus::Ordered {
            return Err(CoreError::validation(
                "status",
                "an order is cancelled only while nothing has arrived against it",
            ));
        }
        if !repo::receipts(conn, shop_id, purchase_id)?.is_empty() {
            return Err(CoreError::validation(
                "status",
                "an order is cancelled only while nothing has arrived against it",
            ));
        }
        finish(
            conn,
            shop_id,
            user_id,
            purchase_id,
            PurchaseStatus::Cancelled,
            audit::ACTION_CANCEL_PURCHASE,
            reason,
        )
    })
}

/// An order the rest of which will never come. What arrived stays on the
/// shelf and on the account; the part that never came is written off, which
/// is a decision, so the reason goes into the log.
pub fn close_short(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    purchase_id: i32,
    reason: String,
) -> Result<PurchaseView, CoreError> {
    let reason = required_reason(&reason)?;
    conn.transaction(|conn| {
        let purchase = repo::get(conn, shop_id, purchase_id)?;
        if purchase.status != PurchaseStatus::PartiallyReceived {
            return Err(CoreError::validation(
                "status",
                "an order is closed short after part of it has arrived and not before",
            ));
        }
        finish(
            conn,
            shop_id,
            user_id,
            purchase_id,
            PurchaseStatus::ClosedShort,
            audit::ACTION_CLOSE_SHORT_PURCHASE,
            reason,
        )
    })
}

/// The last state an order moves to, with the reason the caller had to give.
fn finish(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    purchase_id: i32,
    status: PurchaseStatus,
    action: &'static str,
    reason: String,
) -> Result<PurchaseView, CoreError> {
    repo::set_status(conn, shop_id, purchase_id, status)?;
    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action,
            entity: "purchase",
            entity_id: Some(purchase_id),
            before: None,
            after: Some(
                serde_json::json!({ "status": status.as_str(), "reason": reason }).to_string(),
            ),
        },
    )?;
    get(conn, shop_id, purchase_id)
}

/// The whole receipt, inside the caller's transaction. `save` calls it for an
/// order whose goods came with the paper, and `receive` opens a transaction of
/// its own around it.
#[allow(clippy::too_many_arguments)]
fn receive_inside(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    purchase_id: i32,
    lines: &[ReceiveLine],
    note: Option<String>,
    received_at: Option<NaiveDateTime>,
) -> Result<(), CoreError> {
    if lines.is_empty() {
        return Err(CoreError::validation(
            "lines",
            "a delivery with no line brought nothing",
        ));
    }
    no_line_twice(lines)?;
    let purchase = repo::get(conn, shop_id, purchase_id)?;
    match purchase.status {
        PurchaseStatus::Ordered | PurchaseStatus::PartiallyReceived => {}
        PurchaseStatus::Received => {
            return Err(CoreError::validation(
                "status",
                "the whole of this order has already arrived",
            ))
        }
        PurchaseStatus::Cancelled | PurchaseStatus::ClosedShort => {
            return Err(CoreError::validation(
                "status",
                "this order is closed and takes no more deliveries",
            ))
        }
    }
    // Goods arriving on a closed fiche are a mistake or a fiche somebody has
    // to reopen on purpose (`supplier_debt::ensure_active`).
    supplier_debt::ensure_active(conn, shop_id, purchase.supplier_id)?;

    let at = received_at.unwrap_or_else(clock::now);
    let ordered = repo::lines(conn, shop_id, purchase_id)?;
    // The number is taken inside the transaction, so a delivery that is
    // refused further down burns none of the series.
    let series = reception_series(at.year());
    let number = counters::take_next(conn, shop_id, series.clone())?;
    let receipt = repo::insert_receipt(
        conn,
        &PurchaseReceiptRowWrite {
            shop_id,
            purchase_id,
            series,
            number,
            received_at: at,
            user_id,
            note: note.clone(),
        },
    )?;

    let mut value = Money::ZERO;
    for arriving in lines {
        let line = one_of(&ordered, arriving.purchase_line_id)?;
        let outstanding = line
            .qty_ordered_milli
            .checked_sub(line.qty_received_milli)
            .ok_or(crate::money::MoneyError::Overflow)?;
        if arriving.qty_milli <= 0 {
            return Err(CoreError::validation(
                "qty_milli",
                "a delivery takes in more than nothing",
            ));
        }
        if arriving.qty_milli > outstanding {
            return Err(CoreError::validation(
                "qty_milli",
                "more arrived on this line than the order still asks for",
            ));
        }
        repo::add_received(conn, shop_id, line.id, arriving.qty_milli)?;
        repo::insert_receipt_line(
            conn,
            &PurchaseReceiptLineRowWrite {
                shop_id,
                purchase_id,
                receipt_id: receipt.id,
                purchase_line_id: line.id,
                qty_milli: arriving.qty_milli,
            },
        )?;
        // The movement names no document: a purchase is not one, and the
        // paper behind this row is the bon de réception.
        stock::record(
            conn,
            shop_id,
            &Movement {
                product_id: line.product_id,
                kind: MovementKind::Purchase,
                qty_milli: arriving.qty_milli,
                unit_cost: line.landed_unit_cost,
                document_id: None,
                user_id,
            },
        )?;
        // What the product costs the shop is what the goods last landed at,
        // so a margin read tomorrow is read against today's delivery.
        products_repo::set_cost(conn, shop_id, line.product_id, line.landed_unit_cost)?;
        value = value.checked_add(
            line.landed_unit_cost
                .checked_mul_milli(arriving.qty_milli)?,
        )?;
    }

    // Debt follows goods: the debit is for the value that actually arrived,
    // at the cost it landed at. Goods worth nothing raise no debt, and a
    // movement of zero would sit in every statement for ever.
    if value != Money::ZERO {
        supplier_debt::append_at(
            conn,
            shop_id,
            supplier_debt::NewSupplierEntry {
                supplier_id: purchase.supplier_id,
                purchase_id: Some(purchase_id),
                kind: supplier_debt::SupplierDebtKind::Purchase,
                debit: value,
                credit: Money::ZERO,
                user_id,
                note: None,
            },
            Some(at),
        )?;
        // Credit the shop was already holding lands on the order now that the
        // order has a value to land on: an advance paid when the paper was
        // written is money this delivery is owed against.
        supplier_debt::place_credit_on(conn, shop_id, purchase_id, value)?;
    }

    let after = repo::lines(conn, shop_id, purchase_id)?;
    let whole = after
        .iter()
        .all(|l| l.qty_received_milli >= l.qty_ordered_milli);
    let status = if whole {
        PurchaseStatus::Received
    } else {
        PurchaseStatus::PartiallyReceived
    };
    repo::set_status(conn, shop_id, purchase_id, status)?;

    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action: audit::ACTION_RECEIVE_PURCHASE,
            entity: "purchase",
            entity_id: Some(purchase_id),
            before: Some(serde_json::json!({ "status": purchase.status.as_str() }).to_string()),
            after: Some(
                serde_json::json!({
                    "status": status.as_str(),
                    "receipt_id": receipt.id,
                    "receipt_number": receipt.number,
                    "series": receipt.series,
                    "value_centimes": value.as_centimes(),
                    "lines": lines.len(),
                })
                .to_string(),
            ),
        },
    )?;
    Ok(())
}

/// The `payment` row `paid_now` writes, and the orders it settles.
///
/// Not routed through `supplier_debt::pay`: that one refuses money above the
/// balance, which is the ordinary case here (an order paid the day it is
/// placed has no `purchase` row yet), and it writes an audit entry of its own
/// where this movement is part of saving an order. The row itself is written
/// exactly as `pay` writes it, mode and stamp included, and settled through
/// the same `settle_oldest_first`, so what a payment does to the orders is
/// decided in one place.
///
/// What no order can take stays on the balance as credit, and the next
/// receipt's `place_credit_on` puts it where it belongs.
fn hand_over(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    supplier_id: i32,
    purchase_id: i32,
    paid: Paid,
) -> Result<(), CoreError> {
    let at = clock::now();
    let before = debt_repo::balance(conn, shop_id, supplier_id)?;
    let entry = debt_repo::append(
        conn,
        &SupplierDebtRowWrite {
            shop_id,
            supplier_id,
            // A payment settles orders through its allocations, which can be
            // several: the column that names one order would have to pick.
            purchase_id: None,
            kind: supplier_debt::SupplierDebtKind::Payment,
            debit_centimes: 0,
            credit_centimes: paid.amount.as_centimes(),
            user_id,
            note: None,
            payment_mode: Some(paid.mode),
            created_at: Some(at),
        },
    )?;
    let allocations =
        supplier_debt::settle_oldest_first(conn, shop_id, supplier_id, entry.id, paid.amount)?;
    let after = debt_repo::balance(conn, shop_id, supplier_id)?;
    // The same entry `supplier_debt::pay` writes, under the same action: this
    // is the one path on which money leaves the drawer while an order is being
    // saved, and a log that stayed quiet about it would be the only payment to
    // a supplier nobody can retrace. The order it came in with is named
    // beside it, which `pay` has nothing to name.
    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action: audit::ACTION_PAY_SUPPLIER,
            entity: "supplier_debt",
            entity_id: Some(supplier_id),
            before: Some(
                serde_json::json!({ "balance_centimes": before.as_centimes() }).to_string(),
            ),
            after: Some(
                serde_json::json!({
                    "balance_centimes": after.as_centimes(),
                    "amount_centimes": paid.amount.as_centimes(),
                    "payment_mode": paid.mode.as_str(),
                    "ledger_id": entry.id,
                    "purchase_id": purchase_id,
                    "allocations": allocations
                        .iter()
                        .map(|a| serde_json::json!({
                            "purchase_id": a.purchase_id,
                            "amount_centimes": a.amount.as_centimes(),
                        }))
                        .collect::<Vec<_>>(),
                })
                .to_string(),
            ),
        },
    )?;
    Ok(())
}

/// Every line's landed unit cost, and what the order is worth once they are
/// applied.
struct Landed {
    per_line: Vec<Money>,
    /// The sum of the landed line totals: what the whole order will put on the
    /// supplier's account if all of it arrives.
    total: Money,
}

/// Spreads the transport and the extra costs over the lines by value and
/// divides each share per unit.
///
/// By value, because a share by quantity would divide kilos and pieces by
/// each other. The shares are floored and the remainder goes to the last
/// line, so the shares add up to the extra costs exactly; the per-unit
/// division is floored too, which is the one place the order loses centimes:
/// a line's landed total can come out under its value plus its share by less
/// than one unit's worth. Nothing is ever gained, so the sum of the landed
/// line totals is never above the lines' value plus the extra costs, which is
/// what `purchase_prop` pins.
fn spread(
    conn: &mut SqliteConnection,
    shop_id: i32,
    new: &NewPurchase,
) -> Result<Landed, CoreError> {
    let mut values = Vec::with_capacity(new.lines.len());
    let mut seen: Vec<i32> = Vec::with_capacity(new.lines.len());
    let mut order_value = Money::ZERO;
    for line in &new.lines {
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
        products_repo::get(conn, shop_id, line.product_id)?;
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

    let extra = new.transport.checked_add(new.extra_costs)?;
    if extra != Money::ZERO && order_value == Money::ZERO {
        // A share of a line worth nothing is nothing, so there is no honest
        // place to put the amount. Refused on the field that carries most of
        // it rather than spread by quantity, which would add kilos to pieces.
        return Err(CoreError::validation(
            "transport_centimes",
            "costs cannot be spread over lines that are worth nothing",
        ));
    }

    let mut per_line = Vec::with_capacity(new.lines.len());
    let mut given = Money::ZERO;
    let mut total = Money::ZERO;
    let last = new.lines.len() - 1;
    for (index, line) in new.lines.iter().enumerate() {
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

/// The ordered line a delivery or a return names, or a `NotFound` on the
/// line: a caller naming a line of another order would otherwise reach the
/// composite foreign key as a storage failure.
fn one_of(ordered: &[PurchaseLine], line_id: i32) -> Result<&PurchaseLine, CoreError> {
    ordered
        .iter()
        .find(|l| l.id == line_id)
        .ok_or(CoreError::NotFound {
            entity: "purchase_line",
            id: line_id,
        })
}

/// One line said once. Twice on one paper, the file's `UNIQUE (receipt_id,
/// purchase_line_id)` would refuse the second as a storage failure, and a
/// return has no such key at all.
fn no_line_twice(lines: &[ReceiveLine]) -> Result<(), CoreError> {
    let mut seen: Vec<i32> = Vec::with_capacity(lines.len());
    for line in lines {
        if seen.contains(&line.purchase_line_id) {
            return Err(CoreError::validation(
                "purchase_line_id",
                "a line is named once on one paper",
            ));
        }
        seen.push(line.purchase_line_id);
    }
    Ok(())
}

fn required_reason(reason: &str) -> Result<String, CoreError> {
    optional_field("reason", Some(reason))?.ok_or_else(|| {
        CoreError::validation(
            "reason",
            "closing an order this way is a decision, so it carries a reason",
        )
    })
}

/// A day on the shop's calendar, `YYYY-MM-DD`. Parsed rather than trusted:
/// the column is TEXT and the list, the dashboard and the oldest-first
/// settlement all order by it, so a day spelled another way sorts on its own.
fn parse_day(field: &str, value: &str) -> Result<NaiveDate, CoreError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| CoreError::validation(field, "a day is written YYYY-MM-DD"))
}
