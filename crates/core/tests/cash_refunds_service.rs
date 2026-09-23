// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Cash handed back over the counter (features.md §1; ruling 5 of the
//! 2026-09-20 loop), against a real temp SQLite file: what it does to a day,
//! to a month and to the drawer the notes came out of.
//!
//! Every expected figure here is written by hand from the rule and never
//! computed by the code under test: a test that asked `position` for the
//! number it then asserts would pass against any arithmetic at all.
//!
//! The awkward case this whole file exists around is the double count.
//! `repos::cash::sales_of` drops a cancelled document out of its day, so a
//! refund written on top of that would take the same ticket off the shop
//! twice. The rule is that a sale refunded in cash stays in the takings of
//! the day it was rung — the drawer did take that money — and only a
//! cancellation that handed nothing back drops out.
//!
//! What a refund is refused for, and what it writes into the audit log, is
//! `cash_refunds_guards`: one subject, split at the line limit.

use dzpos_core::error::{CoreError, RetailError};
use dzpos_core::money::{Money, PaymentMode};
use dzpos_core::services::avoir::{self, AvoirLine};
use dzpos_core::services::cash_refunds::Refund;
use dzpos_core::services::clock::{Month, Period};
use dzpos_core::services::debt::PaymentMethod;
use dzpos_core::services::documents::DocumentStatus;
use dzpos_core::services::shifts::{NewShift, TillCount};
use dzpos_core::services::{cancellation, cash, debt, documents, shifts};

mod common;
use common::cash_refunds::{a_facture, an_anonymous_cash_facture, at, day, line, product};
use common::shifts::{a_sale, a_sale_with_stamp, a_second_cashier, AMINA, KARIM};
use common::{an_identified_customer, open_temp, open_temp_selling_factures};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// A cash refund lowers the day's cash figure and the open shift's expected
/// figure by the same centimes, which is ruling 5 in one sentence.
///
/// Both figures by hand. The drawer opened with 10 000, took a 3 000 ticket
/// and gave it straight back, so it is holding 10 000 again; the shop's day
/// took 3 000 and paid 3 000 out, so its net is nothing.
#[test]
fn one_refund_lowers_the_day_and_the_open_drawer_by_the_same_centimes() {
    let (_dir, mut conn) = open_temp();
    let shift = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(at(14, 8)),
            opening_cash: Money::centimes(1_000_000),
        },
    )
    .unwrap();
    let ticket = a_sale(
        &mut conn,
        AMINA,
        PaymentMode::Cash,
        300_000,
        common::shifts::at(9, 0, 0),
    );
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        AMINA,
        ticket,
        "article défectueux".to_string(),
        Some(common::shifts::at(10, 0, 0)),
        Refund::Cash,
    )
    .unwrap();

    let report = shifts::report(&mut conn, SHOP, shift.id).unwrap();
    assert_eq!(report.takings.sales, Money::centimes(300_000));
    assert_eq!(report.refunds, Money::centimes(300_000));
    // 10 000,00 opening + 3 000,00 taken - 3 000,00 handed back.
    assert_eq!(report.expected, Money::centimes(1_000_000));

    let position = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    assert_eq!(position.cash_in.sales, Money::centimes(300_000));
    assert_eq!(position.cash_out.refunds, Money::centimes(300_000));
    assert_eq!(position.cash, Money::ZERO);

    // And the drawer counts out at its opening figure with no note needed,
    // which is the shortage this feature exists to stop the cashier wearing.
    let closed = shifts::close(
        &mut conn,
        SHOP,
        AMINA,
        shift.id,
        TillCount {
            counted: Money::centimes(1_000_000),
            at: Some(common::shifts::at(18, 0, 0)),
            note: None,
        },
    )
    .unwrap();
    let close = closed.close.expect("a closed shift");
    assert_eq!(close.expected, Money::centimes(1_000_000));
    assert_eq!(close.difference().unwrap(), Money::ZERO);
}

