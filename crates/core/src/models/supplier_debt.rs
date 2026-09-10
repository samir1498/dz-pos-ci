//! The append-only supplier ledger and the allocations a payment settles
//! purchases through: the mirror of `models::debt` on the supply side.
//!
//! One row raises what the shop owes a supplier or lowers it, never both. A
//! debit is goods received or a balance carried in from before the software;
//! a credit is money paid or goods sent back. The direction is a fact about
//! the row rather than the sign of a number somebody has to remember to read,
//! and the migration's CHECK says the same thing to the file.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::money::Money;
use crate::schema::{supplier_allocations, supplier_ledger};

pub use super::sql_types::{PaymentMethod, SupplierDebtKind};

/// One movement of what the shop owes a supplier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplierEntry {
    pub id: i32,
    pub shop_id: i32,
    pub supplier_id: i32,
    /// The purchase the movement belongs to. `None` on an opening balance, a
    /// payment and an adjustment, none of which belong to a single purchase.
    pub purchase_id: Option<i32>,
    pub kind: SupplierDebtKind,
    /// What the movement added to what the shop owes. Zero on a payment or a
    /// return.
    pub debit: Money,
    /// What it took off. Zero on a purchase or an opening balance.
    pub credit: Money,
    pub user_id: i32,
    pub note: Option<String>,
    /// How the payment was made. `None` on every movement that is not a
    /// payment: nothing was handed over on goods received or a correction.
    pub payment_mode: Option<PaymentMethod>,
    pub created_at: NaiveDateTime,
}

impl SupplierEntry {
    /// What this row moves the balance by: positive when the shop owes more,
    /// negative when it owes less.
    pub fn signed(&self) -> Result<Money, crate::money::MoneyError> {
        self.debit.checked_sub(self.credit)
    }
}

/// A movement as a caller hands it over, before it has an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSupplierEntry {
    pub supplier_id: i32,
    pub purchase_id: Option<i32>,
    pub kind: SupplierDebtKind,
    pub debit: Money,
    pub credit: Money,
    pub user_id: i32,
    pub note: Option<String>,
}

/// What one payment settled on one purchase. A payment settles several
/// purchases oldest first, so the payment is one ledger row and the purchases
/// it covered are these; kept apart from the ledger so the balance stays one
/// sum over one table, and so what is still owed on a single purchase has an
/// answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplierAllocation {
    pub id: i32,
    pub shop_id: i32,
    pub payment_ledger_id: i32,
    pub purchase_id: i32,
    pub amount: Money,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSupplierAllocation {
    pub payment_ledger_id: i32,
    pub purchase_id: i32,
    pub amount: Money,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = supplier_ledger)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct SupplierDebtRow {
    pub id: i32,
    pub shop_id: i32,
    pub supplier_id: i32,
    pub purchase_id: Option<i32>,
    pub kind: SupplierDebtKind,
    pub debit_centimes: i64,
    pub credit_centimes: i64,
    pub user_id: i32,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
    pub payment_mode: Option<PaymentMethod>,
}

/// `None` on the two columns that carry a default leaves the default in
/// place: a movement written without a payment mode says nothing about one,
/// and one written without a moment is stamped by the file's own clock. The
/// service that takes a payment fills both in from the shop clock.
#[derive(Debug, Insertable)]
#[diesel(table_name = supplier_ledger)]
#[diesel(treat_none_as_default_value = true)]
pub(crate) struct SupplierDebtRowWrite {
    pub shop_id: i32,
    pub supplier_id: i32,
    pub purchase_id: Option<i32>,
    pub kind: SupplierDebtKind,
    pub debit_centimes: i64,
    pub credit_centimes: i64,
    pub user_id: i32,
    pub note: Option<String>,
    pub payment_mode: Option<PaymentMethod>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = supplier_allocations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct SupplierAllocationRow {
    pub id: i32,
    pub shop_id: i32,
    pub payment_ledger_id: i32,
    pub purchase_id: i32,
    pub amount_centimes: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = supplier_allocations)]
pub(crate) struct SupplierAllocationRowWrite {
    pub shop_id: i32,
    pub payment_ledger_id: i32,
    pub purchase_id: i32,
    pub amount_centimes: i64,
}

impl From<SupplierDebtRow> for SupplierEntry {
    fn from(r: SupplierDebtRow) -> Self {
        SupplierEntry {
            id: r.id,
            shop_id: r.shop_id,
            supplier_id: r.supplier_id,
            purchase_id: r.purchase_id,
            kind: r.kind,
            debit: Money::centimes(r.debit_centimes),
            credit: Money::centimes(r.credit_centimes),
            user_id: r.user_id,
            note: r.note,
            payment_mode: r.payment_mode,
            created_at: r.created_at,
        }
    }
}

impl From<SupplierAllocationRow> for SupplierAllocation {
    fn from(r: SupplierAllocationRow) -> Self {
        SupplierAllocation {
            id: r.id,
            shop_id: r.shop_id,
            payment_ledger_id: r.payment_ledger_id,
            purchase_id: r.purchase_id,
            amount: Money::centimes(r.amount_centimes),
            created_at: r.created_at,
        }
    }
}
