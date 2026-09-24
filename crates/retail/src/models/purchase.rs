//! What the shop ordered from a supplier, and what actually arrived
//! (features.md §1, Purchase).
//!
//! A purchase is not a document and a bon de réception is not one either: the
//! `documents` table's régime, its payment mode and its customer key have no
//! honest value for something the shop buys, and its series are the ones the
//! tax code hands out for what the shop sells. The receipt takes its own
//! number out of the `reception:<year>` counter instead.
//!
//! Amounts are `Money` and quantities are thousandths of the unit, the way
//! the stock ledger writes them.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::error::CoreError;
use crate::money::Money;
use crate::schema::{purchase_lines, purchase_receipt_lines, purchase_receipts, purchases};

pub use super::sql_types::PurchaseStatus;

/// An order placed with a supplier, as the rest of the app sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purchase {
    pub id: i32,
    pub shop_id: i32,
    pub supplier_id: i32,
    /// The number written on the paper the supplier sent. `None` as often as
    /// a shop pleases: a delivery note with no number on it is a thing
    /// suppliers hand over.
    pub supplier_document_number: Option<String>,
    /// A day on the shop's calendar, `YYYY-MM-DD`, where `created_at` is the
    /// UTC moment the row was written.
    pub purchase_date: String,
    pub due_date: Option<String>,
    /// What the goods cost to get here. Agreed once for the whole order, so
    /// they sit on the purchase and not on a line; the service spreads them
    /// over the lines by value into each line's landed cost.
    pub transport: Money,
    pub extra_costs: Money,
    pub status: PurchaseStatus,
    pub user_id: i32,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
    /// The year the order's number counts in: the year of `purchase_date`.
    pub series_year: i32,
    /// The number inside that year, gapless per shop (migration 000029).
    pub number: i64,
}

/// The printed form of a purchase's number, `BA-2026-000001`: "bon d'achat",
/// our own number for an order, which the supplier's delivery-note number is
/// not.
pub const PURCHASE_PREFIX: &str = "BA";

/// The printed form of a bon de réception's number, `BR-2026-000001`. The
/// prefix is the one `DocumentKind::BonDeReception` prints, read from there so
/// the two cannot drift; no document of that kind is ever issued, a delivery
/// is a `purchase_receipts` row (features.md §1).
pub fn receipt_prefix() -> &'static str {
    super::sql_types::DocumentKind::BonDeReception.number_prefix()
}

/// The counter a purchase takes its number from in `year`. One series per
/// year, so the first order of January is number 1 (`take_next` creates the
/// row on first use).
pub fn purchase_series(year: i32) -> String {
    format!("purchase:{year}")
}

/// `{prefix}-{year}-{number:06}`, the shape every printed number has
/// (`print::number_of`).
pub fn printed_number(prefix: &str, year: i32, number: i64) -> String {
    format!("{prefix}-{year}-{number:06}")
}

impl Purchase {
    /// The number the shop quotes for this order, `BA-2026-000001`.
    pub fn printed_number(&self) -> String {
        printed_number(PURCHASE_PREFIX, self.series_year, self.number)
    }

    /// What the goods cost to get here, as one amount. Migration 000008 keeps
    /// `transport_centimes` and `extra_costs_centimes` in two columns because
    /// they are two things the shop agreed once for the whole order, and the
    /// service spreads them over the lines by value; their sum is charged to
    /// nobody and stored nowhere, which is why it is answered here. It is
    /// answered here and not on a screen because adding two amounts is the
    /// core's work: checked, so a pair that leaves the range is an error and
    /// not a number that wrapped quietly on the way to a print.
    pub fn extras(&self) -> Result<Money, CoreError> {
        Ok(self.transport.checked_add(self.extra_costs)?)
    }
}