/// The trap. A ticket rung on Monday and handed back on Wednesday stays in
/// Monday's takings, its refund lands on Wednesday, and the month nets to
/// nothing on it.
///
/// Without the second arm of `repos::cash::sales_of` the cancellation takes
/// the ticket out of Monday as well, and the shop is short of it twice: once
/// where it was sold and once where it went back.
///
/// The ticket carries no droit de timbre (nothing is due at or under 300 DA,
/// features.md §3), so the two halves are the same figure and the month is
/// exactly zero rather than zero plus a stamp.
#[test]
fn a_ticket_refunded_on_wednesday_stays_in_mondays_takings_and_the_month_nets_to_nothing() {
    let (_dir, mut conn) = open_temp();
    let ticket = a_sale(&mut conn, AMINA, PaymentMode::Cash, 300_000, at(14, 11));
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        AMINA,
        ticket,
        "retour".to_string(),
        Some(at(16, 11)),
        Refund::Cash,
    )
    .unwrap();

    // Monday: the drawer took 3 000,00 and nothing went back that day.
    let monday = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    assert_eq!(monday.cash_in.sales, Money::centimes(300_000));
    assert_eq!(monday.cash_out.refunds, Money::ZERO);
    assert_eq!(monday.cash, Money::centimes(300_000));

    // Wednesday: nothing was sold and 3 000,00 went back out.
    let wednesday = cash::position(&mut conn, SHOP, Period::Day(day(16))).unwrap();
    assert_eq!(wednesday.cash_in.sales, Money::ZERO);
    assert_eq!(wednesday.cash_out.refunds, Money::centimes(300_000));
    assert_eq!(wednesday.cash, Money::centimes(-300_000));

    // The month holds both and comes to nothing.
    let month =
        cash::position(&mut conn, SHOP, Period::Month(Month::new(2026, 9).unwrap())).unwrap();
    assert_eq!(month.cash_in.sales, Money::centimes(300_000));
    assert_eq!(month.cash_out.refunds, Money::centimes(300_000));
    assert_eq!(month.cash, Money::ZERO);
}

/// A cancellation that handed nothing back still drops out of its day. The
/// other half of the rule above, and without it the second arm could be a
/// filter that let every cancelled document through.
#[test]
fn a_cancellation_with_no_cash_back_still_leaves_the_day_it_was_sold_on() {
    let (_dir, mut conn) = open_temp();
    let ticket = a_sale(&mut conn, AMINA, PaymentMode::Cash, 300_000, at(14, 11));
    cancellation::cancel(
        &mut conn,
        SHOP,
        AMINA,
        ticket,
        "erreur de saisie".to_string(),
        Some(at(16, 11)),
    )
    .unwrap();

    let monday = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    assert_eq!(monday.cash_in.sales, Money::ZERO);
    let wednesday = cash::position(&mut conn, SHOP, Period::Day(day(16))).unwrap();
    assert_eq!(wednesday.cash_out.refunds, Money::ZERO);
}

/// The stamp is never given back, on the cancellation path.
///
/// A cash facture of 2 000,00 carries a 20,00 droit de timbre, so the
/// customer handed 2 020,00 over. What goes back is 2 000,00: the stamp is
/// paid on money that changed hands (Code du timbre 2026 art. 100-I) and a
/// reversal does not unmake that.
#[test]
fn a_cancelled_cash_sale_hands_back_everything_except_the_stamp() {
    let (_dir, mut conn) = open_temp();
    let ticket = a_sale_with_stamp(
        &mut conn,
        SHOP,
        AMINA,
        PaymentMode::Cash,
        200_000,
        2_000,
        at(14, 11),
    );
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        AMINA,
        ticket,
        "retour".to_string(),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap();

    let position = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    // The drawer took the stamp with the rest.
    assert_eq!(position.cash_in.sales, Money::centimes(202_000));
    assert_eq!(position.cash_in.stamp, Money::centimes(2_000));
    // And gave back everything but the stamp.
    assert_eq!(position.cash_out.refunds, Money::centimes(200_000));
    // So the day keeps the 20,00 the shop is holding for the state.
    assert_eq!(position.cash, Money::centimes(2_000));
}

