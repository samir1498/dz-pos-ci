//! The append-only debt ledger and the allocations a payment settles
//! documents through (features.md §2).
//!
//! One row raises what a customer owes or lowers it, never both: the
//! direction is a fact about the row rather than the sign of a number
//! somebody has to remember to read, and the migration's CHECK says the same
//! thing to the file.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::money::Money;
use crate::schema::{debt_allocations, debt_ledger};

pub use super::sql_types::DebtKind;

/// One movement of a customer's debt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtEntry {
    pub id: i32,
    pub shop_id: i32,
    pub customer_id: i32,
    pub document_id: Option<i32>,
    pub kind: DebtKind,
    /// What the movement added to the debt. Zero on a payment or an avoir.
    pub debit: Money,
    /// What it took off. Zero on a sale or an opening balance.
    pub credit: Money,
    pub user_id: i32,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
}

impl DebtEntry {
    /// What this row moves the balance by: positive when the customer owes
    /// more, negative when they owe less.
    pub fn signed(&self) -> Result<Money, crate::money::MoneyError> {
        self.debit.checked_sub(self.credit)
    }
}

/// A movement as a caller hands it over, before it has an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDebtEntry {
    pub customer_id: i32,
    pub document_id: Option<i32>,
    pub kind: DebtKind,
    pub debit: Money,
    pub credit: Money,
    pub user_id: i32,
    pub note: Option<String>,
}

/// What one payment settled on one document. features.md §2: a payment
/// settles several documents oldest first, so a payment is one ledger row and
/// the documents it covered are these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtAllocation {
    pub id: i32,
    pub shop_id: i32,
    pub payment_ledger_id: i32,
    pub document_id: i32,
    pub amount: Money,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDebtAllocation {
    pub payment_ledger_id: i32,
    pub document_id: i32,
    pub amount: Money,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = debt_ledger)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct DebtRow {
    pub id: i32,
    pub shop_id: i32,
    pub customer_id: i32,
    pub document_id: Option<i32>,
    pub kind: DebtKind,
    pub debit_centimes: i64,
    pub credit_centimes: i64,
    pub user_id: i32,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = debt_ledger)]
pub(crate) struct DebtRowWrite {
    pub shop_id: i32,
    pub customer_id: i32,
    pub document_id: Option<i32>,
    pub kind: DebtKind,
    pub debit_centimes: i64,
    pub credit_centimes: i64,
    pub user_id: i32,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = debt_allocations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct DebtAllocationRow {
    pub id: i32,
    pub shop_id: i32,
    pub payment_ledger_id: i32,
    pub document_id: i32,
    pub amount_centimes: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = debt_allocations)]
pub(crate) struct DebtAllocationRowWrite {
    pub shop_id: i32,
    pub payment_ledger_id: i32,
    pub document_id: i32,
    pub amount_centimes: i64,
}

impl From<DebtRow> for DebtEntry {
    fn from(r: DebtRow) -> Self {
        DebtEntry {
            id: r.id,
            shop_id: r.shop_id,
            customer_id: r.customer_id,
            document_id: r.document_id,
            kind: r.kind,
            debit: Money::centimes(r.debit_centimes),
            credit: Money::centimes(r.credit_centimes),
            user_id: r.user_id,
            note: r.note,
            created_at: r.created_at,
        }
    }
}

impl From<DebtAllocationRow> for DebtAllocation {
    fn from(r: DebtAllocationRow) -> Self {
        DebtAllocation {
            id: r.id,
            shop_id: r.shop_id,
            payment_ledger_id: r.payment_ledger_id,
            document_id: r.document_id,
            amount: Money::centimes(r.amount_centimes),
            created_at: r.created_at,
        }
    }
}
