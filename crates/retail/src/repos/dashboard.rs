//! The rows the dashboard is summed from. Every query is scoped by `shop_id`
//! (rule 3) and every SUM is coalesced to zero, so a day nothing happened on
//! answers zeros rather than nothing.
//!
//! Nothing here multiplies. A quantity times a cost is done in `Money` by
//! the service above, because SQLite turns an integer product that overflows
//! into a float and a float near a total is what this project does not do
//! (dz-money). The rows come back as they are stored and the arithmetic is
//! checked in Rust.
//!
//! What counts as a paper the figures read
//! - It still stands: an annulled ticket sold nothing and cost nothing.
//! - It is a ticket, a facture or an avoir: a proforma is a quotation
//!   nobody signed, and the papers the supply side writes are not sales.
//! - It is not written against a paper that was annulled. A cancelled
//!   facture leaves the figures with its own status, and the credit note the
//!   cancellation issued has to leave with it or the sale is reversed twice.
//!   The same clause drops an ordinary avoir written before its facture was
//!   annulled, which is right for the same reason: the facture it credited
//!   is gone from the figures, so the credit has nothing left to lower.

use chrono::NaiveDateTime;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Bool, Nullable};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::sql_types::{DocumentKind, DocumentStatus, PurchaseStatus};
use crate::schema::{document_lines, documents, products, purchases, stock_movements};

/// The three kinds a figure on this screen is ever read from.
const COUNTED_KINDS: [DocumentKind; 3] = [
    DocumentKind::Ticket,
    DocumentKind::Facture,
    DocumentKind::Avoir,
];

/// The clause that drops a credit note whose facture was annulled. Written
/// out rather than built with the dsl: it is a correlated subquery over the
/// same table, which diesel would need an alias for, and the only values in
/// it are two literals this file owns.
const NOT_CREDITING_AN_ANNULLED_PAPER: &str = "(documents.kind <> 'avoir' OR NOT EXISTS ( \
     SELECT 1 FROM documents ref WHERE ref.id = documents.ref_document_id \
     AND ref.shop_id = documents.shop_id AND ref.status = 'cancelled'))";

/// One kind's totals over the period: what its papers asked for, the remise
/// they gave off the whole document, and how many of them there were.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindTotals {
    pub kind: DocumentKind,
    pub total_ttc_centimes: i64,
    pub discount_centimes: i64,
    pub count: i64,
}

/// One line of a counted paper. The name is the line's own and not the
/// fiche's: it is what the paper printed, so a product renamed since reads
/// the way the customer's copy does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRow {
    pub kind: DocumentKind,
    pub product_id: Option<i32>,
    pub name: String,
    pub qty_milli: i64,
    pub line_total_centimes: i64,
}

/// One stock movement of a counted paper. Signed the way the ledger signs
/// it: a sale is negative, the return an avoir or a cancellation writes is
/// positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementRow {
    pub product_id: i32,
    pub qty_milli: i64,
    pub unit_cost_centimes: i64,
}

/// A product the shop is short of: what it has and what it wanted to have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowStockRow {
    pub product_id: i32,
    pub name: String,
    pub qty_on_hand_milli: i64,
    pub low_stock_at_milli: i64,
}

/// Every kind's totals over the period, one row per kind that has a paper.
pub fn totals_by_kind(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Vec<KindTotals>, CoreError> {
    let rows: Vec<(DocumentKind, Option<i64>, Option<i64>, i64)> = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::kind.eq_any(COUNTED_KINDS))
        .filter(documents::status.eq(DocumentStatus::Issued))
        .filter(documents::issued_at.ge(from))
        .filter(documents::issued_at.lt(until))
        .filter(sql::<Bool>(NOT_CREDITING_AN_ANNULLED_PAPER))
        .group_by(documents::kind)
        .select((
            documents::kind,
            sql::<Nullable<BigInt>>("SUM(total_ttc_centimes)"),
            sql::<Nullable<BigInt>>("SUM(discount_centimes)"),
            sql::<BigInt>("COUNT(*)"),
        ))
        .load(conn)?;
    Ok(rows
        .into_iter()
        .map(|(kind, ttc, discount, count)| KindTotals {
            kind,
            total_ttc_centimes: ttc.unwrap_or(0),
            discount_centimes: discount.unwrap_or(0),
            count,
        })
        .collect())
}

