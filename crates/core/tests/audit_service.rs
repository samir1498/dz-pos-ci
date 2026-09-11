// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The audit log of sensitive actions (features.md §5). Each call site is
//! asserted on the stored entry, and the entries that must not be written are
//! asserted too: a log that records everything hides the one that matters.

use chrono::NaiveDate;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::models::sql_types::Role;
use dzpos_core::models::user::NewUser;
use dzpos_core::money::{Bps, Money, Regime};
use dzpos_core::services::{audit, products, settings, shops, users};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::open_temp;

fn block(name: &str) -> StoreBlock {
    StoreBlock {
        name: name.to_string(),
        rc: Some("16/00-1234567 B 21".to_string()),
        nif: None,
        nis: None,
        ai: None,
        address: None,
        phone: None,
    }
}

fn draft(selling: i64, active: bool) -> NewProduct {
    NewProduct {
        name: "Sucre".to_string(),
        barcode: None,
        category_id: None,
        unit: Unit::Piece,
        cost: Money::centimes(500),
        selling: Money::centimes(selling),
        wholesale: None,
        qty_on_hand_milli: 0,
        low_stock_at_milli: 0,
        rate_bps: Some(Bps::new(1900).unwrap()),
        active,
    }
}

#[test]
fn a_new_database_has_an_empty_log() {
    let (_dir, mut conn) = open_temp();
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn writing_the_store_block_records_what_it_replaced() {
    let (_dir, mut conn) = open_temp();
    shops::update_store(&mut conn, SHOP, OWNER, block("Supérette El Bahdja")).unwrap();
    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].action, "update");
    assert_eq!(log[0].entity, "shop");
    assert_eq!(log[0].entity_id, Some(SHOP));
    assert_eq!(log[0].user_id, OWNER);
    let before = log[0].before.as_deref().unwrap();
    let after = log[0].after.as_deref().unwrap();
    assert!(before.contains("Mon magasin"), "{before}");
    assert!(after.contains("Supérette El Bahdja"), "{after}");
    assert!(after.contains("16/00-1234567 B 21"), "{after}");
}

#[test]
fn a_refused_store_block_records_nothing() {
    // The entry and the change are one transaction. A log of attempts that
    // never happened is a log nobody trusts.
    let (_dir, mut conn) = open_temp();
    let mut bad = block("Supérette");
    bad.name = "   ".to_string();
    assert!(shops::update_store(&mut conn, SHOP, OWNER, bad).is_err());
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn a_regime_change_records_the_one_it_left_and_the_day_it_takes() {
    let (_dir, mut conn) = open_temp();
    let day = NaiveDate::from_ymd_opt(2027, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    settings::set_regime(&mut conn, SHOP, OWNER, Regime::Ifu, day).unwrap();
    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].action, "set_regime");
    assert_eq!(log[0].entity, "regime_fiscal");
    assert!(log[0].before.as_deref().unwrap().contains("reel"));
    let after = log[0].after.as_deref().unwrap();
    assert!(after.contains("ifu"), "{after}");
    assert!(after.contains("2027-01-01"), "{after}");
}

#[test]
fn a_regime_change_the_shop_is_already_under_records_nothing() {
    let (_dir, mut conn) = open_temp();
    let day = NaiveDate::from_ymd_opt(2027, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    assert!(settings::set_regime(&mut conn, SHOP, OWNER, Regime::Reel, day).is_err());
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn a_price_change_is_recorded_with_both_amounts() {
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
        .unwrap()
        .id;
    products::update(&mut conn, SHOP, OWNER, p, draft(1_500, true)).unwrap();
    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].entity, "product");
    assert_eq!(log[0].entity_id, Some(p));
    assert!(log[0].before.as_deref().unwrap().contains("1000"));
    assert!(log[0].after.as_deref().unwrap().contains("1500"));
}

#[test]
fn taking_a_product_off_sale_is_recorded() {
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
        .unwrap()
        .id;
    products::update(&mut conn, SHOP, OWNER, p, draft(1_000, false)).unwrap();
    let log = audit::list(&mut conn, SHOP).unwrap();
    assert_eq!(log.len(), 1);
    let after = log[0].after.as_deref().unwrap();
    assert!(after.contains("\"active\":false"), "{after}");
}

#[test]
fn an_edit_that_touches_neither_a_price_nor_the_active_flag_records_nothing() {
    // features.md §5 names price changes and the deletion-like actions. A
    // renamed product on every keystroke would bury them.
    let (_dir, mut conn) = open_temp();
    let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
        .unwrap()
        .id;
    let mut renamed = draft(1_000, true);
    renamed.name = "Sucre cristallisé".to_string();
    products::update(&mut conn, SHOP, OWNER, p, renamed).unwrap();
    assert!(audit::list(&mut conn, SHOP).unwrap().is_empty());
}

/// `services::audit::read` (M4 T7's own screen, `crates/api/tests/
/// audit_log.rs` drives the same rule through HTTP). The row it reads back
/// is the same one `a_price_change_is_recorded_with_both_amounts` above
/// checks `audit::list` sees; this checks the paged, filtered read the
/// screen actually calls.
mod read {
    use super::*;
    use dzpos_core::services::audit::Filter;

