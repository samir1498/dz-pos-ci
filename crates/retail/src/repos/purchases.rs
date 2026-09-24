//! The only place purchases, their lines and their bons de réception touch
//! diesel. Every query is scoped by `shop_id` (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::purchase::{
    Purchase, PurchaseLine, PurchaseLineRow, PurchaseLineRowWrite, PurchaseReceipt,
    PurchaseReceiptLine, PurchaseReceiptLineRow, PurchaseReceiptLineRowWrite, PurchaseReceiptRow,
    PurchaseReceiptRowWrite, PurchaseRow, PurchaseRowWrite, PurchaseStatus,
};
use crate::schema::{purchase_lines, purchase_receipt_lines, purchase_receipts, purchases};

pub fn insert(
    conn: &mut SqliteConnection,
    write: &PurchaseRowWrite,
) -> Result<Purchase, CoreError> {
    // The file cannot hold `number >= 1` as a CHECK (migration 000029 says
    // why), so the one door every purchase comes in by holds it, with the
    // year the number counts in, which has to be the order's own.
    if write.number < 1 {
        return Err(CoreError::validation(
            "number",
            "a purchase is numbered from 1",
        ));
    }
    if write.purchase_date.get(..4) != Some(write.series_year.to_string().as_str()) {
        return Err(CoreError::validation(
            "series_year",
            "a purchase counts in the year it is dated",
        ));
    }
    let row: PurchaseRow = diesel::insert_into(purchases::table)
        .values(write)
        .returning(PurchaseRow::as_returning())
        .get_result(conn)?;
    Ok(Purchase::from(row))
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Purchase, CoreError> {
    let row: PurchaseRow = purchases::table
        .filter(purchases::shop_id.eq(shop_id))
        .filter(purchases::id.eq(id))
        .select(PurchaseRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "purchase",
            id,
        })?;
    Ok(Purchase::from(row))
}

/// The shop's purchases, newest first. `purchase_date` is a day and two
/// orders land on one often, so the id breaks the tie: the later insert is
/// the later order.
pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Purchase>, CoreError> {
    let rows: Vec<PurchaseRow> = purchases::table
        .filter(purchases::shop_id.eq(shop_id))
        .order((purchases::purchase_date.desc(), purchases::id.desc()))
        .select(PurchaseRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Purchase::from).collect())
}

/// One supplier's orders, oldest first: the order money fills them in
/// (features.md §2, oldest-first settlement). `purchase_date` is a day and
/// two orders land on one often, so the id breaks the tie, and the id is the
/// order they were written in.
pub fn of_supplier(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<Vec<i32>, CoreError> {
    Ok(purchases::table
        .filter(purchases::shop_id.eq(shop_id))
        .filter(purchases::supplier_id.eq(supplier_id))
        .order((purchases::purchase_date.asc(), purchases::id.asc()))
        .select(purchases::id)
        .load(conn)?)
}

/// Moves the purchase to another state. The only column of a purchase this
/// layer updates: everything else about an order is settled when it is saved,
/// and what has arrived is counted on the lines.
pub fn set_status(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    status: PurchaseStatus,
) -> Result<Purchase, CoreError> {
    let changed = diesel::update(
        purchases::table
            .filter(purchases::shop_id.eq(shop_id))
            .filter(purchases::id.eq(id)),
    )
    .set(purchases::status.eq(status))
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "purchase",
            id,
        });
    }
    get(conn, shop_id, id)
}

pub fn insert_line(
    conn: &mut SqliteConnection,
    write: &PurchaseLineRowWrite,
) -> Result<PurchaseLine, CoreError> {
    let row: PurchaseLineRow = diesel::insert_into(purchase_lines::table)
        .values(write)
        .returning(PurchaseLineRow::as_returning())
        .get_result(conn)?;
    Ok(PurchaseLine::from(row))
}

/// One purchase's lines, in the order they were saved: the order the extra
/// costs were spread over, so the line that took the rounding remainder is
/// the last of these.
pub fn lines(
    conn: &mut SqliteConnection,
    shop_id: i32,
    purchase_id: i32,
) -> Result<Vec<PurchaseLine>, CoreError> {
    let rows: Vec<PurchaseLineRow> = purchase_lines::table
        .filter(purchase_lines::shop_id.eq(shop_id))
        .filter(purchase_lines::purchase_id.eq(purchase_id))
        .order(purchase_lines::id.asc())
        .select(PurchaseLineRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(PurchaseLine::from).collect())
}

