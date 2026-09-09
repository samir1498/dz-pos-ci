//! What a customer owes (features.md §2). The ledger is append-only and the
//! balance is its sum, so there is nothing here that edits or deletes a
//! movement: a mistake is corrected by an `adjustment` row a comptable can
//! read, and a customer's fiche being edited never touches the ledger. T2
//! adds the endpoint that writes one.
//!
//! `append` runs in whatever transaction the caller opened, and does not open
//! one of its own: a sale on credit writes the document, the lines, the stock
//! and the debt together or writes none of them. The one caller in this
//! milestone that has no transaction of its own is `customers::create`, and
//! it opens one.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::{DebtAllocationRowWrite, DebtRowWrite};
use crate::money::Money;
use crate::repos::customers as customers_repo;
use crate::repos::debt as repo;
use crate::repos::documents as documents_repo;
use crate::services::optional_field;

pub use crate::models::debt::{
    DebtAllocation, DebtEntry, DebtKind, NewDebtAllocation, NewDebtEntry,
};

/// What the customer owes right now: the sum of the ledger, never a stored
/// number. Below zero is the shop owing the customer after an overpayment.
pub fn balance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Money, CoreError> {
    ensure_customer(conn, shop_id, customer_id)?;
    repo::balance(conn, shop_id, customer_id)
}

/// The customer's movements, newest first.
pub fn ledger(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Vec<DebtEntry>, CoreError> {
    ensure_customer(conn, shop_id, customer_id)?;
    repo::ledger(conn, shop_id, customer_id)
}

/// Writes one movement. Runs inside the caller's transaction.
///
/// A movement raises the debt or lowers it, never both and never neither: the
/// migration's CHECK refuses the first, and this refuses the second, because
/// a row that moves nothing is a mistake somebody makes with an empty form
/// and it would sit in a statement forever.
pub fn append(
    conn: &mut SqliteConnection,
    shop_id: i32,
    entry: NewDebtEntry,
) -> Result<DebtEntry, CoreError> {
    ensure_customer(conn, shop_id, entry.customer_id)?;
    // The document a movement cites is checked the same way the allocation's
    // is: a statement that names a document this shop never issued is worse
    // than one that names none.
    if let Some(document_id) = entry.document_id {
        ensure_document(conn, shop_id, document_id)?;
    }
    if entry.debit.is_negative() {
        return Err(CoreError::validation(
            "debit",
            "a movement is written in the direction its column names, never as a negative",
        ));
    }
    if entry.credit.is_negative() {
        return Err(CoreError::validation(
            "credit",
            "a movement is written in the direction its column names, never as a negative",
        ));
    }
    if entry.debit != Money::ZERO && entry.credit != Money::ZERO {
        return Err(CoreError::validation(
            "debit",
            "a movement raises the debt or lowers it, not both",
        ));
    }
    if entry.debit == Money::ZERO && entry.credit == Money::ZERO {
        return Err(CoreError::validation(
            "debit",
            "a movement of nothing moves no debt",
        ));
    }
    let note = optional_field("note", entry.note.as_deref())?;
    repo::append(
        conn,
        &DebtRowWrite {
            shop_id,
            customer_id: entry.customer_id,
            document_id: entry.document_id,
            kind: entry.kind,
            debit_centimes: entry.debit.as_centimes(),
            credit_centimes: entry.credit.as_centimes(),
            user_id: entry.user_id,
            note,
        },
    )
}

/// Records that part of a payment settled one document. Runs inside the
/// caller's transaction, next to the `payment` row it belongs to.
pub fn allocate(
    conn: &mut SqliteConnection,
    shop_id: i32,
    allocation: NewDebtAllocation,
) -> Result<DebtAllocation, CoreError> {
    if allocation.amount.as_centimes() <= 0 {
        return Err(CoreError::validation(
            "amount",
            "an allocation of nothing settles nothing",
        ));
    }
    ensure_entry(conn, shop_id, allocation.payment_ledger_id)?;
    ensure_document(conn, shop_id, allocation.document_id)?;
    repo::allocate(
        conn,
        &DebtAllocationRowWrite {
            shop_id,
            payment_ledger_id: allocation.payment_ledger_id,
            document_id: allocation.document_id,
            amount_centimes: allocation.amount.as_centimes(),
        },
    )
}

/// What a payment has settled against one document, oldest first.
pub fn allocations(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<Vec<DebtAllocation>, CoreError> {
    ensure_document(conn, shop_id, document_id)?;
    repo::allocations(conn, shop_id, document_id)
}

/// The foreign key would accept another shop's customer, so the shop is
/// checked here the way a document's product is (rule 3).
/// Both foreign keys an allocation carries would take another shop's row, so
/// each is checked against the shop asking (rule 3).
fn ensure_entry(conn: &mut SqliteConnection, shop_id: i32, entry_id: i32) -> Result<(), CoreError> {
    if repo::entry_belongs_to_shop(conn, shop_id, entry_id)? {
        return Ok(());
    }
    Err(CoreError::NotFound {
        entity: "debt_entry",
        id: entry_id,
    })
}

fn ensure_document(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<(), CoreError> {
    if documents_repo::belongs_to_shop(conn, shop_id, document_id)? {
        return Ok(());
    }
    Err(CoreError::NotFound {
        entity: "document",
        id: document_id,
    })
}

fn ensure_customer(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<(), CoreError> {
    if customers_repo::belongs_to_shop(conn, shop_id, customer_id)? {
        return Ok(());
    }
    Err(CoreError::NotFound {
        entity: "customer",
        id: customer_id,
    })
}