    #[test]
    fn one_page_carries_the_row_and_the_two_dropdowns_own_options() {
        let (_dir, mut conn) = open_temp();
        let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
            .unwrap()
            .id;
        products::update(&mut conn, SHOP, OWNER, p, draft(1_500, true)).unwrap();

        let (page, facets) = audit::read(&mut conn, SHOP, &Filter::default(), 1).unwrap();
        assert_eq!(page.rows.len(), 1);
        assert_eq!(page.page, 1);
        assert!(!page.has_more);
        assert_eq!(page.rows[0].entry.entity_id, Some(p));
        assert!(!page.rows[0].user_name.is_empty());
        assert_eq!(facets.actions, vec!["update".to_string()]);
        assert_eq!(facets.users.len(), 1);
        assert_eq!(facets.users[0].0, OWNER);
    }

    #[test]
    fn a_user_id_that_wrote_nothing_answers_no_rows() {
        let (_dir, mut conn) = open_temp();
        let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
            .unwrap()
            .id;
        products::update(&mut conn, SHOP, OWNER, p, draft(1_500, true)).unwrap();

        // Creating the clerk is itself an audited action (`user.create`), so
        // the owner is now behind two rows, not one; the clerk, who has not
        // done anything yet, is behind none, which is the row count this
        // test is actually about.
        let clerk = users::create(
            &mut conn,
            SHOP,
            OWNER,
            NewUser {
                name: "Yasmine".to_string(),
                role: Role::Manager,
            },
        )
        .unwrap();

        let filter = Filter {
            user_id: Some(clerk.id),
            ..Filter::default()
        };
        let (page, _) = audit::read(&mut conn, SHOP, &filter, 1).unwrap();
        assert!(page.rows.is_empty());

        let filter = Filter {
            user_id: Some(OWNER),
            ..Filter::default()
        };
        let (page, _) = audit::read(&mut conn, SHOP, &filter, 1).unwrap();
        assert_eq!(page.rows.len(), 2);
    }

    /// The lockout row names, in the ordinary `user_id` column every other
    /// row's filter now runs a SQL `WHERE` against, the person the wrong
    /// PINs were tried against — not whoever a signed-in session would have
    /// named, because nobody is signed in when the row is written
    /// (`services::users::settle`'s own comment: "The actor is the user the
    /// attempts were made on: nobody knows who was standing there, and
    /// claiming otherwise in an audit log is worse than saying nothing").
    /// Filtering that person out of their own lockout row would be
    /// inventing an actor the row never claimed to have, so this checks the
    /// opposite: the SQL filter still finds it, the same as it did when the
    /// filter ran in memory, and the row still carries the passive action
    /// name (`user.locked_out`) that keeps it from reading as something
    /// they did.
    #[test]
    fn a_filter_by_the_locked_out_person_still_finds_their_own_lockout_row() {
        let (_dir, mut conn) = open_temp();
        let clerk = users::create(
            &mut conn,
            SHOP,
            OWNER,
            NewUser {
                name: "Karim".to_string(),
                role: Role::Cashier,
            },
        )
        .unwrap();
        users::set_pin(&mut conn, SHOP, OWNER, clerk.id, "1357", None).unwrap();

        let now = dzpos_core::services::clock::now();
        for _ in 0..users::FAILURES_BEFORE_LOCKOUT {
            let _ = users::verify_pin(&mut conn, SHOP, clerk.id, "9999", now);
        }

        let filter = Filter {
            user_id: Some(clerk.id),
            ..Filter::default()
        };
        let (page, _) = audit::read(&mut conn, SHOP, &filter, 1).unwrap();
        let lockout = page
            .rows
            .iter()
            .find(|r| r.entry.action == audit::ACTION_LOCK_OUT_USER)
            .expect("the lockout row is found under the locked-out user's own filter");
        assert_eq!(lockout.entry.user_id, clerk.id);
        assert_eq!(lockout.user_name, "Karim");
    }

    #[test]
    fn an_action_that_was_never_written_answers_no_rows() {
        let (_dir, mut conn) = open_temp();
        let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
            .unwrap()
            .id;
        products::update(&mut conn, SHOP, OWNER, p, draft(1_500, true)).unwrap();

        let filter = Filter {
            action: Some(audit::ACTION_PAY_DEBT.to_string()),
            ..Filter::default()
        };
        let (page, _) = audit::read(&mut conn, SHOP, &filter, 1).unwrap();
        assert!(page.rows.is_empty());
    }

