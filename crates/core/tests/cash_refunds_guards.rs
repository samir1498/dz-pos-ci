// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! What cash handed back is refused for, what it may not exceed, and what it
//! leaves behind in the audit log (features.md §1; ruling 5 of the 2026-09-20
//! loop).
//!
//! The other half of `cash_refunds_service`, which is the same subject read
//! as a day and a month. Split at the line limit, not by kind of test: the
//! fixtures both halves use live in `common::cash_refunds`.
//!
//! Every figure is written by hand from the rule. The bound the guards stand
//! on is what was actually paid in against the paper, never `remaining_debt`:
//! an adjustment zeroes that column with no money coming in at all, so a
//! written-off facture would otherwise read as paid for and open the drawer.

use chrono::NaiveDate;
use dzpos_core::error::CoreError;
use dzpos_core::money::{Money, PaymentMode};
use dzpos_core::services::avoir::{self, AvoirLine};
use dzpos_core::services::cash_refunds::Refund;
use dzpos_core::services::clock::{Month, Period};
use dzpos_core::services::debt::PaymentMethod;
use dzpos_core::services::shifts::NewShift;
use dzpos_core::services::{audit, cancellation, cash, debt, documents, products, shifts};

mod common;
use common::cash_refunds::{
    a_facture, a_second_shop, an_anonymous_cash_facture, at, day, last_after, line, product,
};
use common::shifts::{a_sale, a_sale_in, AMINA};
use common::{an_identified_customer, open_temp, open_temp_selling_factures};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// A cash-refunded partial avoir does not let a later cancellation erase the
/// facture's day.
///
/// `repos::cash::sales_of` keeps a cancelled document in its day when cash
/// went back on it, and the refund row of a partial avoir names the **avoir**
/// and not the facture. Reading only the row that names the document loses
/// this: the facture drops out of day 14 retroactively and that day reads
/// `0 - the refund` where the drawer actually held the sale less the refund.
///
/// Nothing refuses this sequence, and nothing should: the goods came back on
/// the credit note, the shop gave notes for part of it and then annulled the
/// rest without giving anything more. A file restored from a backup can hold
/// the same shape with no service involved, so the query is where it is
/// answered rather than a guard on the write path.
#[test]
fn a_facture_part_refunded_in_cash_keeps_its_day_when_it_is_later_annulled() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    // 4 units at 1 000,00, no TVA: 4 000,00 TTC, and the droit de timbre on
    // that is 40 tranches of 100 DA at 1 DA each, 40,00. The drawer took
    // 4 040,00 on day 14.
    let facture = a_facture(&mut conn, c, vec![line(p, 4_000)], PaymentMode::Cash, 14);
    assert_eq!(facture.totals.total_ttc, Money::centimes(400_000));
    assert_eq!(facture.totals.stamp, Money::centimes(4_000));
    assert_eq!(facture.totals.net_to_pay, Money::centimes(404_000));

    // One unit back in cash on the same day: 1 000,00 out of the drawer.
    avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![AvoirLine {
            document_line_id: facture.lines[0].id,
            qty_milli: 1_000,
        }]),
        Some("une unité".to_string()),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap();

    let before = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    assert_eq!(before.cash_in.sales, Money::centimes(404_000));
    assert_eq!(before.cash_out.refunds, Money::centimes(100_000));
    assert_eq!(before.cash, Money::centimes(304_000));

    // Then the rest is annulled two days later with nothing handed over.
    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "le client ne revient pas".to_string(),
        Some(at(16, 11)),
    )
    .unwrap();

    // Day 14 reads exactly what it read before: the sale came in on day 14
    // and one unit of it went back on day 14, and a cancellation on day 16
    // moves neither figure.
    let after = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    assert_eq!(after.cash_in.sales, Money::centimes(404_000));
    assert_eq!(after.cash_out.refunds, Money::centimes(100_000));
    assert_eq!(after.cash, Money::centimes(304_000));

    // Day 16 moved no cash at all: nothing was handed over.
    let sixteenth = cash::position(&mut conn, SHOP, Period::Day(day(16))).unwrap();
    assert_eq!(sixteenth.cash_in.sales, Money::ZERO);
    assert_eq!(sixteenth.cash_out.refunds, Money::ZERO);

    // And the month holds both days and nothing else.
    let month = cash::position(
        &mut conn,
        SHOP,
        Period::Month(Month::new(2026, 9).unwrap()),
    )
    .unwrap();
    assert_eq!(month.cash_in.sales, Money::centimes(404_000));
    assert_eq!(month.cash_out.refunds, Money::centimes(100_000));
    assert_eq!(month.cash, Money::centimes(304_000));
}