/// The stamp is never given back, on the avoir path either, and nothing on
/// that path has to arrange it: an avoir carries no stamp at all, so its
/// `net_to_pay` is already the figure without one.
#[test]
fn an_avoir_settled_in_cash_hands_back_its_own_figure_which_carries_no_stamp() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 1900);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    // A cash facture: it is paid for, so cash may go back against it, and it
    // carries a stamp the credit note must not refund.
    let facture = a_facture(&mut conn, c, vec![line(p, 3_000)], PaymentMode::Cash, 14);
    // Typed from the fixture and not read off the answer: 3 units at 1 000,00
    // is 3 000,00 HT, 570,00 of TVA at 19% on top is 3 570,00 TTC, and the
    // droit de timbre on that is 36 tranches of 100 DA at 1 DA each, 36,00.
    // The customer handed 3 606,00 over.
    assert_eq!(facture.totals.total_ttc, Money::centimes(357_000));
    assert_eq!(facture.totals.stamp, Money::centimes(3_600));
    assert_eq!(facture.totals.net_to_pay, Money::centimes(360_600));

    let credit = avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        None,
        Some("retour marchandise".to_string()),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap();
    // The credit note is the facture without its stamp.
    assert_eq!(credit.totals.stamp, Money::ZERO);
    assert_eq!(credit.totals.total_ttc, Money::centimes(357_000));
    assert_eq!(credit.totals.net_to_pay, Money::centimes(357_000));
    // And it says the account did not move, because it did not.
    let triple = credit.balance.expect("an avoir to a named customer");
    assert_eq!(triple.old_balance, Money::ZERO);
    assert_eq!(triple.remaining_debt, Money::ZERO);
    assert_eq!(triple.total_debt, Money::ZERO);

    let position = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    assert_eq!(position.cash_out.refunds, Money::centimes(357_000));
    // The facture still stands, so its takings never left the day.
    assert_eq!(position.cash_in.sales, Money::centimes(360_600));
    // What the shop keeps is the stamp and nothing else.
    assert_eq!(position.cash, Money::centimes(3_600));

    // And the customer was not credited as well: notes, not credit.
    assert_eq!(debt::balance(&mut conn, SHOP, c).unwrap(), Money::ZERO);
}

/// A partial avoir refunds its share and no more.
///
/// One unit of three off a facture of 3 000,00 HT at 19%: the credit note is
/// written for its own third, and that third is what leaves the drawer.
#[test]
fn a_partial_avoir_settled_in_cash_hands_back_its_share_and_no_more() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 1900);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    let facture = a_facture(&mut conn, c, vec![line(p, 3_000)], PaymentMode::Cash, 14);
    let line_id = facture.lines[0].id;

    let credit = avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        Some(vec![AvoirLine {
            document_line_id: line_id,
            qty_milli: 1_000,
        }]),
        Some("une unité".to_string()),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap();

    // 1 000,00 HT and 190,00 of TVA at 19%, and no stamp.
    assert_eq!(credit.totals.total_ht, Money::centimes(100_000));
    assert_eq!(credit.totals.total_ttc, Money::centimes(119_000));
    assert_eq!(credit.totals.stamp, Money::ZERO);

    let position = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    assert_eq!(position.cash_out.refunds, Money::centimes(119_000));
}

/// Cash against a facture that is still owed for is refused.
///
/// The plan's own case: a facture of 20 000 with 15 000 still on it. Handing
/// notes over would be paying for goods nobody paid for, and the customer
/// would be credited twice — once by the notes and once by the ledger row the
/// unchanged path writes. Nothing is written by the refusal: no number is
/// burned, no goods move and no refund row exists.
#[test]
fn cash_against_a_facture_that_is_still_owed_for_is_refused() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    // 20 000,00 on credit, 5 000,00 paid: 15 000,00 still owed.
    let facture = a_facture(&mut conn, c, vec![line(p, 20_000)], PaymentMode::Credit, 14);
    assert_eq!(facture.totals.net_to_pay, Money::centimes(2_000_000));
    debt::pay(
        &mut conn,
        SHOP,
        OWNER,
        c,
        Money::centimes(500_000),
        PaymentMethod::Cash,
        None,
        at(14, 11),
    )
    .unwrap();

    let err = avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        None,
        Some("retour".to_string()),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap_err();
    assert!(
        matches!(&err, RetailError::Kernel(CoreError::Validation { field, .. }) if field == "refund"),
        "{err:?}"
    );

    // The refusal wrote nothing at all. No avoir took a number.
    assert_eq!(
        avoir::list_for(&mut conn, SHOP, facture.id).unwrap().len(),
        0
    );
    // No cash left the drawer.
    let position = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    assert_eq!(position.cash_out.refunds, Money::ZERO);
    // And the debt is where it was: 15 000,00 still owed.
    assert_eq!(
        documents::get(&mut conn, SHOP, facture.id)
            .unwrap()
            .balance
            .map(|b| b.remaining_debt),
        Some(Money::centimes(1_500_000))
    );

    // The same credit note on the ledger path is accepted, which is what says
    // the refusal is about the cash and not about the facture.
    let credit = avoir::issue(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        None,
        Some("retour".to_string()),
        Some(at(14, 13)),
    )
    .unwrap();
    assert_eq!(credit.totals.net_to_pay, Money::centimes(2_000_000));
}

