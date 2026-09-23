//! The only place the supplier ledger touches diesel. Scoped by `shop_id`
//! like every other query (rule 3), and append-only: there is no update and
//! no delete here, because a mistake is corrected by an `adjustment` row
//! nobody can miss rather than by an edit nobody can see.

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable};
use diesel::sqlite::SqliteConnection;

use crate::error::{CoreError, RetailError};
use crate::models::supplier_debt::{
    SupplierAllocation, SupplierAllocationRow, SupplierAllocationRowWrite, SupplierDebtRow,
    SupplierDebtRowWrite, SupplierEntry,
};
use crate::money::Money;
use crate::schema::{supplier_allocations, supplier_ledger};

/// Writes one movement. The row has to carry its own moment: the column's
/// default is SQLite's CURRENT_TIMESTAMP, which is UTC, and every period this
/// app answers for is a stretch of days on the shop's calendar (UTC+1). A
/// payment taken at 00:30 in Algiers would then be stored on the day before
/// and fall out of the day the shop counted its drawer, and out of the cash
/// position with it (`services::cash`). The caller stamps it from
/// `services::clock`, the way `debt::append_at` does on the customer side.
pub fn append(
    conn: &mut SqliteConnection,
    write: &SupplierDebtRowWrite,
) -> Result<SupplierEntry, RetailError> {
    if write.created_at.is_none() {
        return Err(RetailError::Unstamped {
            entity: "supplier_ledger",
        });
    }
    let row: SupplierDebtRow = diesel::insert_into(supplier_ledger::table)
        .values(write)
        .returning(SupplierDebtRow::as_returning())
        .get_result(conn)?;
    Ok(SupplierEntry::from(row))
}

/// What the shop owes the supplier: the sum of the ledger, never a stored
/// number. It may be below zero, which is the supplier owing the shop after
/// an advance or a return past what was due.
///
/// Each column is summed on its own and the subtraction happens in Rust,
/// checked, for the reason `repos::debt::balance` gives: one number out of
/// SQL is a number nothing here could check, and a statement prints the two
/// halves separately anyway.
pub fn balance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<Money, CoreError> {
    let (debit, credit): (Option<i64>, Option<i64>) = supplier_ledger::table
        .filter(supplier_ledger::shop_id.eq(shop_id))
        .filter(supplier_ledger::supplier_id.eq(supplier_id))
        .select((
            sql::<Nullable<BigInt>>("SUM(debit_centimes)"),
            sql::<Nullable<BigInt>>("SUM(credit_centimes)"),
        ))
        .first(conn)?;
    // No rows is no debt, which is what a supplier with an empty ledger is
    // owed.
    let debit = Money::centimes(debit.unwrap_or(0));
    let credit = Money::centimes(credit.unwrap_or(0));
    Ok(debit.checked_sub(credit)?)
}