/// A facture written off by a downward adjustment owes nothing, and cash on
/// it is still refused: an adjustment zeroes `remaining_debt` with no money
/// coming in (features.md §3), so the column that used to stand as the guard
/// says "settled" about a paper nobody ever paid.
#[test]
fn a_facture_written_off_by_an_adjustment_is_not_refunded_in_cash() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    let facture = a_facture(&mut conn, c, vec![line(p, 4_000)], PaymentMode::Credit, 14);
    assert_eq!(facture.totals.net_to_pay, Money::centimes(400_000));

    // The shop writes the debt off. Nothing came over the counter.
    debt::adjust(
        &mut conn,
        SHOP,
        OWNER,
        c,
        Money::centimes(-400_000),
        Some("créance abandonnée".to_string()),
    )
    .unwrap();
    assert_eq!(
        documents::get(&mut conn, SHOP, facture.id)
            .unwrap()
            .balance
            .map(|b| b.remaining_debt),
        Some(Money::ZERO),
        "the write-off left the paper asking for nothing"
    );

    let err = avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        None,
        Some("retour".to_string()),
        Some(at(14, 13)),
        Refund::Cash,
    )
    .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "refund"),
        "{err:?}"
    );
    assert_eq!(
        cash::position(&mut conn, SHOP, Period::Day(day(14)))
            .unwrap()
            .cash_out
            .refunds,
        Money::ZERO
    );
}

/// A credit facture the customer actually paid may be refunded in cash for
/// the whole of it: that money did come over the counter.
#[test]
fn a_credit_facture_paid_in_full_is_refunded_in_cash_for_the_whole_of_it() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    let facture = a_facture(&mut conn, c, vec![line(p, 4_000)], PaymentMode::Credit, 14);
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        c,
        Money::centimes(400_000),
        PaymentMethod::Cash,
        None,
        at(14, 11),
    )
    .unwrap();

    let credit = avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        None,
        Some("retour".to_string()),
        Some(at(14, 13)),
        Refund::Cash,
    )
    .unwrap();
    assert_eq!(credit.totals.net_to_pay, Money::centimes(400_000));

    let position = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    // The payment came in and the refund went out, so the day is level.
    assert_eq!(position.cash_in.customer_payments, Money::centimes(400_000));
    assert_eq!(position.cash_out.refunds, Money::centimes(400_000));
    assert_eq!(position.cash, Money::ZERO);
}

/// A credit facture half paid hands back the half and not a centime more.
#[test]
fn a_credit_facture_half_paid_is_refunded_in_cash_up_to_that_half() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    // 4 units at 1 000,00 on credit: 4 000,00, of which 2 000,00 is paid.
    let facture = a_facture(&mut conn, c, vec![line(p, 4_000)], PaymentMode::Credit, 14);
    let line_id = facture.lines[0].id;
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        c,
        Money::centimes(200_000),
        PaymentMethod::Cash,
        None,
        at(14, 11),
    )
    .unwrap();

    let asked = |qty| {
        Some(vec![AvoirLine {
            document_line_id: line_id,
            qty_milli: qty,
        }])
    };
    // Three units is 3 000,00, more than the 2 000,00 that came in.
    let err = avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        asked(3_000),
        Some("trop".to_string()),
        Some(at(14, 13)),
        Refund::Cash,
    )
    .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "refund"),
        "{err:?}"
    );

    // Two units is exactly what came in, so it goes back.
    let credit = avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        asked(2_000),
        Some("la moitié".to_string()),
        Some(at(14, 14)),
        Refund::Cash,
    )
    .unwrap();
    assert_eq!(credit.totals.net_to_pay, Money::centimes(200_000));
    assert_eq!(
        cash::position(&mut conn, SHOP, Period::Day(day(14)))
            .unwrap()
            .cash_out
            .refunds,
        Money::centimes(200_000)
    );

    // And a second credit note cannot take the same money again: the half is
    // spent, so one more unit is refused.
    let err = avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        asked(1_000),
        Some("encore".to_string()),
        Some(at(14, 15)),
        Refund::Cash,
    )
    .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "refund"),
        "{err:?}"
    );
}

