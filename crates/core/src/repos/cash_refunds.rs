//! The only place `cash_refunds` touches diesel: one writer and the two sums
//! the cash figures are made of. Every query is scoped by `shop_id` (rule 3),
//! and the narrow one by `user_id` as well, because a till shift asks what
//! one person handed back and never what the shop did.
//!
//! The reads are half open, `>= from` and `< until`, the way `repos::cash`
//! reads its four ledgers, so a refund handed over at the second a shift
//! closed belongs to the next window and is counted once.

use chrono::NaiveDateTime;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Nullable};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::cash_refund::{CashRefund, CashRefundRow, CashRefundRowWrite};
use crate::money::Money;
use crate::models::debt::DebtKind;
use crate::models::sql_types::DocumentKind;
use crate::schema::{cash_refunds, debt_allocations, debt_ledger, documents};

/// Writes the row that says the drawer opened. The unique index on
/// `document_id` refuses a second refund against the same paper, and that
/// refusal comes back as a validation error with a field on it: a retried
/// request is a 400 saying what is wrong and never a 500.
pub fn append(
    conn: &mut SqliteConnection,
    write: &CashRefundRowWrite,
) -> Result<CashRefund, CoreError> {
    let row: CashRefundRow = match diesel::insert_into(cash_refunds::table)
        .values(write)
        .returning(CashRefundRow::as_returning())
        .get_result(conn)
    {
        Ok(row) => row,
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) => {
            return Err(CoreError::validation(
                "document_id",
                "cash has already been handed back against this document",
            ))
        }
        Err(other) => return Err(CoreError::Query(other)),
    };
    Ok(CashRefund::from(row))
}

/// What the shop handed back over a day or a month: the outgoing half of the
/// cash position.
pub fn total_for_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Money, CoreError> {
    total_of(conn, shop_id, None, from, until)
}

