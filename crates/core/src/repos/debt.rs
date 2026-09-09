//! The only place the debt ledger touches diesel. Scoped by `shop_id` like
//! every other query (rule 3), and append-only: there is no update and no
//! delete here, because a mistake is corrected by an `adjustment` row nobody
//! can miss rather than by an edit nobody can see.

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::{
    DebtAllocation, DebtAllocationRow, DebtAllocationRowWrite, DebtEntry, DebtRow, DebtRowWrite,
};
use crate::money::Money;
use crate::schema::{debt_allocations, debt_ledger};

pub fn append(conn: &mut SqliteConnection, write: &DebtRowWrite) -> Result<DebtEntry, CoreError> {
    let row: DebtRow = diesel::insert_into(debt_ledger::table)
        .values(write)
        .returning(DebtRow::as_returning())
        .get_result(conn)?;
    Ok(DebtEntry::from(row))
}

/// What the customer owes: the sum of the ledger, never a stored number. It
/// may be below zero, which is the shop owing the customer after an
/// overpayment.
///
/// Each column is summed on its own and the subtraction happens in Rust,
/// checked: `SUM(debit_centimes - credit_centimes)` would hand back one
/// number nothing here could check, and a statement prints the two halves
/// separately anyway.
///
/// `diesel::dsl::sum` is not used because it types a sum over `BigInt` as
/// `Numeric`, which needs a bignum feature this crate has no other use for.
/// The typed `sql` expressions below name the columns of the table the query
/// is already scoped to.
pub fn balance(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Money, CoreError> {
    let (debit, credit): (Option<i64>, Option<i64>) = debt_ledger::table
        .filter(debt_ledger::shop_id.eq(shop_id))
        .filter(debt_ledger::customer_id.eq(customer_id))
        .select((
            sql::<Nullable<BigInt>>("SUM(debit_centimes)"),
            sql::<Nullable<BigInt>>("SUM(credit_centimes)"),
        ))
        .first(conn)?;
    // No rows is no debt, which is what a customer with an empty ledger owes.
    let debit = Money::centimes(debit.unwrap_or(0));
    let credit = Money::centimes(credit.unwrap_or(0));
    Ok(debit.checked_sub(credit)?)
}

/// One customer's movements, newest first. `created_at` is whole seconds and
/// two movements can land inside one, so the id breaks the tie: the later
/// insert is the later movement.
pub fn ledger(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
) -> Result<Vec<DebtEntry>, CoreError> {
    let rows: Vec<DebtRow> = debt_ledger::table
        .filter(debt_ledger::shop_id.eq(shop_id))
        .filter(debt_ledger::customer_id.eq(customer_id))
        .order((debt_ledger::created_at.desc(), debt_ledger::id.desc()))
        .select(DebtRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(DebtEntry::from).collect())
}

/// Whether the ledger row is one of this shop's. An allocation points at a
/// payment by id and the foreign key alone would take another shop's row.
pub fn entry_belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    entry_id: i32,
) -> Result<bool, CoreError> {
    let found: Option<i32> = debt_ledger::table
        .filter(debt_ledger::shop_id.eq(shop_id))
        .filter(debt_ledger::id.eq(entry_id))
        .select(debt_ledger::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

pub fn allocate(
    conn: &mut SqliteConnection,
    write: &DebtAllocationRowWrite,
) -> Result<DebtAllocation, CoreError> {
    let row: DebtAllocationRow = diesel::insert_into(debt_allocations::table)
        .values(write)
        .returning(DebtAllocationRow::as_returning())
        .get_result(conn)?;
    Ok(DebtAllocation::from(row))
}

/// What has been settled against one document, oldest first: that is the
/// order a payment fills them in (features.md §2).
pub fn allocations(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<Vec<DebtAllocation>, CoreError> {
    let rows: Vec<DebtAllocationRow> = debt_allocations::table
        .filter(debt_allocations::shop_id.eq(shop_id))
        .filter(debt_allocations::document_id.eq(document_id))
        .order(debt_allocations::id.asc())
        .select(DebtAllocationRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(DebtAllocation::from).collect())
}