/// A cash facture cancelled after a partial avoir puts back only the goods
/// the avoir left on the shelf, not the line whole.
///
/// `avoir::issue` already returned the credited unit. `return_the_goods`
/// reading `line.qty_milli` would put it back a second time, and the shop
/// would read a count it does not hold.
#[test]
fn a_cancellation_after_a_partial_avoir_puts_back_only_what_is_left() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    let before = products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli;
    // Four units leave the shelf.
    let facture = a_facture(&mut conn, c, vec![line(p, 4_000)], PaymentMode::Cash, 14);
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        before - 4_000
    );

    // One comes back on a credit note.
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![AvoirLine {
            document_line_id: facture.lines[0].id,
            qty_milli: 1_000,
        }]),
        Some("une unité".to_string()),
        Some(at(14, 12)),
    )
    .unwrap();
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        before - 3_000
    );

    // Then the facture is annulled. The three still out come back, and the
    // one already back does not come back twice.
    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "le reste aussi".to_string(),
        Some(at(14, 13)),
    )
    .unwrap();
    assert_eq!(
        products::get(&mut conn, SHOP, p).unwrap().qty_on_hand_milli,
        before,
        "the shelf holds what it held before the sale, and not a unit more"
    );
}

/// A card sale handed back in cash. The card takings keep the sale — the TPE
/// did take that money — the drawer wears the payout, and the cashier's
/// expected figure drops by it even though no card sale ever reached it.
///
/// The shift is the point of the case: `takings_for` reads cash sales only,
/// so the expected figure never had the card sale in it, and subtracting the
/// refund is what makes the drawer come out right.
#[test]
fn a_card_sale_handed_back_in_cash_leaves_the_drawer_and_not_the_card_takings() {
    let (_dir, mut conn) = open_temp();
    let shift = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(common::shifts::at(8, 0, 0)),
            opening_cash: Money::centimes(1_000_000),
        },
    )
    .unwrap();
    let card = a_sale(
        &mut conn,
        AMINA,
        PaymentMode::Card,
        300_000,
        common::shifts::at(9, 0, 0),
    );
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        AMINA,
        card,
        "retour, remboursé en espèces".to_string(),
        Some(common::shifts::at(10, 0, 0)),
        Refund::Cash,
    )
    .unwrap();

    let position = cash::position(&mut conn, SHOP, Period::Day(common::shifts::day())).unwrap();
    // The TPE took it and the card column keeps it.
    assert_eq!(position.card_in.sales, Money::centimes(300_000));
    // The drawer never held it and now it is 3 000,00 lighter.
    assert_eq!(position.cash_in.sales, Money::ZERO);
    assert_eq!(position.cash_out.refunds, Money::centimes(300_000));
    assert_eq!(position.cash, Money::centimes(-300_000));

    let report = shifts::report(&mut conn, SHOP, shift.id).unwrap();
    // No cash came in on this drawer at all.
    assert_eq!(report.takings.sales, Money::ZERO);
    assert_eq!(report.refunds, Money::centimes(300_000));
    // 10 000,00 opening less the 3 000,00 handed over.
    assert_eq!(report.expected, Money::centimes(700_000));
}

