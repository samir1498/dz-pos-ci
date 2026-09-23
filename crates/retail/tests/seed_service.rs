// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The development seeder. It is not a fiscal rule, so what is asserted here
//! is not an amount: it is that the file it leaves behind is one a screen can
//! be developed against, that two runs leave the same one, and that the
//! invariants the rest of the suite holds over a shop hold over this one too.
//!
//! Why that last part matters. The seeder drives the services and never SQL,
//! so every rule and every audit row is real; a figure the dashboard reads
//! off the seeded file that does not add up is a bug in a service and not in
//! the seeder, and this file is where it would first show.

use std::time::Instant;

use chrono::NaiveDate;
use dzpos_kernel::services::clock::{Month, Period};
use dzpos_kernel::services::{audit, sessions, users};
use dzpos_retail::money::Money;
use dzpos_retail::services::{
    cash, dashboard, debt, documents, expenses, products, seed, suppliers,
};

mod common;

use common::open_temp;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// The day every case seeds up to. A fixed one, never the wall clock: a test
/// that seeded "today" would read a different month every day it ran.
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 30).unwrap()
}

#[test]
fn the_seeder_fills_an_empty_shop_with_a_catalogue_a_month_of_trading_and_the_papers_it_leaves() {
    let (_dir, mut conn) = open_temp();
    let counts = seed::run(&mut conn, SHOP, OWNER, today()).unwrap();

    assert_eq!(counts.products, 60);
    assert_eq!(counts.customers, 12);
    assert_eq!(counts.suppliers, 5);
    assert_eq!(counts.days, 30);
    assert!(counts.categories >= 6, "{counts:?}");
    // Every category the seeder opened says a seed opened it. An owner who
    // finds a category nobody typed asks the log where it came from, and
    // "seed" is the answer that stops the question; the import's own rows
    // say `product.import` for the same reason. Both sides of
    // `services::categories::create` are read here because only the import
    // side was, and a seed that quietly stopped auditing would have shipped.
    let entries = audit::list(&mut conn, SHOP).unwrap();
    let opened: Vec<serde_json::Value> = entries
        .iter()
        .filter(|e| e.entity == "category" && e.action == audit::ACTION_CREATE)
        .map(|e| serde_json::from_str(e.after.as_deref().unwrap_or("")).unwrap())
        .collect();
    assert_eq!(
        opened.len(),
        usize::try_from(counts.categories).unwrap(),
        "the seeder made {} categories and audited {}",
        counts.categories,
        opened.len()
    );
    for made in &opened {
        assert_eq!(
            made["source"], "seed",
            "a category does not say a seed made it"
        );
        assert!(
            made["name"].is_string(),
            "a category create has no name: {made}"
        );
        assert!(
            made["default_rate_bps"]
                .as_u64()
                .is_some_and(|r| r <= 10_000),
            "a category create carries no usable rate: {made}"
        );
    }
    // The window the brief asks for: twenty five to sixty sales a day.
    assert!(
        counts.sales >= 30 * 25 && counts.sales <= 30 * 60,
        "{counts:?}"
    );
    assert!(counts.avoirs >= 2, "{counts:?}");
    assert_eq!(counts.cancellations, 2, "{counts:?}");
    assert_eq!(counts.proformas, 1, "{counts:?}");
    assert!(counts.purchases >= 5, "{counts:?}");
    assert!(counts.customer_payments >= 5, "{counts:?}");
    assert!(counts.supplier_payments >= 5, "{counts:?}");
    // Every category the shop files an expense under has at least one row, so
    // no screen that groups by category comes back with a blank column.
    let categories = expenses::categories(&mut conn, SHOP).unwrap();
    assert!(counts.expenses >= i64::try_from(categories.len()).unwrap());
    let filed = expenses::list(&mut conn, SHOP, Month::of(today())).unwrap();
    let earlier = expenses::list(&mut conn, SHOP, Month::of(today().pred_opt().unwrap()));
    for category in &categories {
        assert!(
            filed
                .iter()
                .chain(earlier.iter().flatten())
                .any(|e| e.category_id == category.id),
            "nothing was ever filed under {}",
            category.key
        );
    }

    // The catalogue is what the counts say, and the fiches carry the two
    // rates the shop charges.
    let listed = products::list(&mut conn, SHOP).unwrap();
    assert_eq!(listed.len(), 60);
    let rates: Vec<u32> = {
        let mut rates: Vec<u32> = listed.iter().map(|p| p.rate_bps.as_u32()).collect();
        rates.sort_unstable();
        rates.dedup();
        rates
    };
    assert_eq!(rates, vec![900, 1900]);
    // Half the catalogue carries a barcode the supplier printed and half an
    // in store one the generator gave it; every one of them is thirteen
    // digits, because that is what a scanner reads.
    for product in &listed {
        let code = product.barcode.as_deref().unwrap_or_default();
        assert_eq!(code.len(), 13, "{}: {code}", product.name);
        assert!(code.chars().all(|c| c.is_ascii_digit()), "{code}");
    }
}

