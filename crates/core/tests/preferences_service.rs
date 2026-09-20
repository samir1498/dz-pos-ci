// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! What a shop has set for itself, read back by the shop that set it.
//!
//! The round trips and the refusals live next to the code in
//! `services::preferences`. This file holds the one thing an inline test
//! cannot hold: a second shop, which needs a row in `shops` that no service
//! writes, so the test has to say the INSERT itself. A raw query inside
//! `crates/core/src/services/` is what
//! `repos_own_the_queries.rs` exists to refuse, and it reads the whole
//! source file, test module included.

use chrono::{Duration, NaiveDateTime};
use diesel::{RunQueryDsl, SqliteConnection};
use dzpos_core::lang::Lang;
use dzpos_core::print::FactureLayout;
use dzpos_core::services::preferences::{
    facture_layout, print_lang, session_idle, set_facture_layout, set_print_lang, set_session_idle,
    set_theme, theme, Theme, DEFAULT_SESSION_IDLE_MINUTES, MAX_SESSION_IDLE_MINUTES,
};

mod common;

use common::open_temp;

/// The shop the first migration seeds, and the user who owns it.
const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// The shop next door, which exists only here.
const NEIGHBOUR: i32 = 2;

fn at() -> NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 9, 10)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap()
}

/// A second shop, written straight into the table: opening a shop is not a
/// service any caller has, and the preferences below only need the row to
/// exist for the foreign key to hold.
fn seed_second_shop(conn: &mut SqliteConnection) {
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(conn)
        .unwrap();
}

/// Every reader in `services::preferences` takes a `shop_id`, and until this
/// test nothing checked that any of them used it. Each of the four reads a
/// key the other shop has set and its own shop has not, so a reader that
/// dropped its argument would hand back the neighbour's answer. The four
/// share one shape, so they are proved together rather than one at a time:
/// `print_lang` was the one that arrived today, and the hole was the same in
/// all of them.
#[test]
fn a_shop_reads_its_own_preferences_and_never_the_shop_next_door() {
    let (_dir, mut conn) = open_temp();
    seed_second_shop(&mut conn);

    set_print_lang(&mut conn, NEIGHBOUR, Some(Lang::Ar), at()).unwrap();
    set_theme(&mut conn, NEIGHBOUR, Some(Theme::ObserveDark), at()).unwrap();
    set_facture_layout(&mut conn, NEIGHBOUR, FactureLayout::HalfSheet, at()).unwrap();
    set_session_idle(&mut conn, NEIGHBOUR, OWNER, MAX_SESSION_IDLE_MINUTES, at()).unwrap();

    assert_eq!(print_lang(&mut conn, SHOP).unwrap(), None);
    assert_eq!(theme(&mut conn, SHOP).unwrap(), None);
    assert_eq!(
        facture_layout(&mut conn, SHOP).unwrap(),
        FactureLayout::default()
    );
    assert_eq!(
        session_idle(&mut conn, SHOP).unwrap(),
        Duration::minutes(DEFAULT_SESSION_IDLE_MINUTES)
    );

    // And the neighbour still has what it set, so the four reads above are
    // answering "this shop has none" rather than "nothing is there".
    assert_eq!(print_lang(&mut conn, NEIGHBOUR).unwrap(), Some(Lang::Ar));
    assert_eq!(
        theme(&mut conn, NEIGHBOUR).unwrap(),
        Some(Theme::ObserveDark)
    );
    assert_eq!(
        facture_layout(&mut conn, NEIGHBOUR).unwrap(),
        FactureLayout::HalfSheet
    );
    assert_eq!(
        session_idle(&mut conn, NEIGHBOUR).unwrap(),
        Duration::minutes(MAX_SESSION_IDLE_MINUTES)
    );
}
