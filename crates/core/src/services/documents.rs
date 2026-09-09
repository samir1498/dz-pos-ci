//! Documents and their numbering (features.md §3). One uninterrupted series
//! per kind, assigned at issue, never reused (décret 05-468 art. 10).
//!
//! There is no yearly reset of a series. It is common practice and it is not
//! in the decree, so the comptable answers question R8 before it becomes a
//! setting; until then a series runs on across years.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::document::{
    payment_mode_stored, regime_stored, DocumentLineRowWrite, DocumentRowWrite, DocumentTvaRowWrite,
};
use crate::money::Money;
use crate::repos::counters;
use crate::repos::documents as repo;

pub use crate::models::document::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, NewDocument,
    NewDocumentLine, PartyBlock, PartyKind, SellerBlock,
};

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Document, CoreError> {
    repo::get(conn, shop_id, id)
}

/// Newest first. `kind` of `None` lists every kind.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    kind: Option<DocumentKind>,
) -> Result<Vec<Document>, CoreError> {
    repo::list(conn, shop_id, kind)
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
        let series = new.kind.series();
        let number = counters::take_next(conn, shop_id, series)?;
        let totals = &new.totals;
        let id = repo::insert(
            conn,
            &DocumentRowWrite {
                shop_id,
                kind: new.kind,
                series: series.to_string(),
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
                },
            )?;
        }

        // Under the IFU there are no TVA rows at all, so the recap is empty
        // and nothing is written (features.md, regime_ifu_prints_no_tva).
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