/// Each supplier's two column sums, in one query, for the shop's whole list.
/// A supplier with no movement is not in the answer: no rows is no debt, and
/// the caller reads a missing one as nothing owed.
pub fn balances(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<Vec<(i32, i64, i64)>, CoreError> {
    let rows: Vec<(i32, Option<i64>, Option<i64>)> = supplier_ledger::table
        .filter(supplier_ledger::shop_id.eq(shop_id))
        .group_by(supplier_ledger::supplier_id)
        .select((
            supplier_ledger::supplier_id,
            sql::<Nullable<BigInt>>("SUM(debit_centimes)"),
            sql::<Nullable<BigInt>>("SUM(credit_centimes)"),
        ))
        .load(conn)?;
    Ok(rows
        .into_iter()
        .map(|(supplier_id, debit, credit)| (supplier_id, debit.unwrap_or(0), credit.unwrap_or(0)))
        .collect())
}

/// One supplier's movements, newest first. `created_at` is whole seconds and
/// two movements can land inside one, so the id breaks the tie: the later
/// insert is the later movement.
pub fn ledger(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<Vec<SupplierEntry>, CoreError> {
    let rows: Vec<SupplierDebtRow> = supplier_ledger::table
        .filter(supplier_ledger::shop_id.eq(shop_id))
        .filter(supplier_ledger::supplier_id.eq(supplier_id))
        .order((
            supplier_ledger::created_at.desc(),
            supplier_ledger::id.desc(),
        ))
        .select(SupplierDebtRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(SupplierEntry::from).collect())
}

/// Each order's two column sums, for one supplier, in one query. Only the
/// rows that cite an order are in it: an opening balance, a payment and a
/// correction belong to no single purchase, and what is still owed on a
/// piece of paper is a question about the paper.
///
/// The subtraction happens in Rust, checked, for the reason `balance` gives.
pub fn sums_by_purchase(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<Vec<(i32, i64, i64)>, CoreError> {
    let rows: Vec<(Option<i32>, Option<i64>, Option<i64>)> = supplier_ledger::table
        .filter(supplier_ledger::shop_id.eq(shop_id))
        .filter(supplier_ledger::supplier_id.eq(supplier_id))
        .filter(supplier_ledger::purchase_id.is_not_null())
        .group_by(supplier_ledger::purchase_id)
        .select((
            supplier_ledger::purchase_id,
            sql::<Nullable<BigInt>>("SUM(debit_centimes)"),
            sql::<Nullable<BigInt>>("SUM(credit_centimes)"),
        ))
        .load(conn)?;
    // The filter above is what makes the id present, so a null here is a row
    // SQLite cannot answer with and the map is never taken.
    Ok(rows
        .into_iter()
        .filter_map(|(purchase_id, debit, credit)| {
            purchase_id.map(|id| (id, debit.unwrap_or(0), credit.unwrap_or(0)))
        })
        .collect())
}

/// What has been placed on each of the named orders so far, in one query. An
/// order nothing has settled is not in the answer, and the caller reads a
/// missing one as nothing placed.
pub fn allocated_by_purchase(
    conn: &mut SqliteConnection,
    shop_id: i32,
    purchase_ids: &[i32],
) -> Result<Vec<(i32, i64)>, CoreError> {
    if purchase_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(i32, Option<i64>)> = supplier_allocations::table
        .filter(supplier_allocations::shop_id.eq(shop_id))
        .filter(supplier_allocations::purchase_id.eq_any(purchase_ids.to_vec()))
        .group_by(supplier_allocations::purchase_id)
        .select((
            supplier_allocations::purchase_id,
            sql::<Nullable<BigInt>>("SUM(amount_centimes)"),
        ))
        .load(conn)?;
    Ok(rows
        .into_iter()
        .map(|(purchase_id, placed)| (purchase_id, placed.unwrap_or(0)))
        .collect())
}

/// Whether the ledger row is one of this shop's. An allocation points at a
/// payment by id and the foreign key alone would take another shop's row.
pub fn entry_belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    entry_id: i32,
) -> Result<bool, CoreError> {
    let found: Option<i32> = supplier_ledger::table
        .filter(supplier_ledger::shop_id.eq(shop_id))
        .filter(supplier_ledger::id.eq(entry_id))
        .select(supplier_ledger::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

pub fn allocate(
    conn: &mut SqliteConnection,
    write: &SupplierAllocationRowWrite,
) -> Result<SupplierAllocation, CoreError> {
    let row: SupplierAllocationRow = diesel::insert_into(supplier_allocations::table)
        .values(write)
        .returning(SupplierAllocationRow::as_returning())
        .get_result(conn)?;
    Ok(SupplierAllocation::from(row))
}

/// What one payment settled, oldest purchase first: the order the money
/// filled them in.
pub fn allocations_of_payment(
    conn: &mut SqliteConnection,
    shop_id: i32,
    payment_ledger_id: i32,
) -> Result<Vec<SupplierAllocation>, CoreError> {
    let rows: Vec<SupplierAllocationRow> = supplier_allocations::table
        .filter(supplier_allocations::shop_id.eq(shop_id))
        .filter(supplier_allocations::payment_ledger_id.eq(payment_ledger_id))
        .order(supplier_allocations::id.asc())
        .select(SupplierAllocationRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(SupplierAllocation::from).collect())
}

/// What has been settled against one purchase, oldest first: that is the
/// order a payment fills them in.
pub fn allocations(
    conn: &mut SqliteConnection,
    shop_id: i32,
    purchase_id: i32,
) -> Result<Vec<SupplierAllocation>, CoreError> {
    let rows: Vec<SupplierAllocationRow> = supplier_allocations::table
        .filter(supplier_allocations::shop_id.eq(shop_id))
        .filter(supplier_allocations::purchase_id.eq(purchase_id))
        .order(supplier_allocations::id.asc())
        .select(SupplierAllocationRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(SupplierAllocation::from).collect())
}

#[cfg(test)]
#[path = "../../tests/unit/repos_supplier_debt.rs"]
mod tests;
