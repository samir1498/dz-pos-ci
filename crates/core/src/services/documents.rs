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
use crate::models::debt::{DebtKind, NewDebtEntry};
use crate::models::document::{
    payment_mode_stored, regime_stored, CancelWrite, DocumentLineRowWrite, DocumentRowWrite,
    DocumentTvaRowWrite,
};
use crate::models::stock::{Movement, MovementKind};
use crate::money::{Money, PaymentMode};
use crate::repos::counters;
use crate::repos::documents as repo;
use crate::services::{audit, avoir, clock, customers, debt, optional_field, stock};

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

/// Annuls a document (features.md §3). It keeps its number and its row, so the
/// series never gaps (décret 05-468 art. 10); what it stops doing is asking for
/// its amount and holding the goods off the shelf.
///
/// How the money goes back depends on what the document did with it. A ticket
/// or a cash facture took the money over the counter and owed nobody anything,
/// so the goods come back on a return movement naming the document itself and
/// nothing else is written. A facture that put money on a customer's account,
/// whether it is still owed or has since been paid, is undone the way a
/// facture is always undone: by a whole avoir, in this same transaction, which
/// carries the goods and the money back together and leaves a numbered paper
/// saying so. A facture whose goods have all come back on earlier avoirs is
/// annulled with no second credit note at all: there is nothing left to carry.
///
/// An avoir and a proforma are refused. An avoir is the instrument that undoes
/// a facture, and undoing it in turn would be a second reversal nobody can read
/// against the first; a proforma is a quotation that moved nothing, so there is
/// nothing to put back and no reason to spend a cancellation on it.
///
/// A document is cancelled once. The block a second cancellation would write
/// over is who took the first one and why, which is the whole of what the log
/// is keeping.
///
/// Cancelling a ticket that has already been printed stays allowed: until M4
/// brings roles there is nobody to refuse it, and the audit entry carries the
/// name of whoever took the decision.
pub fn cancel(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    document_id: i32,
    reason: String,
    at: Option<NaiveDateTime>,
) -> Result<Document, CoreError> {
    let Some(reason) = optional_field("reason", Some(&reason))? else {
        return Err(CoreError::validation(
            "reason",
            "a document is annulled for a stated reason",
        ));
    };
    conn.transaction(|conn| {
        let document = repo::get(conn, shop_id, document_id)?;
        if document.status == DocumentStatus::Cancelled {
            return Err(CoreError::validation(
                "document_id",
                "this document is already annulée",
            ));
        }
        // Named rather than excluded. A ticket and a facture are the two
        // papers a sale is handed over on, and undoing one is what this knows
        // how to do: the goods come back and the money comes off the account.
        // Everything else is refused by not being on the list, so a kind added
        // later is refused until somebody decides what undoing it means rather
        // than falling through to a cancellation that half works.
        match document.kind {
            DocumentKind::Ticket | DocumentKind::Facture => {}
            _ => {
                return Err(CoreError::validation(
                    "document_id",
                    "only a ticket and a facture are annulled",
                ))
            }
        }
        let at = at.unwrap_or_else(clock::now);

        // A facture that put money on an account is undone by an avoir, which
        // brings the goods and the money back in one numbered document. A
        // ticket that put money on an account has no avoir to be undone by,
        // because an avoir is written against a facture, so the goods and the
        // ledger move separately. Everything else owed nobody anything and
        // only the goods move.
        let avoir_document_id = match effect_of(conn, shop_id, &document)? {
            CancelEffect::NothingToReverse => None,
            CancelEffect::StockBack => {
                return_the_goods(conn, shop_id, user_id, &document)?;
                None
            }
            CancelEffect::StockBackAndAvoir { .. } if document.kind == DocumentKind::Facture => {
                Some(
                    avoir::issue(
                        conn,
                        shop_id,
                        user_id,
                        document_id,
                        None,
                        Some(reason.clone()),
                        Some(at),
                    )?
                    .id,
                )
            }
            CancelEffect::StockBackAndAvoir { amount } => {
                return_the_goods(conn, shop_id, user_id, &document)?;
                reverse_on_the_ledger(conn, shop_id, user_id, &document, amount, at, &reason)?;
                None
            }
        };

        repo::set_cancelled(
            conn,
            shop_id,
            document_id,
            &CancelWrite {
                cancelled_at: at,
                cancelled_by: user_id,
                cancel_reason: reason.clone(),
                cancel_avoir_document_id: avoir_document_id,
            },
        )?;

        // Read after the write and before the log, because it is what the
        // cancellation left behind.
        let left_asking = repo::get(conn, shop_id, document_id)?
            .balance
            .map(|b| b.remaining_debt.as_centimes());
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_CANCEL,
                entity: "document",
                entity_id: Some(document_id),
                before: Some(
                    serde_json::json!({
                        "status": DocumentStatus::Issued.as_str(),
                        "remaining_debt_centimes":
                            document.balance.map(|b| b.remaining_debt.as_centimes()),
                    })
                    .to_string(),
                ),
                after: Some(
                    serde_json::json!({
                        "status": DocumentStatus::Cancelled.as_str(),
                        "reason": reason,
                        // Named rather than only counted: a reader asking why
                        // a facture stopped asking for its amount is handed
                        // the numbered paper that carried it back.
                        "avoir_document_id": avoir_document_id,
                        // What the paper is left asking for, beside what it
                        // asked for before: a cancellation that moved money
                        // and one that moved none read the same otherwise,
                        // and the difference is the whole of what happened.
                        "remaining_debt_centimes": left_asking,
                    })
                    .to_string(),
                ),
            },
        )?;

        repo::get(conn, shop_id, document_id)
    })
}

