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

/// One movement onto the customer's ledger. A row that arrives without a
/// moment on it is refused rather than stamped by the file, the same rule
/// `repos::supplier_debt::append` holds on the other side: the column's
/// default is SQLite's CURRENT_TIMESTAMP, which is UTC, and every period this
/// app answers for is a stretch of days on the shop's calendar (UTC+1). A
/// payment taken at 00:30 in Algiers would then be stored on the day before
/// and fall out of the day the shop counted its drawer, and out of the cash
/// position with it (`repos::cash::customer_payments` filters this very
/// column). The caller stamps it from `services::clock`.
pub fn append(conn: &mut SqliteConnection, write: &DebtRowWrite) -> Result<DebtEntry, CoreError> {
    if write.created_at.is_none() {
        return Err(CoreError::Unstamped {
            entity: "debt_ledger",
        });
    }
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

/// Each customer's two column sums, in one query, for the shop's whole list.
/// A customer with no movement is not in the answer: no rows is no debt, and
/// the caller reads a missing one as nothing owed.
///
/// The subtraction stays in Rust, checked, for the reason `balance` gives;
/// this is the same query grouped, so the list screen reads one row per
/// customer instead of one query per customer.
pub fn balances(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<Vec<(i32, i64, i64)>, CoreError> {
    let rows: Vec<(i32, Option<i64>, Option<i64>)> = debt_ledger::table
        .filter(debt_ledger::shop_id.eq(shop_id))
        .group_by(debt_ledger::customer_id)
        .select((
            debt_ledger::customer_id,
            sql::<Nullable<BigInt>>("SUM(debit_centimes)"),
            sql::<Nullable<BigInt>>("SUM(credit_centimes)"),
        ))
        .load(conn)?;
    Ok(rows
        .into_iter()
        .map(|(customer_id, debit, credit)| (customer_id, debit.unwrap_or(0), credit.unwrap_or(0)))
        .collect())
}

/// One customer's movements, newest first. Two movements can carry the same
/// moment, because a caller hands one in rather than reading it here and two
/// calls can be handed the same, so the id breaks the tie: the later insert
/// is the later movement.
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

/// What one payment settled, oldest document first: the order the money
/// filled them in (features.md §2).
pub fn allocations_of_payment(
    conn: &mut SqliteConnection,
    shop_id: i32,
    payment_ledger_id: i32,
) -> Result<Vec<DebtAllocation>, CoreError> {
    let rows: Vec<DebtAllocationRow> = debt_allocations::table
        .filter(debt_allocations::shop_id.eq(shop_id))
        .filter(debt_allocations::payment_ledger_id.eq(payment_ledger_id))
        .order(debt_allocations::id.asc())
        .select(DebtAllocationRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(DebtAllocation::from).collect())
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

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::customer::{CustomerRowWrite, PartyKind};
    use crate::models::debt::{DebtKind, PaymentMethod};
    use crate::repos::customers;
    use crate::repos::testdb::{open, OWNER, SHOP};

    fn a_customer(conn: &mut SqliteConnection, name: &str) -> i32 {
        customers::insert(
            conn,
            &CustomerRowWrite {
                shop_id: SHOP,
                name: name.to_string(),
                party_kind: PartyKind::Consumer,
                phone: None,
                address: None,
                rc: None,
                nif: None,
                nis: None,
                ai: None,
                credit_limit_centimes: None,
                warn_threshold_centimes: None,
                notes: None,
                active: true,
                updated_at: dzpos_kernel::services::clock::now(),
            },
        )
        .unwrap()
        .id
    }

    fn movement(customer_id: i32, kind: DebtKind, debit: i64, credit: i64) -> DebtRowWrite {
        DebtRowWrite {
            shop_id: SHOP,
            customer_id,
            document_id: None,
            kind,
            debit_centimes: debit,
            credit_centimes: credit,
            user_id: OWNER,
            note: None,
            payment_mode: None,
            // Stamped from the shop's clock, which is what `append` asks of
            // every caller.
            created_at: Some(dzpos_kernel::services::clock::now()),
        }
    }

    /// Every query here takes a `shop_id`, and the customer ledger is the
    /// column the dashboard reads its debt figure from, so a shop seeing the
    /// shop next door's debtors is the worst answer this file can give. The
    /// supplier ledger beside it is proved the same way
    /// (`repos::supplier_debt`).
    #[test]
    fn a_shop_reads_its_own_ledger_and_never_the_shop_next_door() {
        let (_dir, mut conn) = open();
        let customer = a_customer(&mut conn, "Cliente Amrani");
        append(&mut conn, &movement(customer, DebtKind::Sale, 100_000, 0)).unwrap();
        diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
            .execute(&mut conn)
            .unwrap();

        assert!(balances(&mut conn, 2).unwrap().is_empty());
        assert!(ledger(&mut conn, 2, customer).unwrap().is_empty());
        // And this shop still has what it wrote, so the two reads above are
        // answering "not this shop's" rather than "nothing is there".
        assert_eq!(
            balances(&mut conn, SHOP).unwrap(),
            vec![(customer, 100_000, 0)]
        );
    }

    #[test]
    fn a_movement_with_no_moment_on_it_is_refused_rather_than_dated_by_the_file() {
        // The column's default is SQLite's CURRENT_TIMESTAMP, which is UTC,
        // and the day a shop counts its drawer for is a day on the shop's
        // calendar (UTC+1). A payment taken at 00:30 in Algiers would land on
        // the day before in the file and drop out of that count and out of
        // the cash position, which reads this column through
        // `repos::cash::customer_payments`. So the caller stamps it from the
        // clock or the row does not go in.
        let (_dir, mut conn) = open();
        let customer = a_customer(&mut conn, "Cliente Sans Heure");
        let mut unstamped = movement(customer, DebtKind::Payment, 0, 1_000);
        unstamped.payment_mode = Some(PaymentMethod::Cash);
        unstamped.created_at = None;
        match append(&mut conn, &unstamped) {
            Err(CoreError::Unstamped { entity }) => assert_eq!(entity, "debt_ledger"),
            other => panic!("expected an unstamped row to be refused, got {other:?}"),
        }
        // And nothing was written: a refusal leaves the ledger as it was.
        assert!(ledger(&mut conn, SHOP, customer).unwrap().is_empty());
    }

    #[test]
    fn a_stamped_movement_keeps_the_moment_it_was_handed() {
        // The mirror of the test above, so the refusal cannot be read as
        // this repo refusing every payment. What it adds over
        // `debt_service::a_movement_is_stamped_by_the_shops_clock_and_not_by_utc`,
        // which already holds the calendar a layer up, is exactness: the
        // moment comes back the same to the nanosecond, so the column keeps
        // what it was handed rather than something near it.
        let (_dir, mut conn) = open();
        let customer = a_customer(&mut conn, "Cliente Amrani");
        let at = dzpos_kernel::services::clock::now();
        let mut stamped = movement(customer, DebtKind::Payment, 0, 1_000);
        stamped.payment_mode = Some(PaymentMethod::Cash);
        stamped.created_at = Some(at);
        let written = append(&mut conn, &stamped).unwrap();
        assert_eq!(written.created_at, at);
        assert_eq!(ledger(&mut conn, SHOP, customer).unwrap().len(), 1);
    }
}