/// What one person handed back over one stretch of the clock: what a till
/// shift takes off that drawer's expected figure.
///
/// By the person who handed the cash over and never by whoever rang the sale.
/// Cashier B refunding cashier A's ticket is B's drawer that is light, and a
/// filter on the document's author would take it off A's evening instead and
/// leave both counts wrong.
pub fn total_for_user(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Money, CoreError> {
    total_of(conn, shop_id, Some(user_id), from, until)
}

/// Every centime handed back over the counter against one paper: on the paper
/// itself, and on any avoir written against it.
///
/// Both arms, because the two write paths name different documents. A
/// cancelled cash sale's row names the sale; an avoir's row names the avoir,
/// which carries `ref_document_id` back to the facture. A caller asking "how
/// much of this facture has already gone back in notes" wants the sum of
/// both, and reading one arm would let a facture be refunded twice over.
///
/// Not windowed: the question is about a paper's whole life, not about a day.
pub fn total_against_document(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<Money, CoreError> {
    let avoirs = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::kind.eq(DocumentKind::Avoir))
        .filter(documents::ref_document_id.eq(document_id))
        .select(documents::id);
    let total: Option<i64> = cash_refunds::table
        .filter(cash_refunds::shop_id.eq(shop_id))
        .filter(
            cash_refunds::document_id
                .eq(document_id)
                .or(cash_refunds::document_id.eq_any(avoirs)),
        )
        .select(sql::<Nullable<BigInt>>("SUM(amount_centimes)"))
        .first(conn)?;
    Ok(Money::centimes(total.unwrap_or(0)))
}

/// The most a credit note on this paper may still hand back over the counter:
/// what was actually paid in against it, less whatever has already gone back.
///
/// **Not `remaining_debt`.** That column is what the paper is still asking
/// for, and a downward adjustment zeroes it with no money coming in at all
/// (features.md §3), so a written-off facture reads as settled and a guard
/// that trusted it would open the drawer for goods nobody ever paid for.
///
/// What counts as paid in: the whole `net_to_pay` on a cash or card document,
/// which was settled when it was issued, and on a credit document the sum of
/// the **payment** allocations against it. The kind filter is the point —
/// `debt_allocations` also carries what an avoir settled and what a write-off
/// took off, and neither is money that came in.
///
/// Here and not in `repos::debt` because the subtraction beside it is this
/// module's own, and the two are one answer: a caller reading them apart
/// could take the refunds off a figure that was never the paid-in one.
pub fn still_to_hand_back(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
    paid_on_the_spot: Option<Money>,
) -> Result<Money, CoreError> {
    let paid_in = match paid_on_the_spot {
        Some(amount) => amount,
        None => {
            let total: Option<i64> = debt_allocations::table
                .inner_join(debt_ledger::table)
                .filter(debt_allocations::shop_id.eq(shop_id))
                .filter(debt_allocations::document_id.eq(document_id))
                .filter(debt_ledger::kind.eq(DebtKind::Payment))
                .select(sql::<Nullable<BigInt>>("SUM(debt_allocations.amount_centimes)"))
                .first(conn)?;
            Money::centimes(total.unwrap_or(0))
        }
    };
    let already = total_against_document(conn, shop_id, document_id)?;
    // Floored at zero: a file that disagrees with itself answers "nothing
    // left" rather than a negative bound that would read as a refusal for the
    // wrong reason.
    Ok(paid_in.checked_sub(already)?.max(Money::ZERO))
}

/// One query, asked about the shop or about one person. `None` is the shop,
/// the shape `repos::cash::sales_of` uses for the same pair of questions.
fn total_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: Option<i32>,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<Money, CoreError> {
    let mut query = cash_refunds::table
        .filter(cash_refunds::shop_id.eq(shop_id))
        .filter(cash_refunds::refunded_at.ge(from))
        .filter(cash_refunds::refunded_at.lt(until))
        .into_boxed();
    if let Some(user_id) = user_id {
        query = query.filter(cash_refunds::user_id.eq(user_id));
    }
    let total: Option<i64> = query
        .select(sql::<Nullable<BigInt>>("SUM(amount_centimes)"))
        .first(conn)?;
    Ok(Money::centimes(total.unwrap_or(0)))
}

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::repos::testdb::{open, OWNER, SHOP};

    fn at(day: u32, hour: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 9, day)
            .and_then(|d| d.and_hms_opt(hour, 0, 0))
            .expect("a real moment")
    }

    fn write(document_id: i32, centimes: i64, day: u32, hour: u32) -> CashRefundRowWrite {
        CashRefundRowWrite {
            shop_id: SHOP,
            document_id,
            user_id: OWNER,
            amount_centimes: centimes,
            refunded_at: at(day, hour),
        }
    }

    /// The round trip, and the index behind it: a second refund naming the
    /// same paper is refused, and refused as a validation error rather than
    /// as a bare query failure.
    #[test]
    fn one_document_is_refunded_once() {
        let (_dir, mut conn) = open();
        let document = seed_a_ticket(&mut conn);
        let written = append(&mut conn, &write(document, 3_000, 10, 12)).expect("the first refund");
        assert_eq!(written.amount, Money::centimes(3_000));
        assert_eq!(written.user_id, OWNER);

        let err = append(&mut conn, &write(document, 3_000, 10, 13)).expect_err("the second");
        assert!(
            matches!(&err, CoreError::Validation { field, .. } if field == "document_id"),
            "a second refund on one document is a validation error, was {err:?}"
        );
    }

    /// A ticket to hang the refund on. Written straight in: the repo is being
    /// asked about its own table and not about how a sale is made.
    fn seed_a_ticket(conn: &mut SqliteConnection) -> i32 {
        #[derive(diesel::QueryableByName)]
        struct Id {
            #[diesel(sql_type = diesel::sql_types::Integer)]
            id: i32,
        }
        diesel::sql_query(
            "INSERT INTO documents (shop_id, kind, series, number, issued_at, user_id, \
             seller_name, payment_mode, regime, total_ht_centimes, discount_centimes, \
             subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes, \
             net_to_pay_centimes, status) \
             VALUES (1, 'ticket', 'doc_ticket:2026', \
             (SELECT COALESCE(MAX(number), 0) + 1 FROM documents), \
             '2026-09-10 12:00:00', 1, 'A shop', \
             'cash', 'reel', 3000, 0, 3000, 0, 3000, 0, 3000, 'issued')",
        )
        .execute(conn)
        .expect("a seeded ticket");
        let found: Id = diesel::sql_query("SELECT MAX(id) AS id FROM documents")
            .get_result(conn)
            .expect("its id");
        found.id
    }

    /// The shop sum takes every person's refunds; the narrow one takes the
    /// named person's and leaves the other's alone. Both are half open, so a
    /// refund at the closing second falls outside.
    #[test]
    fn the_shop_sum_and_one_persons_sum_are_not_the_same_question() {
        let (_dir, mut conn) = open();
        let first = seed_a_ticket(&mut conn);
        let second = seed_a_ticket(&mut conn);
        append(&mut conn, &write(first, 3_000, 10, 12)).expect("the owner's refund");
        let other = CashRefundRowWrite {
            user_id: 2,
            ..write(second, 5_000, 10, 13)
        };
        diesel::sql_query(
            "INSERT INTO users (id, shop_id, name, role, active) \
             VALUES (2, 1, 'B', 'cashier', 1)",
        )
        .execute(&mut conn)
        .expect("a second cashier");
        append(&mut conn, &other).expect("the other cashier's refund");

        assert_eq!(
            total_for_shop(&mut conn, SHOP, at(10, 0), at(11, 0)).expect("the shop's day"),
            Money::centimes(8_000)
        );
        assert_eq!(
            total_for_user(&mut conn, SHOP, OWNER, at(10, 0), at(11, 0)).expect("one person"),
            Money::centimes(3_000)
        );
        // Half open at the top: the 13:00 refund is outside a window ending
        // at 13:00 and the 12:00 one is inside a window starting there.
        assert_eq!(
            total_for_shop(&mut conn, SHOP, at(10, 12), at(10, 13)).expect("one hour"),
            Money::centimes(3_000)
        );
        // A window with nothing in it answers zero rather than nothing.
        assert_eq!(
            total_for_shop(&mut conn, SHOP, at(11, 0), at(12, 0)).expect("an empty day"),
            Money::ZERO
        );
    }
}