/// What cancelling a document would do, so a screen can say it before it asks.
///
/// The screen cannot work this out from the document alone. A facture whose
/// goods have all come back on earlier credit notes carries debt, was sold on
/// credit and names a customer, and cancelling it does nothing at all: the
/// avoirs already did it. Reading the three fields would say otherwise, and a
/// confirm that promises a credit note the shop then cannot find is worse than
/// no confirm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelEffect {
    /// The document is annulled and nothing moves: every one of its lines has
    /// already come back on an avoir.
    NothingToReverse,
    /// The goods go back on the shelf. Nobody was owed anything.
    StockBack,
    /// The goods go back and this much comes off the customer's account. On a
    /// facture it is a numbered avoir; on a ticket it is a ledger row alone,
    /// because an avoir is written against a facture.
    StockBackAndAvoir { amount: Money },
}

pub fn cancel_effect(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<CancelEffect, CoreError> {
    let document = repo::get(conn, shop_id, document_id)?;
    effect_of(conn, shop_id, &document)
}

fn effect_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document: &Document,
) -> Result<CancelEffect, CoreError> {
    if !carries_money(document) {
        return Ok(CancelEffect::StockBack);
    }
    if document.kind != DocumentKind::Facture {
        // A ticket carries no avoirs, so the whole of it is still out.
        return Ok(CancelEffect::StockBackAndAvoir {
            amount: document.totals.net_to_pay,
        });
    }
    if !avoir::anything_left(conn, shop_id, document)? {
        return Ok(CancelEffect::NothingToReverse);
    }
    Ok(CancelEffect::StockBackAndAvoir {
        amount: avoir::what_is_left(conn, shop_id, document)?,
    })
}

/// Whether the document put money on a customer's account, and so has to be
/// undone on the ledger rather than by the goods alone.
///
/// A credit sale is the plain case. A facture the customer has since paid is
/// the other one: the ledger carries both the sale and the payment, and
/// cancelling it has to leave the payment standing as credit the shop is
/// holding rather than quietly cancel the money too.
fn carries_money(document: &Document) -> bool {
    document.customer_id.is_some() && document.payment_mode == PaymentMode::Credit
}

/// Takes a cancelled credit ticket off the customer's account.
///
/// The ledger half of an avoir with no document above it. A ticket carries no
/// number in the avoir series and nothing is written against it, so what says
/// the money came back is one credit row naming the ticket. What that row
/// settles is the ticket's own unpaid part first, then the customer's other
/// unpaid papers oldest first, and what none of them can take is credit the
/// shop is holding: the same three steps, and for the same reason, as the
/// money half of `avoir::issue` (features.md §3).
fn reverse_on_the_ledger(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    document: &Document,
    amount: Money,
    at: NaiveDateTime,
    reason: &str,
) -> Result<(), CoreError> {
    let Some(customer_id) = document.customer_id else {
        return Ok(());
    };
    if amount == Money::ZERO {
        return Ok(());
    }
    let entry = debt::append_at(
        conn,
        shop_id,
        NewDebtEntry {
            customer_id,
            document_id: Some(document.id),
            kind: DebtKind::Avoir,
            debit: Money::ZERO,
            credit: amount,
            user_id,
            note: Some(reason.to_string()),
        },
        Some(at),
    )?;
    let unpaid = document.balance.map_or(Money::ZERO, |b| b.remaining_debt);
    let on_itself = amount.min(unpaid);
    debt::settle_document(conn, shop_id, entry.id, document.id, on_itself)?;
    debt::settle_oldest_first(
        conn,
        shop_id,
        customer_id,
        entry.id,
        amount.checked_sub(on_itself)?,
    )?;
    Ok(())
}

/// Puts the goods of a cancelled document back on the shelf, one movement per
/// line, each naming the document that is no longer holding them.
fn return_the_goods(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    document: &Document,
) -> Result<(), CoreError> {
    // What the goods cost when they left on this document. They go back at
    // that cost and never at the fiche's cost today, for the same reason the
    // avoir does it: a delivery between the sale and the cancellation moves
    // the fiche, and a reversal that followed it would move the month's
    // margin with every purchase.
    let sold_at = stock::sale_costs(conn, shop_id, document.id)?;
    for line in &document.lines {
        let Some(product_id) = line.product_id else {
            continue;
        };
        // The fiche's cost is not a fallback, for the reason `avoir::issue`
        // says: a sold line without a movement is a file that disagrees with
        // itself, and today's cost would move a margin already earned.
        let Some(unit_cost) = sold_at.get(&product_id).copied() else {
            return Err(CoreError::UnpricedReversal {
                document_id: document.id,
                product_id,
                reason: "the line it puts back has no sale movement",
            });
        };
        stock::record(
            conn,
            shop_id,
            &Movement {
                product_id,
                kind: MovementKind::Return,
                qty_milli: line.qty_milli,
                unit_cost,
                document_id: Some(document.id),
                user_id,
            },
        )?;
    }
    Ok(())
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
