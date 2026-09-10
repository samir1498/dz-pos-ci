//! The only place the supplier ledger touches diesel. Scoped by `shop_id`
//! like every other query (rule 3), and append-only: there is no update and
//! no delete here, because a mistake is corrected by an `adjustment` row
//! nobody can miss rather than by an edit nobody can see.

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::supplier_debt::{
    SupplierAllocation, SupplierAllocationRow, SupplierAllocationRowWrite, SupplierDebtRow,
    SupplierDebtRowWrite, SupplierEntry,
};
use crate::money::Money;
use crate::schema::{supplier_allocations, supplier_ledger};

pub fn append(
    conn: &mut SqliteConnection,
    write: &SupplierDebtRowWrite,
) -> Result<SupplierEntry, CoreError> {
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
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::supplier::SupplierRowWrite;
    use crate::models::supplier_debt::{PaymentMethod, SupplierDebtKind};
    use crate::repos::suppliers;
    use crate::repos::testdb::{open, OWNER, SHOP};

    fn a_supplier(conn: &mut SqliteConnection, name: &str) -> i32 {
        suppliers::insert(
            conn,
            &SupplierRowWrite {
                shop_id: SHOP,
                name: name.to_string(),
                phone: None,
                address: None,
                rc: None,
                nif: None,
                nis: None,
                ai: None,
                notes: None,
                active: true,
                updated_at: crate::services::clock::now(),
            },
        )
        .unwrap()
        .id
    }

    fn movement(
        supplier_id: i32,
        kind: SupplierDebtKind,
        debit: i64,
        credit: i64,
    ) -> SupplierDebtRowWrite {
        SupplierDebtRowWrite {
            shop_id: SHOP,
            supplier_id,
            purchase_id: None,
            kind,
            debit_centimes: debit,
            credit_centimes: credit,
            user_id: OWNER,
            note: None,
            payment_mode: None,
            created_at: None,
        }
    }

    #[test]
    fn a_supplier_with_no_movement_owes_nothing() {
        // The SUM is coalesced to zero, so an empty ledger answers a balance
        // rather than no row at all.
        let (_dir, mut conn) = open();
        let supplier = a_supplier(&mut conn, "Sarl Amrani");
        assert_eq!(
            balance(&mut conn, SHOP, supplier).unwrap(),
            crate::money::Money::ZERO
        );
        assert!(ledger(&mut conn, SHOP, supplier).unwrap().is_empty());
    }

    #[test]
    fn the_balance_is_the_sum_of_the_ledger_and_may_go_below_zero() {
        // A shop that has paid in advance is owed goods, and the ledger says
        // so rather than clamping at nothing.
        let (_dir, mut conn) = open();
        let supplier = a_supplier(&mut conn, "Sarl Amrani");
        append(
            &mut conn,
            &movement(supplier, SupplierDebtKind::Opening, 250_000, 0),
        )
        .unwrap();
        append(
            &mut conn,
            &movement(supplier, SupplierDebtKind::Purchase, 100_000, 0),
        )
        .unwrap();
        let mut paid = movement(supplier, SupplierDebtKind::Payment, 0, 400_000);
        paid.payment_mode = Some(PaymentMethod::Cash);
        append(&mut conn, &paid).unwrap();
        assert_eq!(
            balance(&mut conn, SHOP, supplier).unwrap(),
            crate::money::Money::centimes(-50_000)
        );
    }

    #[test]
    fn every_kind_and_both_modes_go_to_the_file_and_come_back() {
        let (_dir, mut conn) = open();
        let supplier = a_supplier(&mut conn, "Sarl Amrani");
        for kind in [
            SupplierDebtKind::Opening,
            SupplierDebtKind::Purchase,
            SupplierDebtKind::Return,
            SupplierDebtKind::Adjustment,
        ] {
            let written = append(&mut conn, &movement(supplier, kind, 1000, 0)).unwrap();
            assert_eq!(written.kind, kind);
            assert_eq!(written.payment_mode, None);
        }
        for mode in [PaymentMethod::Cash, PaymentMethod::Card] {
            let mut paid = movement(supplier, SupplierDebtKind::Payment, 0, 1000);
            paid.payment_mode = Some(mode);
            let written = append(&mut conn, &paid).unwrap();
            assert_eq!(written.kind, SupplierDebtKind::Payment);
            assert_eq!(written.payment_mode, Some(mode));
        }
    }

    #[test]
    fn the_ledger_reads_newest_first_and_the_id_breaks_a_tie_inside_one_second() {
        let (_dir, mut conn) = open();
        let supplier = a_supplier(&mut conn, "Sarl Amrani");
        let stamped = crate::services::clock::now();
        let mut ids = Vec::new();
        for _ in 0..3 {
            let mut write = movement(supplier, SupplierDebtKind::Purchase, 1000, 0);
            write.created_at = Some(stamped);
            ids.push(append(&mut conn, &write).unwrap().id);
        }
        ids.reverse();
        let read: Vec<i32> = ledger(&mut conn, SHOP, supplier)
            .unwrap()
            .into_iter()
            .map(|e| e.id)
            .collect();
        assert_eq!(read, ids);
    }

    #[test]
    fn each_suppliers_two_sums_come_back_in_one_query_for_the_whole_list() {
        let (_dir, mut conn) = open();
        let one = a_supplier(&mut conn, "Sarl Amrani");
        let two = a_supplier(&mut conn, "Bensalem");
        a_supplier(&mut conn, "Jamais servi");
        append(
            &mut conn,
            &movement(one, SupplierDebtKind::Purchase, 100_000, 0),
        )
        .unwrap();
        append(
            &mut conn,
            &movement(two, SupplierDebtKind::Return, 0, 30_000),
        )
        .unwrap();
        let mut rows = balances(&mut conn, SHOP).unwrap();
        rows.sort_by_key(|(supplier_id, _, _)| *supplier_id);
        // A supplier with no movement is not in the answer: no rows is no
        // debt, and the caller reads a missing one as nothing owed.
        assert_eq!(rows, vec![(one, 100_000, 0), (two, 0, 30_000)]);
    }

    #[test]
    fn another_shops_ledger_is_neither_summed_nor_listed() {
        let (_dir, mut conn) = open();
        let supplier = a_supplier(&mut conn, "Sarl Amrani");
        append(
            &mut conn,
            &movement(supplier, SupplierDebtKind::Purchase, 100_000, 0),
        )
        .unwrap();
        diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
            .execute(&mut conn)
            .unwrap();
        assert_eq!(
            balance(&mut conn, 2, supplier).unwrap(),
            crate::money::Money::ZERO
        );
        assert!(ledger(&mut conn, 2, supplier).unwrap().is_empty());
        assert!(balances(&mut conn, 2).unwrap().is_empty());
    }

    #[test]
    fn an_allocation_says_what_a_payment_settled_on_which_purchase() {
        let (_dir, mut conn) = open();
        let supplier = a_supplier(&mut conn, "Sarl Amrani");
        let purchase = crate::repos::purchases::insert(
            &mut conn,
            &crate::models::purchase::PurchaseRowWrite {
                shop_id: SHOP,
                supplier_id: supplier,
                supplier_document_number: None,
                purchase_date: "2026-09-10".to_string(),
                due_date: None,
                transport_centimes: 0,
                extra_costs_centimes: 0,
                status: crate::models::purchase::PurchaseStatus::Ordered,
                user_id: OWNER,
                note: None,
            },
        )
        .unwrap()
        .id;
        let mut paid = movement(supplier, SupplierDebtKind::Payment, 0, 60_000);
        paid.payment_mode = Some(PaymentMethod::Cash);
        let payment = append(&mut conn, &paid).unwrap();
        allocate(
            &mut conn,
            &SupplierAllocationRowWrite {
                shop_id: SHOP,
                payment_ledger_id: payment.id,
                purchase_id: purchase,
                amount_centimes: 60_000,
            },
        )
        .unwrap();
        let of_payment = allocations_of_payment(&mut conn, SHOP, payment.id).unwrap();
        assert_eq!(of_payment.len(), 1);
        assert_eq!(of_payment[0].amount, crate::money::Money::centimes(60_000));
        assert_eq!(of_payment[0].purchase_id, purchase);
        assert_eq!(allocations(&mut conn, SHOP, purchase).unwrap().len(), 1);
        assert!(allocations(&mut conn, 2, purchase).unwrap().is_empty());
        // An allocation points at a payment by id and the foreign key alone
        // would take another shop's row.
        assert!(entry_belongs_to_shop(&mut conn, SHOP, payment.id).unwrap());
        assert!(!entry_belongs_to_shop(&mut conn, 2, payment.id).unwrap());
    }
}
