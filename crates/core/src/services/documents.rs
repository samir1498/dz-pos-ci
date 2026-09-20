//! Documents and their numbering (features.md §3). One uninterrupted series
//! per kind and per year, assigned at issue, never reused (décret 05-468
//! art. 10).
//!
//! A series restarts at 1 each year and the number carries the year
//! (`FA-2026-000001`): the common practice in Algeria, decided by Samir on
//! 2026-09-10 and not a rule of the decree, which asks only that the series be
//! uninterrupted. The comptable (R8) confirms the practice, not the choice.
//! There is no setting to turn the reset off.

use chrono::{Datelike, NaiveDateTime};
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::document::{
    payment_mode_stored, regime_stored, CancelWrite, DocumentLineRowWrite, DocumentRowWrite,
    DocumentTvaRowWrite,
};
use crate::money::Money;
use crate::repos::counters;
use crate::repos::documents as repo;
use crate::services::customers;

pub use crate::models::document::{
    BalanceTriple, Cancellation, Document, DocumentKind, DocumentLine, DocumentStatus, NewDocument,
    NewDocumentLine, PartyBlock, PartyKind, SellerBlock,
};

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Document, CoreError> {
    repo::get(conn, shop_id, id)
}

/// The document, only if it is of the kind the caller is asking for.
///
/// A route that prints one paper needs this rather than a kind check of its
/// own: a ticket handed to the facture template would be a document titled
/// FACTURE carrying a number out of the ticket series, and the honest answer
/// to "print the facture 7" when 7 is a ticket is that there is no such
/// facture. The entity name is the kind, so the 404 says which.
pub fn get_of_kind(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    kind: DocumentKind,
) -> Result<Document, CoreError> {
    let found = repo::get(conn, shop_id, id)?;
    if found.kind != kind {
        return Err(CoreError::NotFound {
            entity: kind.as_str(),
            id,
        });
    }
    Ok(found)
}

/// Newest first. `kind` of `None` lists every kind.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    kind: Option<DocumentKind>,
) -> Result<Vec<Document>, CoreError> {
    repo::list(conn, shop_id, kind)
}

/// Every avoir written against one document, oldest first: the order they
/// were issued in, which is the order a facture's credit notes are read in.
///
/// `pub(crate)` and not `pub`, like `mark_cancelled` below: the repo it
/// stands in front of is `pub(crate)` too (`lib.rs`), so a door widened to
/// `pub` would hand the rest of the workspace a read the crate had kept to
/// itself. It is also the raw read: `avoir::list_for` asks
/// `get_of_kind` first so a foreign shop's id or a document that is not a
/// facture answers `NotFound` rather than an empty list, and this answers
/// the empty list. Callers outside this crate want that function.
pub(crate) fn avoirs_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<Vec<Document>, CoreError> {
    repo::avoirs_of(conn, shop_id, document_id)
}

/// Every kind, oldest first, over a stretch of days on the shop's calendar.
/// Both ends are inclusive and either may be absent, which is how a shop
/// asking for its whole history reaches this.
///
/// `pub(crate)` for the reason `avoirs_of` above gives.
pub(crate) fn list_in_range(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: Option<chrono::NaiveDate>,
    to: Option<chrono::NaiveDate>,
) -> Result<Vec<Document>, CoreError> {
    repo::list_in_range(conn, shop_id, from, to)
}

/// Stamps the cancellation onto the document itself.
///
/// The decision and the rest of the undo belong to `services::cancellation`,
/// which sits above this module and `avoir`. What belongs here is the write
/// to a document, so cancelling goes through this module rather than round
/// it into its repo.
pub(crate) fn mark_cancelled(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
    at: NaiveDateTime,
    user_id: i32,
    reason: String,
    avoir_document_id: Option<i32>,
) -> Result<(), CoreError> {
    repo::set_cancelled(
        conn,
        shop_id,
        document_id,
        &CancelWrite {
            cancelled_at: at,
            cancelled_by: user_id,
            cancel_reason: reason,
            cancel_avoir_document_id: avoir_document_id,
        },
    )
}

