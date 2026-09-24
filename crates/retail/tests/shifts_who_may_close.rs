// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Who may count a drawer (plan `till-shifts-a-float-and-a-count`, ruling 10:
//! "a cashier opens and closes their own shift; anyone else's is manager and
//! owner").
//!
//! Its own suite and not a corner of `shifts_service.rs`, which asks a
//! different question: what a drawer is expected to hold. The two share their
//! fixtures through `common::shifts` and nothing else, so a figure bent to
//! make one of them pass does not quietly satisfy the other.
//!
//! The check lives inside `services::shifts::close` rather than on the route,
//! because one route counts every drawer and whose drawer it is is a fact
//! about the row: `crates/api/src/gates/table.rs` gates the close route on
//! `OpenAndCloseTill`, which all three roles hold, and the refusal below is
//! the only thing standing between a cashier and every open till in the shop
//! — including the expected figure and the difference in the answer, the two
//! figures `GET /till/shifts/{id}` refuses a cashier under `SeeReports`.

use dzpos_kernel::services::audit;
use dzpos_retail::audit_actions;
use dzpos_retail::error::CoreError;
use dzpos_retail::money::Money;
use dzpos_retail::services::shifts::{self, NewShift, TillCount};

mod common;

use common::open_temp;
use common::shifts::{a_floor_manager, a_second_cashier, at, AMINA, KARIM, LEILA};

const SHOP: i32 = 1;

/// A drawer opened at nine with `opening_cash` in it.
fn opened_at_nine(opening_cash: i64) -> NewShift {
    NewShift {
        opened_at: Some(at(9, 0, 0)),
        opening_cash: Money::centimes(opening_cash),
    }
}

#[test]
fn a_cashier_counts_their_own_drawer_and_is_refused_anybody_elses() {
    // Ruling 10: "a cashier opens and closes their own shift; anyone else's
    // is manager and owner". The route cannot make this call — one route
    // serves both cases and whose drawer it is is a fact about the row — so
    // `close` makes it, and this is where it is proved.
    //
    // Karim is the cashier and Amina is the seeded owner. Both drawers are
    // opened and both are counted, because the refusal on its own is
    // satisfied by a `close` that refuses every closer who is not the opener,
    // which would stop a manager counting a till somebody walked away from —
    // the case the permission exists for.
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    let aminas = shifts::open(&mut conn, SHOP, AMINA, opened_at_nine(1_000)).unwrap();
    let karims = shifts::open(&mut conn, SHOP, KARIM, opened_at_nine(2_000)).unwrap();

    let count = |counted| TillCount {
        counted: Money::centimes(counted),
        note: Some("fin de service".to_string()),
        at: Some(at(19, 0, 0)),
    };

    // The cashier reaching for the owner's drawer, by its id.
    let refused = shifts::close(&mut conn, SHOP, aminas.id, KARIM, count(1_000)).unwrap_err();
    match &refused {
        // The string and not the variant: it is what travels on the wire and
        // what a screen puts in front of the person who was refused.
        CoreError::Forbidden { permission } => {
            assert_eq!(
                permission.as_str(),
                "close_another_persons_till",
                "{refused}"
            )
        }
        other => panic!("a cashier closed the owner's drawer: {other}"),
    }
    // And nothing was written on the way to the refusal: the row is still
    // open, so a second refusal is still possible and no figure was stored.
    assert!(shifts::open_for(&mut conn, SHOP, AMINA)
        .unwrap()
        .is_some_and(|s| s.id == aminas.id));

    // Their own, which is the whole of what a cashier may do here.
    let own = shifts::close(&mut conn, SHOP, karims.id, KARIM, count(2_000)).unwrap();
    assert_eq!(own.close.unwrap().closed_by, KARIM);

    // And the owner counting the cashier's would have been allowed, which is
    // the other side of the same rule. Karim's is closed now, so Amina's own
    // drawer, closed by Amina, is what is left to prove the ordinary evening
    // still goes through the new branch untouched.
    let mine = shifts::close(&mut conn, SHOP, aminas.id, AMINA, count(1_000)).unwrap();
    assert_eq!(mine.close.unwrap().closed_by, AMINA);
}

#[test]
fn a_manager_counts_a_drawer_its_cashier_walked_away_from() {
    // The reason `CloseAnotherPersonsTill` exists rather than a flat "your
    // own only" rule. A test that only proved the refusal above would pass
    // against a `close` that refuses everybody, and a shop whose cashier went
    // home at six with the drawer open would have no way to shut it.
    let (_dir, mut conn) = open_temp();
    a_second_cashier(&mut conn);
    a_floor_manager(&mut conn);
    let karims = shifts::open(&mut conn, SHOP, KARIM, opened_at_nine(2_000)).unwrap();
    let closed = shifts::close(
        &mut conn,
        SHOP,
        karims.id,
        LEILA,
        TillCount {
            counted: Money::centimes(1_500),
            note: Some("caissier parti, tiroir compté par la responsable".to_string()),
            at: Some(at(19, 0, 0)),
        },
    )
    .unwrap();
    let close = closed.close.unwrap();
    assert_eq!(close.closed_by, LEILA);
    assert_eq!(closed.opened_by, KARIM);
    // 1 500 counted against 2 000 expected, by hand: the opening cash and
    // nothing sold. Short by 500, and short reads negative.
    assert_eq!(close.expected, Money::centimes(2_000));
    assert_eq!(close.difference().unwrap(), Money::centimes(-500));

    // The audit row names both people, which is the only place the shop
    // learns that the counter was not the opener.
    let row = audit::list(&mut conn, SHOP)
        .unwrap()
        .into_iter()
        .find(|e| e.action == audit_actions::ACTION_CLOSE_TILL)
        .unwrap();
    assert_eq!(row.user_id, LEILA);
    let after: serde_json::Value = serde_json::from_str(&row.after.unwrap()).unwrap();
    assert_eq!(after["opened_by"], KARIM);
}