/// A sale on the last day of a month, refunded in cash on the first of the
/// next. Each month carries its own half and the two come to nothing.
///
/// The half-open bounds are what the case is about: the sale belongs to
/// September on `issued_at` and the refund to October on `refunded_at`, and a
/// query that read either against the wrong end of the range would put both
/// in one month and leave the other at zero.
#[test]
fn a_sale_on_the_last_of_the_month_refunded_on_the_first_of_the_next_splits_across_them() {
    let (_dir, mut conn) = open_temp();
    let last = NaiveDate::from_ymd_opt(2026, 9, 30)
        .and_then(|d| d.and_hms_opt(23, 30, 0))
        .unwrap();
    let first = NaiveDate::from_ymd_opt(2026, 10, 1)
        .and_then(|d| d.and_hms_opt(0, 30, 0))
        .unwrap();
    let ticket = a_sale(&mut conn, AMINA, PaymentMode::Cash, 300_000, last);
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        AMINA,
        ticket,
        "retour le lendemain".to_string(),
        Some(first),
        Refund::Cash,
    )
    .unwrap();

    let september = cash::position(
        &mut conn,
        SHOP,
        Period::Month(Month::new(2026, 9).unwrap()),
    )
    .unwrap();
    // September took it and gave nothing back: the sale stays on the month
    // it was rung even though it was annulled the next day.
    assert_eq!(september.cash_in.sales, Money::centimes(300_000));
    assert_eq!(september.cash_out.refunds, Money::ZERO);
    assert_eq!(september.cash, Money::centimes(300_000));

    let october = cash::position(
        &mut conn,
        SHOP,
        Period::Month(Month::new(2026, 10).unwrap()),
    )
    .unwrap();
    assert_eq!(october.cash_in.sales, Money::ZERO);
    assert_eq!(october.cash_out.refunds, Money::centimes(300_000));
    assert_eq!(october.cash, Money::centimes(-300_000));

    // And the two months come to nothing on that ticket.
    assert_eq!(
        september.cash.checked_add(october.cash).unwrap(),
        Money::ZERO
    );
}

