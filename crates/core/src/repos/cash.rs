//! The sums the cash position is made of. Four ledgers, one query each, every
//! one scoped by `shop_id` (rule 3) and every one coalesced to zero, so a day
//! nothing happened on answers zeros rather than nothing.
//!
//! Its own module rather than a handful of functions added to `documents`,
//! `debt` and `supplier_debt`: the four are read together, by one caller, over
//! one range of days, and reading them together is what makes the figure a
//! single answer instead of four that were taken at different moments.
//!
//! The bounds are half open, `>= the first moment of the first day` and
//! `< the first moment of the day after the last`. Both columns are stamped
//! by the shop's clock (`clock::now`, and `debt::append_at` for a movement
//! dated by the document it belongs to), so the comparison is on the same
//! calendar the range was asked on.
//!
//! Two of the four are asked twice: once about the shop, which is the cash
//! position, and once about one person over one stretch of an evening, which
//! is a till shift's expected figure. The pair share the query that says
//! which rows count and differ by the one `user_id` filter, because two
//! hand-written copies of "a cash sale is a ticket or a facture that still
//! stands" drift the day one of them learns something (a fifth document kind,
//! a second status) the other does not.

use chrono::NaiveDateTime;
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Bool, Integer, Nullable};
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::{DebtKind, PaymentMethod};
use crate::models::document::payment_mode_stored;
use crate::models::sql_types::{DocumentKind, DocumentStatus, SupplierDebtKind};
use crate::money::{Money, PaymentMode};
use crate::schema::{debt_ledger, documents, supplier_ledger};

/// What the shop sold and was paid for on the spot, and how much droit de
/// timbre came over the counter inside it.
///
/// Only a ticket and a facture: a proforma is a quotation nobody paid, an
/// avoir is a credit note, and the papers the supply side writes are not
/// sales. A document that still stands, or one whose cash was handed back
/// over the counter: an annulled ticket nobody was paid back for is money
/// that never stayed in the drawer and drops out of its day, but a ticket
/// refunded in cash did come in on its own day and goes out again on the day
/// the notes did, as a `cash_refunds` row.
///
/// Without that second arm the reversal is felt twice. A cashier rings
/// 3 000 DA cash and cancels it with the notes back inside one shift: the
/// drawer is where it started, and `opening + takings - refunds` would read
/// `opening - 3 000` because the sale had already left the takings. Over a
/// month the same ticket would be subtracted once for leaving the sales
/// column and once again as a refund.
///
/// The `avoir` path needs no arm here: a facture credited by an avoir keeps
/// its `Issued` status and never left the takings in the first place, and the
/// avoir itself is not a ticket or a facture so the kind filter above already
/// leaves it out.
///
/// `net_to_pay` and not `total_ttc`: what the drawer took is what the customer
/// handed over, and on a cash facture that is the amount plus the stamp
/// (`net_to_pay = total_ttc + stamp`, features.md §3). The stamp is summed
/// again on the same rows and comes back beside the takings, so a screen that
/// wants the shop's own money can take the tax it collects for the state back
/// out. It is a part of the first figure and never a second one to add.
///
/// Both sums are over one scan of the same rows, so the two can never be
/// answered about different sets of documents.
pub fn sales(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    mode: PaymentMode,
) -> Result<(Money, Money), CoreError> {
    sales_of(conn, shop_id, None, from, until, mode)
}

/// The same sum about one person: what this cashier rang up and was paid for
/// on the spot, over one stretch of the day.
///
/// The `user_id` filter is the whole of what a till shift adds, and it is not
/// optional. Two cashiers hold overlapping shifts on purpose, so the shop-wide
/// figure above handed to each of them counts the other one's takings: two
/// people each ringing 1 000 000 centimes between 09:00 and 14:00 would each
/// be told to expect 2 000 000 and each be recorded 1 000 000 short for doing
/// nothing wrong.
pub fn sales_by_user(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    mode: PaymentMode,
) -> Result<(Money, Money), CoreError> {
    sales_of(conn, shop_id, Some(user_id), from, until, mode)
}