    #[test]
    fn a_day_that_holds_no_row_answers_none_and_the_written_day_answers_one() {
        let (_dir, mut conn) = open_temp();
        let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
            .unwrap()
            .id;
        products::update(&mut conn, SHOP, OWNER, p, draft(1_500, true)).unwrap();
        // The shop's calendar day, not the UTC one the column stores
        // (`services::audit::day_range_utc`): the two agree all but one hour
        // in twenty-four, and this proves the filter reads the row on the
        // day it was actually written on, not on whichever the column
        // happens to spell at the moment the test runs.
        let created_at = audit::list(&mut conn, SHOP).unwrap()[0].created_at;
        let today = dzpos_core::services::clock::shop_time(
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(created_at, chrono::Utc),
        )
        .date();

        let filter = Filter {
            day: Some(today.pred_opt().unwrap()),
            ..Filter::default()
        };
        let (page, _) = audit::read(&mut conn, SHOP, &filter, 1).unwrap();
        assert!(page.rows.is_empty(), "{:?}", page.rows);

        let filter = Filter {
            day: Some(today),
            ..Filter::default()
        };
        let (page, _) = audit::read(&mut conn, SHOP, &filter, 1).unwrap();
        assert_eq!(page.rows.len(), 1);
    }

    /// `PAGE_SIZE` rows on the first page, the rest on the second, newest
    /// first: the log is a review screen and not the story `list` reads.
    #[test]
    fn a_shop_with_more_rows_than_a_page_pages_them_newest_first() {
        let (_dir, mut conn) = open_temp();
        let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
            .unwrap()
            .id;
        let total = audit::PAGE_SIZE + 1;
        for i in 0..total {
            // Alternating the price is what makes every call a change:
            // `products::update` logs nothing when the price does not move.
            let selling = 1_000 + i as i64 + 1;
            products::update(&mut conn, SHOP, OWNER, p, draft(selling, true)).unwrap();
        }

        let (first, _) = audit::read(&mut conn, SHOP, &Filter::default(), 1).unwrap();
        assert_eq!(first.rows.len(), audit::PAGE_SIZE);
        assert!(first.has_more);
        let newest = first.rows[0].entry.after.clone().unwrap();
        assert!(
            newest.contains(&(1_000 + total as i64).to_string()),
            "{newest}"
        );

        let (second, _) = audit::read(&mut conn, SHOP, &Filter::default(), 2).unwrap();
        assert_eq!(second.rows.len(), 1);
        assert!(!second.has_more);
    }

    /// Exactly a page and not one row more: `has_more` has to read `false`
    /// here, or the screen offers a next page that comes back empty. The
    /// paging test above only ever writes `PAGE_SIZE + 1` rows, so it would
    /// pass whether this boundary read `false` or `true`; this is the test
    /// that actually pins it.
    #[test]
    fn a_shop_with_exactly_a_page_of_rows_has_no_next_page() {
        let (_dir, mut conn) = open_temp();
        let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
            .unwrap()
            .id;
        for i in 0..audit::PAGE_SIZE {
            let selling = 1_000 + i as i64 + 1;
            products::update(&mut conn, SHOP, OWNER, p, draft(selling, true)).unwrap();
        }

        let (page, _) = audit::read(&mut conn, SHOP, &Filter::default(), 1).unwrap();
        assert_eq!(page.rows.len(), audit::PAGE_SIZE);
        assert!(
            !page.has_more,
            "a full first page is not a reason to ask for a second"
        );
    }

    /// A page number taken straight off the query string can be anything up
    /// to `i64::MAX`. `start` used to be `(page_number - 1) as usize *
    /// PAGE_SIZE`, which overflows and panics under overflow checks once
    /// `page_number` is large enough — a caller-controlled 500, not a bug
    /// that needs a caller to have a lot of data. It now saturates instead:
    /// an absurd page answers empty rather than crashing.
    #[test]
    fn an_absurd_page_number_answers_empty_rather_than_panicking() {
        let (_dir, mut conn) = open_temp();
        let p = products::create(&mut conn, SHOP, OWNER, draft(1_000, true))
            .unwrap()
            .id;
        products::update(&mut conn, SHOP, OWNER, p, draft(1_500, true)).unwrap();

        let (page, _) = audit::read(&mut conn, SHOP, &Filter::default(), i64::MAX).unwrap();
        assert!(page.rows.is_empty());
        assert!(!page.has_more);
    }
}

#[test]
fn the_log_is_scoped_to_its_shop() {
    use diesel::prelude::*;
    let (_dir, mut conn) = open_temp();
    shops::update_store(&mut conn, SHOP, OWNER, block("Supérette")).unwrap();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(&mut conn)
        .unwrap();
    assert_eq!(audit::list(&mut conn, SHOP).unwrap().len(), 1);
    assert!(audit::list(&mut conn, 2).unwrap().is_empty());

    // The owner's screen calls `read`, not `list`: `list`'s own scoping does
    // not prove `read`'s, which loads the same rows through different repo
    // calls (`search`, `count`, `distinct_actions`) and joins the shop's
    // own users on top.
    let (page, _) = audit::read(&mut conn, SHOP, &audit::Filter::default(), 1).unwrap();
    assert_eq!(page.rows.len(), 1);
    let (other, _) = audit::read(&mut conn, 2, &audit::Filter::default(), 1).unwrap();
    assert!(other.rows.is_empty());
}