/// Every line of every counted paper of the period.
pub fn lines(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Vec<LineRow>, CoreError> {
    let rows: Vec<(DocumentKind, Option<i32>, String, i64, i64)> = document_lines::table
        .inner_join(documents::table.on(document_lines::document_id.eq(documents::id)))
        .filter(document_lines::shop_id.eq(shop_id))
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::kind.eq_any(COUNTED_KINDS))
        .filter(documents::status.eq(DocumentStatus::Issued))
        .filter(documents::issued_at.ge(from))
        .filter(documents::issued_at.lt(until))
        .filter(sql::<Bool>(NOT_CREDITING_AN_ANNULLED_PAPER))
        .select((
            documents::kind,
            document_lines::product_id,
            document_lines::name,
            document_lines::qty_milli,
            document_lines::line_total_centimes,
        ))
        .load(conn)?;
    Ok(rows
        .into_iter()
        .map(
            |(kind, product_id, name, qty_milli, line_total_centimes)| LineRow {
                kind,
                product_id,
                name,
                qty_milli,
                line_total_centimes,
            },
        )
        .collect())
}

/// Every stock movement written by a counted paper of the period.
///
/// Dated by the paper and never by the movement's own `created_at`: an avoir
/// backdated to the day it belongs on writes its rows at the moment the file
/// was touched, and the two sides of a margin have to be summed over one
/// calendar. The join is what drops a purchase and a return to a supplier
/// too: neither names a document.
pub fn movements(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Vec<MovementRow>, CoreError> {
    let rows: Vec<(i32, i64, i64)> = stock_movements::table
        .inner_join(documents::table.on(stock_movements::document_id.eq(documents::id.nullable())))
        .filter(stock_movements::shop_id.eq(shop_id))
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::kind.eq_any(COUNTED_KINDS))
        .filter(documents::status.eq(DocumentStatus::Issued))
        .filter(documents::issued_at.ge(from))
        .filter(documents::issued_at.lt(until))
        .filter(sql::<Bool>(NOT_CREDITING_AN_ANNULLED_PAPER))
        .select((
            stock_movements::product_id,
            stock_movements::qty_milli,
            stock_movements::unit_cost_centimes,
        ))
        .load(conn)?;
    Ok(rows
        .into_iter()
        .map(|(product_id, qty_milli, unit_cost_centimes)| MovementRow {
            product_id,
            qty_milli,
            unit_cost_centimes,
        })
        .collect())
}

/// The products in use whose count has fallen under the threshold somebody
/// set on them, the furthest under first.
///
/// A retired product is left out: it is not being reordered. The threshold is
/// compared as it stands, zero included, so a count that has gone negative
/// shows up whether or not anybody ever set one.
pub fn low_stock(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<LowStockRow>, CoreError> {
    let rows: Vec<(i32, String, i64, i64)> = products::table
        .filter(products::shop_id.eq(shop_id))
        .filter(products::active.eq(true))
        .filter(products::qty_on_hand_milli.lt(products::low_stock_at_milli))
        .order((
            sql::<BigInt>("qty_on_hand_milli - low_stock_at_milli").asc(),
            products::id.asc(),
        ))
        .select((
            products::id,
            products::name,
            products::qty_on_hand_milli,
            products::low_stock_at_milli,
        ))
        .load(conn)?;
    Ok(rows
        .into_iter()
        .map(
            |(product_id, name, qty_on_hand_milli, low_stock_at_milli)| LowStockRow {
                product_id,
                name,
                qty_on_hand_milli,
                low_stock_at_milli,
            },
        )
        .collect())
}

/// Purchases the shop is still waiting on: ordered, or delivered in part.
/// A cancelled one and one closed short are finished, whatever they left
/// undelivered.
pub fn open_purchases(conn: &mut SqliteConnection, shop_id: i32) -> Result<i64, CoreError> {
    let count: i64 = purchases::table
        .filter(purchases::shop_id.eq(shop_id))
        .filter(
            purchases::status.eq_any([PurchaseStatus::Ordered, PurchaseStatus::PartiallyReceived]),
        )
        .count()
        .get_result(conn)?;
    Ok(count)
}