/// A purchase as a caller hands it over, before it has an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPurchase {
    pub supplier_id: i32,
    pub supplier_document_number: Option<String>,
    pub purchase_date: String,
    pub due_date: Option<String>,
    pub transport: Money,
    pub extra_costs: Money,
    pub status: PurchaseStatus,
    pub user_id: i32,
    pub note: Option<String>,
}

/// One product on an order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseLine {
    pub id: i32,
    pub shop_id: i32,
    pub purchase_id: i32,
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    /// What the supplier charges for the unit.
    pub unit_cost: Money,
    /// That plus this line's share of the transport and the extra costs,
    /// fixed once when the purchase is saved: a margin is read against the
    /// cost the goods landed at, and a share recomputed later would move a
    /// cost a sale has already been measured against.
    pub landed_unit_cost: Money,
    /// The running total the receipts add up to, never above what was
    /// ordered.
    pub qty_received_milli: i64,
    /// What went back to the supplier, never above what arrived: goods the
    /// shop never took in are goods it cannot send back.
    pub qty_returned_milli: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPurchaseLine {
    pub purchase_id: i32,
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    pub unit_cost: Money,
    pub landed_unit_cost: Money,
    pub qty_received_milli: i64,
    pub qty_returned_milli: i64,
}

/// One delivery against one purchase: the bon de réception, kept and listed
/// like the paper it stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseReceipt {
    pub id: i32,
    pub shop_id: i32,
    pub purchase_id: i32,
    /// The counter the number came out of, `reception:<year>`.
    pub series: String,
    pub number: i64,
    /// A moment on the shop's calendar, the way a document's `issued_at` is:
    /// two deliveries land on one afternoon often enough, and the list and
    /// the statement both order by this.
    pub received_at: NaiveDateTime,
    pub user_id: i32,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
}

impl PurchaseReceipt {
    /// The number the shop quotes for this delivery, `BR-2026-000001`. The
    /// year is the one in the series key (`reception:2026`), which is the one
    /// the number came out of; a key that does not end in a year reads as 0
    /// rather than failing the whole order's page.
    pub fn printed_number(&self) -> String {
        let year = self
            .series
            .rsplit(':')
            .next()
            .and_then(|y| y.parse::<i32>().ok())
            .unwrap_or(0);
        printed_number(receipt_prefix(), year, self.number)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPurchaseReceipt {
    pub purchase_id: i32,
    pub series: String,
    pub number: i64,
    pub received_at: NaiveDateTime,
    pub user_id: i32,
    pub note: Option<String>,
}

/// What arrived on one delivery. A line that took nothing is not written: the
/// lines that were not delivered this time are simply absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseReceiptLine {
    pub id: i32,
    pub shop_id: i32,
    /// The purchase both the receipt and the line belong to. Stored so the
    /// file can tie the two together rather than trusting a caller to.
    pub purchase_id: i32,
    pub receipt_id: i32,
    pub purchase_line_id: i32,
    pub qty_milli: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPurchaseReceiptLine {
    pub purchase_id: i32,
    pub receipt_id: i32,
    pub purchase_line_id: i32,
    pub qty_milli: i64,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = purchases)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct PurchaseRow {
    pub id: i32,
    pub shop_id: i32,
    pub supplier_id: i32,
    pub supplier_document_number: Option<String>,
    pub purchase_date: String,
    pub due_date: Option<String>,
    pub transport_centimes: i64,
    pub extra_costs_centimes: i64,
    pub status: PurchaseStatus,
    pub user_id: i32,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
    pub series_year: i32,
    pub number: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = purchases)]
pub(crate) struct PurchaseRowWrite {
    pub shop_id: i32,
    pub supplier_id: i32,
    pub supplier_document_number: Option<String>,
    pub purchase_date: String,
    pub due_date: Option<String>,
    pub transport_centimes: i64,
    pub extra_costs_centimes: i64,
    pub status: PurchaseStatus,
    pub user_id: i32,
    pub note: Option<String>,
    pub series_year: i32,
    pub number: i64,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = purchase_lines)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct PurchaseLineRow {
    pub id: i32,
    pub shop_id: i32,
    pub purchase_id: i32,
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    pub unit_cost_centimes: i64,
    pub landed_unit_cost_centimes: i64,
    pub qty_received_milli: i64,
    pub qty_returned_milli: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = purchase_lines)]