/// Cash on a credit sale is refused: the money is on the account and comes
/// off it there.
#[test]
fn cash_back_on_a_cancelled_credit_sale_is_refused() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    let facture = a_facture(&mut conn, c, vec![line(p, 3_000)], PaymentMode::Credit, 14);

    let err = cancellation::cancel_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "commande annulée".to_string(),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap_err();
    assert!(
        matches!(&err, RetailError::Kernel(CoreError::Validation { field, .. }) if field == "refund"),
        "{err:?}"
    );
    // Nothing moved: the facture still stands.
    assert_eq!(
        documents::get(&mut conn, SHOP, facture.id).unwrap().status,
        DocumentStatus::Issued
    );
}

/// A second refund against one document is refused as a validation error with
/// a field on it, never as a 500 out of the unique index.
#[test]
fn a_second_refund_against_one_document_is_refused_on_the_field() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    let facture = a_facture(&mut conn, c, vec![line(p, 1_000)], PaymentMode::Cash, 14);
    let line_id = facture.lines[0].id;

    let asked = |qty| {
        Some(vec![AvoirLine {
            document_line_id: line_id,
            qty_milli: qty,
        }])
    };
    // Two partial credit notes are two documents, so the index is not what
    // stops the second one; the file has to be asked about one document
    // twice, which `cash_refunds::hand_over` is the way to do.
    let first = avoir::issue_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        asked(400),
        None,
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap();

    let err = dzpos_core::services::cash_refunds::hand_over(
        &mut conn,
        SHOP,
        OWNER,
        first.id,
        Money::centimes(1),
        at(14, 13),
    )
    .unwrap_err();
    assert!(
        matches!(&err, CoreError::Validation { field, .. } if field == "document_id"),
        "a second refund on one paper is a validation error, was {err:?}"
    );

    // And the drawer is out by the first refund only: 400 thousandths of a
    // line of 1 000,00 at no TVA, so 400,00, typed from the fixture.
    assert_eq!(first.totals.net_to_pay, Money::centimes(40_000));
    let position = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    assert_eq!(position.cash_out.refunds, Money::centimes(40_000));
}

/// A reversal that comes to nothing hands nothing over, and says so on the
/// field rather than writing a row saying zero cash moved.
#[test]
fn a_refund_of_nothing_is_refused_on_the_field() {
    let (_dir, mut conn) = open_temp();
    // A ticket worth only its stamp: `net_to_pay - stamp` is zero.
    let ticket = a_sale_with_stamp(
        &mut conn,
        SHOP,
        AMINA,
        PaymentMode::Cash,
        0,
        500,
        at(14, 11),
    );
    let err = cancellation::cancel_settling(
        &mut conn,
        SHOP,
        AMINA,
        ticket,
        "retour".to_string(),
        Some(at(14, 12)),
        Refund::Cash,
    )
    .unwrap_err();
    assert!(
        matches!(&err, RetailError::Kernel(CoreError::Validation { field, .. }) if field == "refund"),
        "{err:?}"
    );
}

/// Attribution. Karim refunds Amina's ticket, so Karim's drawer is short by
/// it and Amina's is not.
///
/// The refund row carries whoever handed the notes over, and a shift
/// subtracts by its own opener. A reader that filtered the document's ringer
/// instead would take it off Amina's evening, leaving Karim over by 3 000 and
/// Amina short by the same, both for doing nothing wrong.
#[test]
fn the_drawer_that_is_short_is_the_one_the_notes_came_out_of() {
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    let amina = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(common::shifts::at(8, 0, 0)),
            opening_cash: Money::centimes(1_000_000),
        },
    )
    .unwrap();
    let karim = shifts::open(
        &mut conn,
        SHOP,
        KARIM,
        NewShift {
            opened_at: Some(common::shifts::at(8, 0, 0)),
            opening_cash: Money::centimes(1_000_000),
        },
    )
    .unwrap();

    // Amina rings it; Karim hands the notes back.
    let ticket = a_sale(
        &mut conn,
        AMINA,
        PaymentMode::Cash,
        300_000,
        common::shifts::at(9, 0, 0),
    );
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        KARIM,
        ticket,
        "retour".to_string(),
        Some(common::shifts::at(10, 0, 0)),
        Refund::Cash,
    )
    .unwrap();

    let hers = shifts::report(&mut conn, SHOP, amina.id).unwrap();
    // Her takings keep the sale and her drawer keeps the money: she was
    // handed 3 000,00 and never gave any of it back.
    assert_eq!(hers.takings.sales, Money::centimes(300_000));
    assert_eq!(hers.refunds, Money::ZERO);
    assert_eq!(hers.expected, Money::centimes(1_300_000));

    let his = shifts::report(&mut conn, SHOP, karim.id).unwrap();
    assert_eq!(his.takings.sales, Money::ZERO);
    assert_eq!(his.refunds, Money::centimes(300_000));
    assert_eq!(his.expected, Money::centimes(700_000));
}