#[test]
fn the_dashboard_reads_figures_off_the_seeded_file_rather_than_zeros() {
    let (_dir, mut conn) = open_temp();
    seed::run(&mut conn, SHOP, OWNER, today()).unwrap();

    let read = dashboard::read(&mut conn, SHOP, today()).unwrap();
    assert!(read.today.sales_count > 0, "{:?}", read.today);
    assert!(read.today.sales_ttc > Money::ZERO);
    assert!(read.today.margin > Money::ZERO, "{:?}", read.today);
    assert!(read.this_month.expenses > Money::ZERO);
    assert!(read.this_month.cost_of_goods > Money::ZERO);
    assert!(read.cash_today.cash_in.sales > Money::ZERO);
    // The lists the screen draws beside the figures.
    assert_eq!(read.top_by_quantity.len(), 10);
    assert_eq!(read.top_by_margin.len(), 10);
    assert!(!read.low_stock.is_empty(), "nothing to reorder");
    assert!(read.customer_debt.parties > 0);
    assert!(read.supplier_debt.parties > 0);
    assert!(read.open_purchases > 0, "no order still waiting on goods");

    // And the chart: thirty days, none of them a gap, and the shop traded on
    // every one of them.
    let series = dashboard::series(&mut conn, SHOP, today(), 30).unwrap();
    assert_eq!(series.days.len(), 30);
    for point in &series.days {
        assert!(
            point.figures.sales_count > 0,
            "{} is a day with no sale on it",
            point.from
        );
    }
    assert_eq!(series.weeks.len(), 5);
}

#[test]
fn the_invariants_the_rest_of_the_suite_holds_over_a_shop_hold_over_the_seeded_one() {
    let (_dir, mut conn) = open_temp();
    seed::run(&mut conn, SHOP, OWNER, today()).unwrap();

    // The month is its own days, which is what the dashboard property holds
    // over a history nobody wrote by hand and this holds over the one the
    // seeder writes.
    let month = dashboard::read(&mut conn, SHOP, today())
        .unwrap()
        .this_month;
    let series = dashboard::series(&mut conn, SHOP, today(), 30).unwrap();
    let summed = series.days.iter().fold((0i64, 0i64, 0i64, 0i64), |a, p| {
        (
            a.0 + p.figures.sales_ttc.as_centimes(),
            a.1 + p.figures.sales_ht.as_centimes(),
            a.2 + p.figures.cost_of_goods.as_centimes(),
            a.3 + p.figures.expenses.as_centimes(),
        )
    });
    assert_eq!(summed.0, month.sales_ttc.as_centimes());
    assert_eq!(summed.1, month.sales_ht.as_centimes());
    assert_eq!(summed.2, month.cost_of_goods.as_centimes());
    assert_eq!(summed.3, month.expenses.as_centimes());

    // The cash position the dashboard shows is the one the cash rule answers,
    // read again over the same month.
    let read = dashboard::read(&mut conn, SHOP, today()).unwrap();
    assert_eq!(
        read.cash_this_month,
        cash::position(&mut conn, SHOP, Period::Month(Month::of(today()))).unwrap()
    );

    // What the screen says is owed is the ledgers added up, party by party,
    // and never a column somebody kept in step by hand.
    let owed = read.customer_debt;
    let mut by_hand = Money::ZERO;
    let mut parties = 0i64;
    for entry in
        dzpos_retail::services::customers::list_with_balance(&mut conn, SHOP, None).unwrap()
    {
        assert_eq!(
            entry.balance,
            debt::balance(&mut conn, SHOP, entry.customer.id).unwrap()
        );
        if entry.balance > Money::ZERO {
            by_hand = by_hand.checked_add(entry.balance).unwrap();
            parties += 1;
        }
    }
    assert_eq!(owed.total, by_hand);
    assert_eq!(owed.parties, parties);
    assert!(parties > 0, "nobody owes the shop anything");

    let owed = read.supplier_debt;
    let mut by_hand = Money::ZERO;
    for entry in suppliers::list_with_balance(&mut conn, SHOP, None).unwrap() {
        if entry.balance > Money::ZERO {
            by_hand = by_hand.checked_add(entry.balance).unwrap();
        }
    }
    assert_eq!(owed.total, by_hand);

    // Every paper it wrote is one the shop could have written: a cancelled
    // one keeps its number, and the series never gaps.
    for kind in [
        dzpos_retail::models::sql_types::DocumentKind::Ticket,
        dzpos_retail::models::sql_types::DocumentKind::Facture,
        dzpos_retail::models::sql_types::DocumentKind::Avoir,
    ] {
        let mut numbers: Vec<i64> = documents::list(&mut conn, SHOP, Some(kind))
            .unwrap()
            .iter()
            .map(|d| d.number)
            .collect();
        assert!(!numbers.is_empty(), "{kind:?}: no paper at all");
        numbers.sort_unstable();
        let first = numbers[0];
        for (step, number) in numbers.iter().enumerate() {
            assert_eq!(
                *number,
                first + i64::try_from(step).unwrap(),
                "{kind:?} skipped a number"
            );
        }
    }
}