/// Takes the next number of the kind's series and writes the document, its
/// lines and its TVA recap. The number and the rows commit or roll back
/// together, so a refused line burns no number and the series never gaps.
///
/// The counter hands numbers out one at a time with no dedupe loop, unlike
/// the in-store barcode: a document number has no second source that could
/// already be using it, so the counter is the whole answer.
pub fn issue(
    conn: &mut SqliteConnection,
    shop_id: i32,
    new: NewDocument,
) -> Result<Document, CoreError> {
    conn.transaction(|conn| {
        // Before a number is taken, because a refused document should cost
        // nothing at all: the rollback would give the number back, but the
        // two ids a caller hands over are known wrong or right without
        // touching the counter. The foreign keys alone would take the
        // neighbour's fiche and the neighbour's facture (rule 3).
        if let Some(customer_id) = new.customer_id {
            if !customers::customer_belongs_to_shop(conn, shop_id, customer_id)? {
                return Err(CoreError::NotFound {
                    entity: "customer",
                    id: customer_id,
                });
            }
        }
        if let Some(ref_document_id) = new.ref_document_id {
            if !repo::belongs_to_shop(conn, shop_id, ref_document_id)? {
                return Err(CoreError::NotFound {
                    entity: "document",
                    id: ref_document_id,
                });
            }
        }
        // The year the document is issued in, off its own `issued_at`, which
        // the callers read from the shop clock (UTC+1, no daylight saving).
        // Never `Utc::now().year()`: for one hour a night the two disagree,
        // and on 31 December that hour decides which year's series a document
        // belongs to.
        let series_year = new.issued_at.year();
        let series = new.kind.series_of_year(series_year);
        let number = counters::take_next(conn, shop_id, series.clone())?;
        let totals = &new.totals;
        let id = repo::insert(
            conn,
            &DocumentRowWrite {
                shop_id,
                kind: new.kind,
                series,
                series_year,
                number,
                issued_at: new.issued_at,
                user_id: new.user_id,
                regime: regime_stored(new.regime),
                payment_mode: payment_mode_stored(new.payment_mode),
                seller_name: new.seller.name.clone(),
                seller_rc: new.seller.rc.clone(),
                seller_nif: new.seller.nif.clone(),
                seller_nis: new.seller.nis.clone(),
                seller_ai: new.seller.ai.clone(),
                seller_address: new.seller.address.clone(),
                seller_phone: new.seller.phone.clone(),
                customer_id: new.customer_id,
                buyer_name: new.buyer.as_ref().map(|b| b.name.clone()),
                buyer_party_kind: new.buyer.as_ref().map(|b| b.party_kind),
                buyer_rc: new.buyer.as_ref().and_then(|b| b.rc.clone()),
                buyer_nif: new.buyer.as_ref().and_then(|b| b.nif.clone()),
                buyer_nis: new.buyer.as_ref().and_then(|b| b.nis.clone()),
                buyer_ai: new.buyer.as_ref().and_then(|b| b.ai.clone()),
                buyer_address: new.buyer.as_ref().and_then(|b| b.address.clone()),
                ref_document_id: new.ref_document_id,
                old_balance_centimes: new.balance.map(|b| b.old_balance.as_centimes()),
                remaining_debt_centimes: new.balance.map(|b| b.remaining_debt.as_centimes()),
                total_debt_centimes: new.balance.map(|b| b.total_debt.as_centimes()),
                total_ht_centimes: totals.total_ht.as_centimes(),
                discount_centimes: totals.discount.as_centimes(),
                subtotal_ht_centimes: totals.subtotal_ht.as_centimes(),
                tva_centimes: totals.tva.as_centimes(),
                total_ttc_centimes: totals.total_ttc.as_centimes(),
                stamp_centimes: totals.stamp.as_centimes(),
                net_to_pay_centimes: totals.net_to_pay.as_centimes(),
                tendered_centimes: new.tendered.map(Money::as_centimes),
                change_centimes: new.change.map(Money::as_centimes),
                status: DocumentStatus::Issued,
            },
        )?;

        for (position, line) in new.lines.iter().enumerate() {
            // Checked here rather than before the number is taken: the
            // foreign key alone would accept another shop's product, and the
            // rollback is what proves a refused line costs no number.
            if let Some(product_id) = line.product_id {
                if !repo::product_belongs_to_shop(conn, shop_id, product_id)? {
                    return Err(CoreError::NotFound {
                        entity: "product",
                        id: product_id,
                    });
                }
            }
            // A line that credits another names a line of the document this
            // one is written against. Checked here for the same two reasons
            // the product is: the foreign key alone would take a line of
            // another shop's facture (rule 3), and a line of some third
            // document nobody named would credit a paper this one does not
            // refer to.
            if let Some(ref_line_id) = line.ref_line_id {
                let Some(ref_document_id) = new.ref_document_id else {
                    return Err(CoreError::validation(
                        "ref_line_id",
                        "a line credits another only on a document written against one",
                    ));
                };
                if !repo::line_belongs_to_document(conn, shop_id, ref_line_id, ref_document_id)? {
                    return Err(CoreError::NotFound {
                        entity: "document_line",
                        id: ref_line_id,
                    });
                }
            }
            let position = i32::try_from(position)
                .map_err(|_| CoreError::validation("lines", "more lines than a document holds"))?;
            repo::insert_line(
                conn,
                &DocumentLineRowWrite {
                    shop_id,
                    document_id: id,
                    position,
                    product_id: line.product_id,
                    name: line.name.clone(),
                    barcode: line.barcode.clone(),
                    qty_milli: line.qty_milli,
                    unit_price_centimes: line.unit_price.as_centimes(),
                    line_discount_centimes: line.line_discount.as_centimes(),
                    rate_bps: i32::try_from(line.rate_bps.as_u32())
                        .map_err(|_| crate::money::MoneyError::RateOutOfRange)?,
                    line_total_centimes: line.line_total.as_centimes(),
                    ref_line_id: line.ref_line_id,
                },
            )?;
        }

        // Under the IFU there are no TVA rows at all, so the recap is empty
        // and nothing is written (features.md §3, `an_ifu_line_stores_no_rate_so_a_reprint_never_needs_the_regime`).
        for row in &totals.tva_by_rate {
            repo::insert_tva(
                conn,
                &DocumentTvaRowWrite {
                    shop_id,
                    document_id: id,
                    rate_bps: i32::try_from(row.rate.as_u32())
                        .map_err(|_| crate::money::MoneyError::RateOutOfRange)?,
                    base_centimes: row.base.as_centimes(),
                    amount_centimes: row.amount.as_centimes(),
                },
            )?;
        }

        repo::get(conn, shop_id, id)
    })
}