pub(crate) struct PurchaseLineRowWrite {
    pub shop_id: i32,
    pub purchase_id: i32,
    pub product_id: i32,
    pub qty_ordered_milli: i64,
    pub unit_cost_centimes: i64,
    pub landed_unit_cost_centimes: i64,
    pub qty_received_milli: i64,
    pub qty_returned_milli: i64,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = purchase_receipts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct PurchaseReceiptRow {
    pub id: i32,
    pub shop_id: i32,
    pub purchase_id: i32,
    pub series: String,
    pub number: i64,
    pub received_at: NaiveDateTime,
    pub user_id: i32,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = purchase_receipts)]
pub(crate) struct PurchaseReceiptRowWrite {
    pub shop_id: i32,
    pub purchase_id: i32,
    pub series: String,
    pub number: i64,
    pub received_at: NaiveDateTime,
    pub user_id: i32,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = purchase_receipt_lines)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct PurchaseReceiptLineRow {
    pub id: i32,
    pub shop_id: i32,
    pub purchase_id: i32,
    pub receipt_id: i32,
    pub purchase_line_id: i32,
    pub qty_milli: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = purchase_receipt_lines)]
pub(crate) struct PurchaseReceiptLineRowWrite {
    pub shop_id: i32,
    pub purchase_id: i32,
    pub receipt_id: i32,
    pub purchase_line_id: i32,
    pub qty_milli: i64,
}

impl From<PurchaseRow> for Purchase {
    fn from(r: PurchaseRow) -> Self {
        Purchase {
            id: r.id,
            shop_id: r.shop_id,
            supplier_id: r.supplier_id,
            supplier_document_number: r.supplier_document_number,
            purchase_date: r.purchase_date,
            due_date: r.due_date,
            transport: Money::centimes(r.transport_centimes),
            extra_costs: Money::centimes(r.extra_costs_centimes),
            status: r.status,
            user_id: r.user_id,
            note: r.note,
            created_at: r.created_at,
            series_year: r.series_year,
            number: r.number,
        }
    }
}

impl From<PurchaseLineRow> for PurchaseLine {
    fn from(r: PurchaseLineRow) -> Self {
        PurchaseLine {
            id: r.id,
            shop_id: r.shop_id,
            purchase_id: r.purchase_id,
            product_id: r.product_id,
            qty_ordered_milli: r.qty_ordered_milli,
            unit_cost: Money::centimes(r.unit_cost_centimes),
            landed_unit_cost: Money::centimes(r.landed_unit_cost_centimes),
            qty_received_milli: r.qty_received_milli,
            qty_returned_milli: r.qty_returned_milli,
        }
    }
}

impl From<PurchaseReceiptRow> for PurchaseReceipt {
    fn from(r: PurchaseReceiptRow) -> Self {
        PurchaseReceipt {
            id: r.id,
            shop_id: r.shop_id,
            purchase_id: r.purchase_id,
            series: r.series,
            number: r.number,
            received_at: r.received_at,
            user_id: r.user_id,
            note: r.note,
            created_at: r.created_at,
        }
    }
}

impl From<PurchaseReceiptLineRow> for PurchaseReceiptLine {
    fn from(r: PurchaseReceiptLineRow) -> Self {
        PurchaseReceiptLine {
            id: r.id,
            shop_id: r.shop_id,
            purchase_id: r.purchase_id,
            receipt_id: r.receipt_id,
            purchase_line_id: r.purchase_line_id,
            qty_milli: r.qty_milli,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/models_purchase.rs"]
mod tests;