/// A facture that was part credited already hands back only what is left on
/// it, never its whole figure again.
///
/// `cancellation::effect_of` answers `StockBack` for every document that put
/// no money on an account, without asking what earlier avoirs already took
/// off it, so the amount cannot come from the facture's own totals. It comes
/// from `avoir::what_is_left`, the same subtraction the credit path is
/// measured by, which is the facture's `total_ttc` when no avoir exists and
/// so leaves a clean cancellation handing back `net_to_pay - stamp`.
///
/// The unique index cannot catch this one: the partial avoir's refund row
/// names the avoir and the cancellation's names the facture, so two different
/// papers each hold a row and the drawer is out by more than the sale.
#[test]
fn a_facture_already_credited_in_part_hands_back_only_what_is_left_on_it() {
    let (_dir, mut conn) = open_temp_selling_factures();
    let p = product(&mut conn, "Ciment", 100_000, 0);
    let c = an_identified_customer(&mut conn, "Entreprise Benali");
    // 3 units at 1 000,00, no TVA, paid over the counter. Every figure below
    // is worked out by hand from features.md's droit de timbre row and typed
    // as a literal; reading `facture.totals.stamp` back off the code under
    // test, which is what this did until 2026-09-21, proves only that the
    // code agrees with itself.
    //
    //   total_ttc  = 3 x 100 000 c                 = 300 000 c (3 000,00 DA)
    //   tranches   = ceil(3 000 DA / 100 DA)       =      30
    //   band       = 3 000 DA is under 30 000 DA   =   1 DA a tranche
    //   stamp      = 30 x 1 DA = 30,00 DA          =   3 000 c (over the
    //                                                  5 DA minimum, no cap
    //                                                  in reach)
    //   net_to_pay = 300 000 + 3 000               = 303 000 c
    let facture = a_facture(&mut conn, c, vec![line(p, 3_000)], PaymentMode::Cash, 14);
    assert_eq!(facture.totals.total_ttc, Money::centimes(300_000));
    assert_eq!(facture.totals.stamp, Money::centimes(3_000));
    assert_eq!(facture.totals.net_to_pay, Money::centimes(303_000));

    // One of the three units comes back in cash: 1 000,00.
    let partial = avoir::issue_settling(
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
    assert_eq!(partial.totals.net_to_pay, Money::centimes(100_000));

    // Then the whole thing is annulled, in cash again. What is left to hand
    // back is 2 000,00 and not the facture's own 3 000,00.
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        OWNER,
        facture.id,
        "retour du reste".to_string(),
        Some(at(14, 13)),
        Refund::Cash,
    )
    .unwrap();

    let position = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    // 1 000,00 on the credit note and 2 000,00 on the cancellation.
    assert_eq!(position.cash_out.refunds, Money::centimes(300_000));
    // The drawer took the sale and the stamp: 303 000 c in, 300 000 c back
    // out, and the 3 000 c of stamp is what stays. The stamp is never given
    // back (features.md §1, "Cash handed back").
    assert_eq!(position.cash_in.sales, Money::centimes(303_000));
    assert_eq!(position.cash, Money::centimes(3_000));
}