#[test]
fn two_runs_of_the_seeder_leave_the_same_shop() {
    let (_one, mut first) = open_temp();
    let (_two, mut second) = open_temp();
    let counts = seed::run(&mut first, SHOP, OWNER, today()).unwrap();
    assert_eq!(
        counts,
        seed::run(&mut second, SHOP, OWNER, today()).unwrap()
    );

    // The counts matching is the weak half: two files with the same number of
    // sales can still hold different ones. The figures are the check.
    assert_eq!(
        dashboard::read(&mut first, SHOP, today()).unwrap(),
        dashboard::read(&mut second, SHOP, today()).unwrap()
    );
    assert_eq!(
        dashboard::series(&mut first, SHOP, today(), 30).unwrap(),
        dashboard::series(&mut second, SHOP, today(), 30).unwrap()
    );
    let names = |conn: &mut _| -> Vec<(String, Option<String>, i64)> {
        products::list(conn, SHOP)
            .unwrap()
            .into_iter()
            .map(|p| (p.name, p.barcode, p.qty_on_hand_milli))
            .collect()
    };
    assert_eq!(names(&mut first), names(&mut second));
}

#[test]
fn the_seeder_refuses_a_shop_that_already_holds_something() {
    let (_dir, mut conn) = open_temp();
    seed::run(&mut conn, SHOP, OWNER, today()).unwrap();
    assert!(!seed::is_empty(&mut conn, SHOP).unwrap());

    let err = seed::run(&mut conn, SHOP, OWNER, today()).unwrap_err();
    // Refused, and refused as a rule rather than as a crash: the binary turns
    // this into the sentence that tells the operator to pass --force.
    assert!(
        matches!(
            err,
            dzpos_retail::error::RetailError::Kernel(
                dzpos_retail::error::CoreError::Validation { .. }
            )
        ),
        "{err:?}"
    );
}

#[test]
fn a_seeded_shop_is_written_in_a_few_seconds_and_not_in_a_minute() {
    let (_dir, mut conn) = open_temp();
    let started = Instant::now();
    seed::run(&mut conn, SHOP, OWNER, today()).unwrap();
    let took = started.elapsed();
    // Generous on purpose: the box builds for other worktrees while this
    // runs, and a tight bound here would fail somebody else's gate rather
    // than report a seeder that got slow. Thirty seconds is still an order of
    // magnitude under the minutes an unbatched run would take.
    assert!(took.as_secs() < 30, "the seeder took {took:?}");
}

// The file a developer opens can be signed in to (M4 T2). The owner the first
// migration writes carries the `'!unset'` sentinel and no password, and from
// M4 on every route wants a session, so a seeded file without a credential is
// a file the API answers 401 to on every request. Both doors are checked,
// because the sign-in screen offers both.
#[test]
fn the_seeded_owner_can_sign_in_by_pin_and_by_password() {
    let (_dir, mut conn) = open_temp();
    seed::run(&mut conn, SHOP, OWNER, today()).unwrap();
    let at = today().and_hms_opt(9, 0, 0).unwrap();

    let by_pin = sessions::sign_in_with_pin(&mut conn, SHOP, OWNER, seed::OWNER_PIN, at).unwrap();
    assert_eq!(by_pin.actor.user_id, OWNER);

    let owner = users::get(&mut conn, SHOP, OWNER).unwrap();
    let by_password =
        sessions::sign_in_with_password(&mut conn, SHOP, &owner.name, seed::OWNER_PASSWORD, at)
            .unwrap();
    assert_eq!(by_password.actor.user_id, OWNER);
}