/// One shop's refund never reaches another shop's day, on either arm of the
/// `exists` clause or on the sum beside it (rule 3).
///
/// Both shops ring a cash ticket on the same day and both annul it, and only
/// shop 2 hands the notes back. Shop 1's day has to read its ticket gone and
/// no refund at all; a query missing its `shop_id` filter would hold shop 1's
/// cancelled ticket in its takings on the strength of shop 2's row.
#[test]
fn one_shops_refund_never_reaches_another_shops_day() {
    let (_dir, mut conn) = open_temp();
    a_second_shop(&mut conn);

    let mine = a_sale_in(&mut conn, SHOP, AMINA, PaymentMode::Cash, 300_000, at(14, 11));
    let theirs = a_sale_in(&mut conn, 2, AMINA, PaymentMode::Cash, 500_000, at(14, 11));

    // Shop 1 annuls with nothing handed back.
    cancellation::cancel(
        &mut conn,
        SHOP,
        AMINA,
        mine,
        "erreur".to_string(),
        Some(at(14, 12)),
    )
    .unwrap();
    // Shop 2 annuls the same day and hands the notes over.
    cancellation::cancel_settling(
        &mut conn,
        2,
        AMINA,
        theirs,
        "retour".to_string(),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap();

    let ours = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    // Our ticket left our day, because we handed nothing back. Shop 2's row
    // must not hold it here.
    assert_eq!(ours.cash_in.sales, Money::ZERO);
    assert_eq!(ours.cash_out.refunds, Money::ZERO);
    assert_eq!(ours.cash, Money::ZERO);

    let theirs = cash::position(&mut conn, 2, Period::Day(day(14))).unwrap();
    assert_eq!(theirs.cash_in.sales, Money::centimes(500_000));
    assert_eq!(theirs.cash_out.refunds, Money::centimes(500_000));
    assert_eq!(theirs.cash, Money::ZERO);

    // And the drawer figure is scoped the same way.
    assert_eq!(
        cash::refunds_for(&mut conn, SHOP, AMINA, at(14, 0), at(15, 0)).unwrap(),
        Money::ZERO
    );
    assert_eq!(
        cash::refunds_for(&mut conn, 2, AMINA, at(14, 0), at(15, 0)).unwrap(),
        Money::centimes(500_000)
    );
}

/// The walk-in with no fiche leaves an audit row behind.
///
/// `issue` used to return the moment it found no customer, which skipped the
/// log along with the ledger it had nothing to write to. That is the one sale
/// ruling 5 exists for, so 200,00 could leave the drawer with nothing in the
/// audit log at all.
#[test]
fn a_cash_avoir_on_a_facture_naming_nobody_is_written_into_the_audit_log() {
    let (_dir, mut conn) = open_temp();
    let facture = an_anonymous_cash_facture(&mut conn, 200_000, 2_000, at(14, 10));
    let credit = avoir::issue_settling(
        &mut conn,
        SHOP,
        AMINA,
        facture.id,
        None,
        Some("retour".to_string()),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap();

    let entry = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == "document.avoir")
        .expect("a credit note on a facture naming nobody wrote no audit entry");
    assert_eq!(entry.entity, "document");
    assert_eq!(entry.entity_id, Some(facture.id));
    assert_eq!(entry.user_id, AMINA);
    let after: serde_json::Value = serde_json::from_str(&entry.after.unwrap()).unwrap();
    assert_eq!(after["avoir_document_id"], credit.id);
    assert_eq!(after["amount_centimes"], 200_000);
    // No account to read, so the balance is the zero it always was.
    assert_eq!(after["balance_centimes"], 0);
    assert_eq!(after["settlement"], "cash");
    assert_eq!(after["cash_back_centimes"], 200_000);
}

/// Both reversal actions say in the log how the money went back, and what
/// went over the counter when it did.
///
/// Without it a credit note that opened the drawer and one that credited an
/// account are the same row, and the drawer is the one somebody answers for.
#[test]
fn the_audit_log_says_whether_the_notes_went_back_or_the_ledger_did() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");

    // A credit facture the customer paid, credited back in notes.
    let paid = a_facture(&mut conn, c, vec![line(p, 2_000)], PaymentMode::Credit, 14);
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        c,
        Money::centimes(200_000),
        PaymentMethod::Cash,
        None,
        at(14, 11),
    )
    .unwrap();
    avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        paid.id,
        None,
        Some("retour".to_string()),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap();
    let cash_avoir = last_after(&mut conn, "document.avoir");
    assert_eq!(cash_avoir["settlement"], "cash");
    assert_eq!(cash_avoir["cash_back_centimes"], 200_000);

    // A second credit facture, still owed for, credited on the account.
    let owed = a_facture(&mut conn, c, vec![line(p, 2_000)], PaymentMode::Credit, 15);
    avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        owed.id,
        None,
        Some("retour".to_string()),
        Some(at(15, 12)),
    )
    .unwrap();
    let ledger_avoir = last_after(&mut conn, "document.avoir");
    assert_eq!(ledger_avoir["settlement"], "ledger");
    assert_eq!(ledger_avoir["cash_back_centimes"], serde_json::Value::Null);

    // And the same on the cancellation action, both ways.
    let ticket = a_sale(&mut conn, OWNER, PaymentMode::Cash, 300_000, at(16, 10));
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        OWNER,
        ticket,
        "retour".to_string(),
        Some(at(16, 11)),
        Refund::Cash,
    )
    .unwrap();
    let cash_cancel = last_after(&mut conn, "document.cancel");
    assert_eq!(cash_cancel["settlement"], "cash");
    assert_eq!(cash_cancel["cash_back_centimes"], 300_000);

    let other = a_sale(&mut conn, OWNER, PaymentMode::Cash, 300_000, at(16, 12));
    cancellation::cancel(
        &mut conn,
        SHOP,
        OWNER,
        other,
        "erreur".to_string(),
        Some(at(16, 13)),
    )
    .unwrap();
    let quiet_cancel = last_after(&mut conn, "document.cancel");
    assert_eq!(quiet_cancel["settlement"], "ledger");
    assert_eq!(quiet_cancel["cash_back_centimes"], serde_json::Value::Null);
}