/// Adds what one receipt brought in to the line's running total. Written as a
/// column expression rather than a read and a write, so two receipts keyed at
/// once cannot both read the same total and each store their own. The file's
/// CHECK refuses a total above what was ordered; how much a single receipt
/// may add is the service's rule.
///
/// A quantity of nothing or less is refused here rather than by the file: a
/// column expression that adds zero writes a row and reports success, and one
/// that adds a negative walks the total backwards inside a CHECK that only
/// looks at the ceiling. Both are a caller sending the wrong number, which is
/// what a validation error says.
pub fn add_received(
    conn: &mut SqliteConnection,
    shop_id: i32,
    line_id: i32,
    qty_milli: i64,
) -> Result<PurchaseLine, CoreError> {
    at_least_one(qty_milli, "qty_milli", "a receipt takes in")?;
    bump(
        conn,
        shop_id,
        line_id,
        purchase_lines::qty_received_milli.eq(purchase_lines::qty_received_milli + qty_milli),
    )
}

/// The same for what went back to the supplier. The file keeps the total at
/// or under what arrived; this keeps the step itself a real movement.
pub fn add_returned(
    conn: &mut SqliteConnection,
    shop_id: i32,
    line_id: i32,
    qty_milli: i64,
) -> Result<PurchaseLine, CoreError> {
    at_least_one(qty_milli, "qty_milli", "a return sends back")?;
    bump(
        conn,
        shop_id,
        line_id,
        purchase_lines::qty_returned_milli.eq(purchase_lines::qty_returned_milli + qty_milli),
    )
}

fn at_least_one(qty_milli: i64, field: &str, what: &str) -> Result<(), CoreError> {
    if qty_milli <= 0 {
        return Err(CoreError::Validation {
            field: field.to_string(),
            message: format!("{what} more than nothing"),
        });
    }
    Ok(())
}

/// One line's running total moved by a column expression, scoped by shop.
fn bump<C>(
    conn: &mut SqliteConnection,
    shop_id: i32,
    line_id: i32,
    change: C,
) -> Result<PurchaseLine, CoreError>
where
    C: diesel::query_builder::AsChangeset<Target = purchase_lines::table>,
    C::Changeset: diesel::query_builder::QueryFragment<diesel::sqlite::Sqlite>,
{
    let row: PurchaseLineRow = diesel::update(
        purchase_lines::table
            .filter(purchase_lines::shop_id.eq(shop_id))
            .filter(purchase_lines::id.eq(line_id)),
    )
    .set(change)
    .returning(PurchaseLineRow::as_returning())
    .get_result(conn)
    .optional()?
    .ok_or(CoreError::NotFound {
        entity: "purchase_line",
        id: line_id,
    })?;
    Ok(PurchaseLine::from(row))
}

pub fn insert_receipt(
    conn: &mut SqliteConnection,
    write: &PurchaseReceiptRowWrite,
) -> Result<PurchaseReceipt, CoreError> {
    let row: PurchaseReceiptRow = diesel::insert_into(purchase_receipts::table)
        .values(write)
        .returning(PurchaseReceiptRow::as_returning())
        .get_result(conn)?;
    Ok(PurchaseReceipt::from(row))
}

/// One purchase's deliveries, newest first: what arrived last is what somebody
/// opening the order is looking at.
pub fn receipts(
    conn: &mut SqliteConnection,
    shop_id: i32,
    purchase_id: i32,
) -> Result<Vec<PurchaseReceipt>, CoreError> {
    let rows: Vec<PurchaseReceiptRow> = purchase_receipts::table
        .filter(purchase_receipts::shop_id.eq(shop_id))
        .filter(purchase_receipts::purchase_id.eq(purchase_id))
        .order(purchase_receipts::id.desc())
        .select(PurchaseReceiptRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(PurchaseReceipt::from).collect())
}

pub fn insert_receipt_line(
    conn: &mut SqliteConnection,
    write: &PurchaseReceiptLineRowWrite,
) -> Result<PurchaseReceiptLine, CoreError> {
    let row: PurchaseReceiptLineRow = diesel::insert_into(purchase_receipt_lines::table)
        .values(write)
        .returning(PurchaseReceiptLineRow::as_returning())
        .get_result(conn)?;
    Ok(PurchaseReceiptLine::from(row))
}

/// What arrived on one delivery, in the order it was written.
pub fn receipt_lines(
    conn: &mut SqliteConnection,
    shop_id: i32,
    receipt_id: i32,
) -> Result<Vec<PurchaseReceiptLine>, CoreError> {
    let rows: Vec<PurchaseReceiptLineRow> = purchase_receipt_lines::table
        .filter(purchase_receipt_lines::shop_id.eq(shop_id))
        .filter(purchase_receipt_lines::receipt_id.eq(receipt_id))
        .order(purchase_receipt_lines::id.asc())
        .select(PurchaseReceiptLineRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(PurchaseReceiptLine::from).collect())
}

#[cfg(test)]
#[path = "../../tests/unit/repos_purchases.rs"]
mod tests;
