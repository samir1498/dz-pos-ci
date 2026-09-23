//! The only place documents touch diesel. Every query is scoped by `shop_id`
//! (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use chrono::NaiveDateTime;

use crate::error::CoreError;
use crate::models::document::{
    assemble, payment_mode_parse, CancelWrite, Document, DocumentKind, DocumentLineRow,
    DocumentLineRowWrite, DocumentRow, DocumentRowWrite, DocumentStatus, DocumentTvaRow,
    DocumentTvaRowWrite, RungSale,
};
use crate::money::Money;
use crate::schema::{document_lines, document_tva, documents, products};

/// Whether the series string ends in the year the row says it counts in:
/// `doc_ticket:2026` against 2026. The year has to be the whole last segment,
/// so `doc_ticket:12026` is not a 2026 series and neither is the yearless
/// `doc_ticket` the scheme before this one wrote.
fn series_names_its_year(series: &str, year: i32) -> bool {
    series
        .strip_suffix(&year.to_string())
        .is_some_and(|stem| stem.ends_with(':'))
}

pub fn insert(conn: &mut SqliteConnection, write: &DocumentRowWrite) -> Result<i32, CoreError> {
    // The series string and the year are one fact stored twice: the counter
    // takes the next number by the string and every printed number is spelled
    // out of the column (features.md §4, Numbering). A row where the two
    // disagree hands one number out under two spellings, and no later reader
    // can tell which half is right. Refused here rather than by a CHECK
    // because a table-level CHECK cannot be added to an existing SQLite
    // table; `up.sql` says where it goes when `documents` is next rebuilt.
    if !series_names_its_year(&write.series, write.series_year) {
        return Err(CoreError::validation(
            "series",
            "the series does not name the year the document says it was numbered in",
        ));
    }
    let id: i32 = diesel::insert_into(documents::table)
        .values(write)
        .returning(documents::id)
        .get_result(conn)?;
    Ok(id)
}

pub fn insert_line(
    conn: &mut SqliteConnection,
    write: &DocumentLineRowWrite,
) -> Result<(), CoreError> {
    diesel::insert_into(document_lines::table)
        .values(write)
        .execute(conn)?;
    Ok(())
}

pub fn insert_tva(
    conn: &mut SqliteConnection,
    write: &DocumentTvaRowWrite,
) -> Result<(), CoreError> {
    diesel::insert_into(document_tva::table)
        .values(write)
        .execute(conn)?;
    Ok(())
}