/// The case ruling 5 was taken for: a facture with no customer, credited in
/// cash. There is no ledger to write the credit on, so the refund row is the
/// only record the money left, and it is written all the same.
///
/// `avoir::issue` returns early for a document naming nobody — there is no
/// account to move — so a refund written after that point would be skipped
/// on exactly the sale this feature exists for, and every other test here
/// names a customer and would stay green.
#[test]
fn an_avoir_on_a_facture_naming_nobody_still_writes_the_cash_that_left() {
    let (_dir, mut conn) = open_temp();
    // Written straight through `services::documents` with its totals stated:
    // what is under test is the refund, not how a basket adds up. No product
    // on the line, so there is no sale movement for the credit note to price
    // the goods back at and the stock half stays out of the way.
    let facture = an_anonymous_cash_facture(&mut conn, 200_000, 2_000, at(14, 10));
    assert_eq!(facture.customer_id, None);

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
    assert_eq!(credit.customer_id, None);
    assert_eq!(credit.totals.stamp, Money::ZERO);

    let position = cash::position(&mut conn, SHOP, Period::Day(day(14))).unwrap();
    // The drawer took 2 020,00 and gave 2 000,00 back, keeping the stamp.
    assert_eq!(position.cash_in.sales, Money::centimes(202_000));
    assert_eq!(position.cash_out.refunds, Money::centimes(200_000));
    assert_eq!(position.cash, Money::centimes(2_000));

    // And the drawer it came out of is short by it.
    assert_eq!(
        cash::refunds_for(&mut conn, SHOP, AMINA, at(14, 0), at(15, 0)).unwrap(),
        Money::centimes(200_000)
    );
}

/// The two edges of the window a drawer subtracts its refunds over: the
/// moment it opened belongs to it, the moment it was counted does not.
///
/// `repos::cash_refunds::total_of` filters `refunded_at >= from` and
/// `refunded_at < until`, and `shifts::close` hands it `opened_at` and
/// `closed_at`. Half open at the top is what keeps two drawers of one person
/// that meet at a moment from both claiming the same notes — the same shape
/// `services::shifts::covers` gives a sale — and closed at the bottom is what
/// stops a refund handed over the second the drawer opened from belonging to
/// nobody. A test that only refunded at half past the hour passes under `>`
/// and under `>=` alike, which is why both refunds here sit exactly on a
/// bound.
///
/// The figure, by hand: 10 000,00 float, plus the 5 000,00 ticket rung at
/// 10:00 inside the window (a cash sale handed straight back stays in the
/// takings of the day it was rung, which is what the head of this file is
/// about), less the 3 000,00 handed back at 09:00:00 exactly. 1 000 000 +
/// 500 000 - 300 000 = 1 200 000 centimes.
#[test]
fn a_refund_at_the_opening_moment_is_this_drawers_and_one_at_the_counting_moment_is_not() {
    let (_dir, mut conn) = open_temp();
    let shift = shifts::open(
        &mut conn,
        SHOP,
        AMINA,
        NewShift {
            opened_at: Some(common::shifts::at(9, 0, 0)),
            opening_cash: Money::centimes(1_000_000),
        },
    )
    .unwrap();

    // Rung at 08:00, before this drawer was opened, and handed back at
    // 09:00:00 — the opening moment itself. The sale is outside the takings
    // window and the refund is inside the refund window, so this drawer is
    // short by it and took nothing for it.
    let earlier = a_sale(
        &mut conn,
        AMINA,
        PaymentMode::Cash,
        300_000,
        common::shifts::at(8, 0, 0),
    );
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        AMINA,
        earlier,
        "retour de la veille".to_string(),
        Some(common::shifts::at(9, 0, 0)),
        Refund::Cash,
    )
    .unwrap();

    // Rung at 10:00 and handed back at 19:00:00 — the moment the drawer is
    // counted, which the next stretch owns. The takings keep it; the refund
    // is the next drawer's problem.
    let inside = a_sale(
        &mut conn,
        AMINA,
        PaymentMode::Cash,
        500_000,
        common::shifts::at(10, 0, 0),
    );
    cancellation::cancel_settling(
        &mut conn,
        SHOP,
        AMINA,
        inside,
        "retour au comptoir".to_string(),
        Some(common::shifts::at(19, 0, 0)),
        Refund::Cash,
    )
    .unwrap();

    let closed = shifts::close(
        &mut conn,
        SHOP,
        shift.id,
        AMINA,
        TillCount {
            counted: Money::centimes(1_200_000),
            note: Some("compté à la fermeture".to_string()),
            at: Some(common::shifts::at(19, 0, 0)),
        },
    )
    .unwrap();
    let close = closed.close.clone().unwrap();
    assert_eq!(close.expected, Money::centimes(1_200_000));
    assert_eq!(close.difference().unwrap(), Money::ZERO);

    // And the report reads the same window back: one refund of the two.
    let report = shifts::report(&mut conn, SHOP, shift.id).unwrap();
    assert_eq!(report.refunds, Money::centimes(300_000));
    assert_eq!(report.takings.sales, Money::centimes(500_000));
}
