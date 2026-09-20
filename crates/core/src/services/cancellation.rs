//! Undoing a document: the goods go back, the ledger is put right, and a
//! facture gets the avoir that says so (features.md §3).
//!
//! Above `documents` and `avoir` rather than inside either. Cancelling a
//! facture writes an avoir, so the code that does it needs both, and while
//! it sat in `documents` the two modules imported each other: a ring the
//! compiler accepts and nobody can read one half of.
//!
//! It moved here on 2026-09-20 with no rule and no arithmetic edited. One
//! line is not the old one: the write that stamps the cancellation onto the
//! document goes through `documents::mark_cancelled` rather than straight
//! into that module's repo, because reaching past a sibling into its repo is
//! what `crates/core/tests/services_go_through_services.rs` refuses.

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::{DebtKind, NewDebtEntry};
use crate::models::document::{Document, DocumentKind, DocumentStatus};
use crate::models::stock::{Movement, MovementKind};
use crate::money::{Money, PaymentMode};
use crate::services::{audit, avoir, clock, debt, documents, optional_field, stock};

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
        let document = documents::get(conn, shop_id, document_id)?;
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

        documents::mark_cancelled(
            conn,
            shop_id,
            document_id,
            at,
            user_id,
            reason.clone(),
            avoir_document_id,
        )?;

        // Read after the write and before the log, because it is what the
        // cancellation left behind.
        let left_asking = documents::get(conn, shop_id, document_id)?
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

        documents::get(conn, shop_id, document_id)
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
    let document = documents::get(conn, shop_id, document_id)?;
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