/// One query, asked about the shop or about one person. `None` is the shop.
fn sales_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: Option<i32>,
    from: NaiveDateTime,
    until: NaiveDateTime,
    mode: PaymentMode,
) -> Result<(Money, Money), CoreError> {
    // Still standing, or handed back in cash. `EXISTS` and not a join: a
    // sum is the wrong place to find out that a join somebody widened later
    // counts a document twice.
    //
    // Two ways a paper can have had cash handed back on it, and both count.
    // The row names the document itself when a cash sale was cancelled with
    // the notes going back. It names an **avoir** when the money went back on
    // a credit note, and that avoir carries `ref_document_id` to the facture.
    // Reading only the first arm loses the second the moment such a facture
    // is annulled: it drops out of the day it was sold on and the day reads
    // `0 - the refund` where the drawer held the sale less the refund. That
    // state does not need a service to produce it — a file restored from a
    // backup can already hold it — so the query is where it is answered.
    //
    // Scoped by `shop_id` on both arms (rule 3): one shop's refund may not
    // decide what another shop's day reads.
    let refunded = sql::<Bool>("EXISTS (SELECT 1 FROM cash_refunds r WHERE r.shop_id = ")
        .bind::<Integer, _>(shop_id)
        .sql(
            " AND (r.document_id = documents.id \
             OR r.document_id IN (SELECT a.id FROM documents a \
             WHERE a.shop_id = r.shop_id AND a.kind = 'avoir' \
             AND a.ref_document_id = documents.id)))",
        );
    let mut query = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::kind.eq_any([DocumentKind::Ticket, DocumentKind::Facture]))
        .filter(documents::status.eq(DocumentStatus::Issued).or(refunded))
        .filter(documents::payment_mode.eq(payment_mode_stored(mode)))
        .filter(documents::issued_at.ge(from))
        .filter(documents::issued_at.lt(until))
        .into_boxed();
    if let Some(user_id) = user_id {
        query = query.filter(documents::user_id.eq(user_id));
    }
    let (took, stamp): (Option<i64>, Option<i64>) = query
        .select((
            sql::<Nullable<BigInt>>("SUM(net_to_pay_centimes)"),
            sql::<Nullable<BigInt>>("SUM(stamp_centimes)"),
        ))
        .first(conn)?;
    Ok((
        Money::centimes(took.unwrap_or(0)),
        Money::centimes(stamp.unwrap_or(0)),
    ))
}

/// Money a customer handed over against what they owed. A payment lowers the
/// debt, so it is the credit column that carries the amount.
pub fn customer_payments(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    method: PaymentMethod,
) -> Result<Money, CoreError> {
    customer_payments_of(conn, shop_id, None, from, until, method)
}

/// The same sum about one person: money against a debt that this cashier took
/// over the counter. `user_id` on `debt_ledger` is whoever wrote the movement,
/// which is who was handed the notes.
///
/// Not optional, for the reason `sales_by_user` above gives.
pub fn customer_payments_by_user(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    method: PaymentMethod,
) -> Result<Money, CoreError> {
    customer_payments_of(conn, shop_id, Some(user_id), from, until, method)
}

/// One query, asked about the shop or about one person. `None` is the shop.
fn customer_payments_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: Option<i32>,
    from: NaiveDateTime,
    until: NaiveDateTime,
    method: PaymentMethod,
) -> Result<Money, CoreError> {
    let mut query = debt_ledger::table
        .filter(debt_ledger::shop_id.eq(shop_id))
        .filter(debt_ledger::kind.eq(DebtKind::Payment))
        .filter(debt_ledger::payment_mode.eq(method))
        .filter(debt_ledger::created_at.ge(from))
        .filter(debt_ledger::created_at.lt(until))
        .into_boxed();
    if let Some(user_id) = user_id {
        query = query.filter(debt_ledger::user_id.eq(user_id));
    }
    let total: Option<i64> = query
        .select(sql::<Nullable<BigInt>>("SUM(credit_centimes)"))
        .first(conn)?;
    Ok(Money::centimes(total.unwrap_or(0)))
}

/// Money the shop handed over to a supplier. The mirror of the above: a
/// payment lowers what the shop owes, so it is a credit row here too.
pub fn supplier_payments(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
    method: PaymentMethod,
) -> Result<Money, CoreError> {
    let total: Option<i64> = supplier_ledger::table
        .filter(supplier_ledger::shop_id.eq(shop_id))
        .filter(supplier_ledger::kind.eq(SupplierDebtKind::Payment))
        .filter(supplier_ledger::payment_mode.eq(method))
        .filter(supplier_ledger::created_at.ge(from))
        .filter(supplier_ledger::created_at.lt(until))
        .select(sql::<Nullable<BigInt>>("SUM(credit_centimes)"))
        .first(conn)?;
    Ok(Money::centimes(total.unwrap_or(0)))
}