/// Whether the product is one of this shop's. A line points at a product by
/// id, and the foreign key alone would let another shop's row through.
pub fn product_belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    product_id: i32,
) -> Result<bool, CoreError> {
    let found: Option<i32> = products::table
        .filter(products::shop_id.eq(shop_id))
        .filter(products::id.eq(product_id))
        .select(products::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

/// Whether the line is one of this shop's and belongs to the document the
/// caller named. An avoir line credits a facture line by id, and the foreign
/// key alone would take a line of another shop's facture (rule 3) or a line of
/// some third document the avoir never refers to.
pub fn line_belongs_to_document(
    conn: &mut SqliteConnection,
    shop_id: i32,
    line_id: i32,
    document_id: i32,
) -> Result<bool, CoreError> {
    let found: Option<i32> = document_lines::table
        .filter(document_lines::shop_id.eq(shop_id))
        .filter(document_lines::id.eq(line_id))
        .filter(document_lines::document_id.eq(document_id))
        .select(document_lines::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

/// Every avoir written against one document, oldest first: the order they were
/// issued in, which is the order a facture's credit notes are read in.
pub fn avoirs_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<Vec<Document>, CoreError> {
    let rows: Vec<DocumentRow> = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::ref_document_id.eq(document_id))
        .filter(documents::kind.eq(DocumentKind::Avoir))
        .order((documents::issued_at.asc(), documents::id.asc()))
        .select(DocumentRow::as_select())
        .load(conn)?;
    rows.into_iter()
        .map(|row| with_children(conn, shop_id, row))
        .collect()
}

/// Whether the document is one of this shop's, without reading it whole. A
/// debt allocation names a document by id and nothing else.
pub fn belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<bool, CoreError> {
    let found: Option<i32> = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::id.eq(document_id))
        .select(documents::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

/// One document's id, what is still unpaid on it and what it asked for, for
/// every document of this customer that still carries debt, oldest first.
///
/// Only a document that still stands: a cancelled facture is not a debt any
/// more, whatever its `remaining_debt` column was left holding, and money
/// handed over settles the paper the customer can still be shown.
///
/// `issued_at` is whole seconds and two documents can land inside one, so the
/// id breaks the tie: settling oldest first has to mean one order and not
/// whichever order the file happened to answer in.
pub fn unpaid_of_customer(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Vec<(i32, i64, i64)>, CoreError> {
    let rows: Vec<(i32, Option<i64>, i64)> = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::customer_id.eq(customer_id))
        .filter(documents::status.eq(DocumentStatus::Issued))
        .filter(documents::remaining_debt_centimes.gt(0))
        .order((documents::issued_at.asc(), documents::id.asc()))
        .select((
            documents::id,
            documents::remaining_debt_centimes,
            documents::net_to_pay_centimes,
        ))
        .load(conn)?;
    // The filter above is what makes the column present, so a null here is a
    // row SQLite cannot answer with and the map is never taken.
    Ok(rows
        .into_iter()
        .filter_map(|(id, remaining, net)| remaining.map(|remaining| (id, remaining, net)))
        .collect())
}

/// Writes back what is left unpaid on one document. The only column of a
/// document a payment moves: the old balance and the total debt are what the
/// paper said on the day and a reprint has to keep saying it.
pub fn set_remaining_debt(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
    remaining_centimes: i64,
) -> Result<(), CoreError> {
    diesel::update(
        documents::table
            .filter(documents::shop_id.eq(shop_id))
            .filter(documents::id.eq(document_id)),
    )
    .set(documents::remaining_debt_centimes.eq(Some(remaining_centimes)))
    .execute(conn)?;
    Ok(())
}

/// Marks one document annulée and writes the block that says when, by whom,
/// why and with which avoir. The status and the four columns move in one
/// statement, so the row can never say it was cancelled without saying by whom
/// (`models::document::cancellation` refuses to read one that does).
pub fn set_cancelled(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
    write: &CancelWrite,
) -> Result<(), CoreError> {
    diesel::update(
        documents::table
            .filter(documents::shop_id.eq(shop_id))
            .filter(documents::id.eq(document_id)),
    )
    .set((documents::status.eq(DocumentStatus::Cancelled), write))
    .execute(conn)?;
    Ok(())
}

/// The kind, the series year and the number of the documents named, for the
/// shop asking: the three a printed number is spelled out of. A statement
/// prints a document number beside the movement that cites it, and reading
/// each document whole for three columns would be one query per line.
pub fn kinds_and_numbers(
    conn: &mut SqliteConnection,
    shop_id: i32,
    ids: &[i32],
) -> Result<Vec<(i32, DocumentKind, i32, i64)>, CoreError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(i32, DocumentKind, i32, i64)> = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::id.eq_any(ids))
        .select((
            documents::id,
            documents::kind,
            documents::series_year,
            documents::number,
        ))
        .load(conn)?;
    Ok(rows)
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Document, CoreError> {
    let row: DocumentRow = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::id.eq(id))
        .select(DocumentRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "document",
            id,
        })?;
    with_children(conn, shop_id, row)
}

/// Newest first. `issued_at` is whole seconds and a busy till issues two
/// tickets inside one, so the id breaks the tie: the later insert is the
/// later sale.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    kind: Option<DocumentKind>,
) -> Result<Vec<Document>, CoreError> {
    let mut query = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .into_boxed();
    if let Some(kind) = kind {
        query = query.filter(documents::kind.eq(kind));
    }
    let rows: Vec<DocumentRow> = query
        .order((documents::issued_at.desc(), documents::id.desc()))
        .select(DocumentRow::as_select())
        .load(conn)?;
    rows.into_iter()
        .map(|row| with_children(conn, shop_id, row))
        .collect()
}

