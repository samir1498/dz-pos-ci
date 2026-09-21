//! The proforma, the quotation a shop hands over before there is a sale
//! (features.md §3).
//!
//! It looks like a facture and is not one. It is made out to a named customer,
//! it prices a basket the way the till would price it, and it takes a number
//! out of its own series so that two quotations can be told apart. What it
//! does not do is any of the things a facture does: no goods leave the shelf,
//! nobody owes anything, and no money is recorded as having changed hands.
//! Nothing here has happened yet.
//!
//! That is why it is its own service rather than a flag on the sale. A sale
//! writes a document, a stock movement per line and, on credit, a ledger
//! movement, all in one transaction; a proforma writes the document and stops.
//! Passing a kind down through the sale and skipping two of its three writes
//! would leave the skipping to be got right every time the sale grows a fourth.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::money::{compute_totals, Money, TotalsOptions};
use crate::services::documents::{
    BalanceTriple, Document, DocumentKind, NewDocument, NewDocumentLine, SellerBlock,
};
use crate::services::pricing::{self, NewSale};
use crate::services::{clock, customers, documents, settings, shops};

/// Writes the quotation: its number out of the proforma series, its document
/// with the lines and the TVA recap, and nothing else at all.
///
/// A customer is required. A facture is made out to somebody because décret
/// 05-468 art. 3 says so; a proforma is made out to somebody because a
/// quotation nobody was quoted is a piece of paper with no reader. The
/// identifiers that art. 3 asks of a facture are not asked here: a quotation
/// comes before the paperwork, and refusing to quote a customer until their RC
/// is on file would stop the sale that the paperwork is for.
pub fn issue(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewSale,
) -> Result<Document, CoreError> {
    if new.lines.is_empty() {
        return Err(CoreError::validation("lines", "a quotation needs a line"));
    }
    let Some(customer_id) = new.customer_id else {
        return Err(CoreError::validation(
            "customer_id",
            "a proforma is made out to a customer, so one has to be named",
        ));
    };
    if new.global_discount.is_negative() {
        return Err(CoreError::validation(
            "global_discount",
            "a discount cannot be negative",
        ));
    }
    // Nothing was handed over, because nothing has been bought. An amount here
    // is refused rather than dropped: money the caller sent and the document
    // does not hold is money nobody can account for later.
    if new.tendered.is_some() {
        return Err(CoreError::validation(
            "tendered",
            "a proforma quotes an amount, it does not take one",
        ));
    }

    conn.transaction(|conn| {
        let issued_at = new.issued_at.unwrap_or_else(clock::now);
        let regime = settings::regime_as_of(conn, shop_id, issued_at)?;
        let seller = SellerBlock::from(shops::get(conn, shop_id)?);
        // Read through the service, so another shop's fiche is a NotFound
        // rather than a buyer block printed on this shop's paper (rule 3).
        let customer = customers::prove(conn, shop_id, customer_id)?;
        if !customer.fiche().active {
            return Err(CoreError::validation(
                "customer_id",
                "this customer's fiche is closed",
            ));
        }

        let priced = pricing::price_lines(conn, shop_id, regime, &new.lines)?;
        let money_lines = pricing::money_lines(&priced);
        let total_ht = pricing::sum_line_totals(&money_lines)?;
        if new.global_discount > total_ht {
            return Err(CoreError::validation(
                "global_discount",
                "a discount above the basket would make the quotation negative",
            ));
        }
        // The payment mode the customer says they will pay in, because it is
        // what decides the droit de timbre: a quotation for a cash purchase has
        // to quote the stamp, or the facture that follows it comes to more than
        // the customer was told.
        let totals = compute_totals(
            &money_lines,
            &TotalsOptions {
                global_discount: new.global_discount,
                payment_mode: new.payment_mode,
                stamp_enabled: pricing::STAMP_ENABLED,
                regime,
            },
        )
        .map_err(pricing::too_large("lines"))?;

        documents::issue(
            conn,
            shop_id,
            NewDocument {
                kind: DocumentKind::Proforma,
                issued_at,
                user_id,
                regime,
                payment_mode: new.payment_mode,
                seller,
                customer: Some(customer.clone()),
                buyer: Some(pricing::buyer_block(customer.fiche())),
                ref_document_id: None,
                // Three zeros rather than no triple at all. The customer is
                // named, so the paper has a debt block, and what it has to say
                // is that this quotation changes nothing: reading a missing
                // triple as "owes nothing" would make a proforma to a customer
                // who owes 3 000,00 look the same as one to a customer who
                // owes nothing.
                balance: Some(BalanceTriple {
                    old_balance: Money::ZERO,
                    remaining_debt: Money::ZERO,
                    total_debt: Money::ZERO,
                }),
                totals,
                tendered: None,
                change: None,
                lines: priced
                    .iter()
                    .map(|p| NewDocumentLine {
                        product_id: Some(p.product_id),
                        name: p.name.clone(),
                        barcode: p.barcode.clone(),
                        qty_milli: p.qty_milli,
                        unit_price: p.unit_price,
                        line_discount: p.line_discount,
                        rate_bps: p.rate_bps,
                        line_total: p.line_total,
                        ref_line_id: None,
                    })
                    .collect(),
            },
        )
    })
}