/// Every kind, oldest first, over a stretch of days on the shop's calendar.
/// Both ends are inclusive and either may be absent, which is how a shop
/// asking for its whole history reaches this.
///
/// Oldest first rather than newest first: this is what the Excel export
/// reads, and a workbook of sales is read down the page in the order the
/// shop sold them. `issued_at` is already the shop's own calendar
/// (`services::clock`), so a day is a day here and needs no conversion.
pub fn list_in_range(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: Option<chrono::NaiveDate>,
    to: Option<chrono::NaiveDate>,
) -> Result<Vec<Document>, CoreError> {
    let mut query = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .into_boxed();
    if let Some(from) = from.and_then(|d| d.and_hms_opt(0, 0, 0)) {
        query = query.filter(documents::issued_at.ge(from));
    }
    // Half-open at the top: everything before midnight opening the day
    // after, rather than everything up to and including 23:59:59.
    //
    // 23:59:59 is a second, not the end of a day. A timestamp carrying a
    // fraction of it sorts above the bound, so a sale rung up at 23:59:59.4
    // fell out of a range that names the day it was issued on, and the
    // comptable reading that month got a file with the last sale missing
    // and nothing on the page to say so. `succ_opt` is the day after, and
    // it fails only past the end of the calendar, where a `to` no shop
    // typed leaves the range open at the top rather than empty.
    if let Some(after) = to
        .and_then(|d| d.succ_opt())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
    {
        query = query.filter(documents::issued_at.lt(after));
    }
    let rows: Vec<DocumentRow> = query
        .order((documents::issued_at.asc(), documents::id.asc()))
        .select(DocumentRow::as_select())
        .load(conn)?;
    rows.into_iter()
        .map(|row| with_children(conn, shop_id, row))
        .collect()
}

/// What one person rang up over one half-open stretch of the clock, oldest
/// first, four columns a row. A ticket or a facture that still stands, the
/// same set `repos::cash::sales` sums: a proforma is a quotation nobody paid,
/// an avoir is a credit note, and an annulled ticket is money that did not
/// stay in the drawer.
///
/// `issued_at` and not `created_at`, for the reason `RungSale` gives.
///
/// The bounds are half open, `>= from` and `< until`, the same shape every
/// query in this directory takes over a stretch of time.
pub fn rung_by(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Vec<RungSale>, CoreError> {
    let rows: Vec<(i32, NaiveDateTime, String, i64)> = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::user_id.eq(user_id))
        .filter(documents::kind.eq_any([DocumentKind::Ticket, DocumentKind::Facture]))
        .filter(documents::status.eq(DocumentStatus::Issued))
        .filter(documents::issued_at.ge(from))
        .filter(documents::issued_at.lt(until))
        .order((documents::issued_at.asc(), documents::id.asc()))
        .select((
            documents::id,
            documents::issued_at,
            documents::payment_mode,
            documents::net_to_pay_centimes,
        ))
        .load(conn)?;
    rows.into_iter()
        .map(|(document_id, issued_at, mode, net_to_pay)| {
            Ok(RungSale {
                document_id,
                issued_at,
                payment_mode: payment_mode_parse(&mode)?,
                net_to_pay: Money::centimes(net_to_pay),
            })
        })
        .collect()
}

fn with_children(
    conn: &mut SqliteConnection,
    shop_id: i32,
    row: DocumentRow,
) -> Result<Document, CoreError> {
    let lines: Vec<DocumentLineRow> = document_lines::table
        .filter(document_lines::shop_id.eq(shop_id))
        .filter(document_lines::document_id.eq(row.id))
        .order(document_lines::position.asc())
        .select(DocumentLineRow::as_select())
        .load(conn)?;
    let tva: Vec<DocumentTvaRow> = document_tva::table
        .filter(document_tva::shop_id.eq(shop_id))
        .filter(document_tva::document_id.eq(row.id))
        .order(document_tva::rate_bps.asc())
        .select(DocumentTvaRow::as_select())
        .load(conn)?;
    assemble(row, lines, tva)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../../tests/unit/repos_documents.rs"]
mod tests;
