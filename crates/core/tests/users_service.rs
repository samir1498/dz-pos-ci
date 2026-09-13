// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Who works in the shop and how they prove it (features.md §5). The rules
//! this file holds to are the ones a screen must not restate: the shape of a
//! PIN, the password minimum, the last owner, the wait after a run of wrong
//! PINs, and a deactivated user who is kept but cannot sign in.

use chrono::{Duration, NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sql_types::Text;
use dzpos_core::error::CoreError;
use dzpos_core::services::audit;
use dzpos_core::services::users::{self, NewUser, Role};

const SHOP: i32 = 1;
/// The owner the first migration seeds, and now a real row of `users`.
const OWNER: i32 = 1;

mod common;

use common::open_temp;

#[derive(QueryableByName)]
struct Stored {
    #[diesel(sql_type = Text)]
    hash: String,
}

fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(h, min, 0)
        .unwrap()
}

/// The moment every test in this file starts from. Passed in rather than read
/// off the wall clock, so the lockout is asserted and never waited out.
fn noon() -> NaiveDateTime {
    at(2026, 9, 11, 12, 0)
}

/// A cashier with a PIN, which is the shape most of the file needs.
fn a_cashier(conn: &mut SqliteConnection, name: &str, pin: &str) -> i32 {
    let user = users::create(
        conn,
        SHOP,
        OWNER,
        NewUser {
            name: name.to_string(),
            role: Role::Cashier,
        },
    )
    .unwrap();
    users::set_pin(conn, SHOP, OWNER, user.id, pin, None).unwrap();
    user.id
}

fn stored_pin_hash(conn: &mut SqliteConnection, id: i32) -> String {
    let row: Stored = diesel::sql_query(format!(
        "SELECT pin_hash AS hash FROM users WHERE id = {id}"
    ))
    .get_result(conn)
    .unwrap();
    row.hash
}

fn audit_actions(conn: &mut SqliteConnection) -> Vec<String> {
    audit::list(conn, SHOP)
        .unwrap()
        .into_iter()
        .map(|entry| entry.action)
        .collect()
}

// ---- the seeded owner ----

#[test]
fn the_seeded_owner_reads_back_as_a_user_with_no_credential_yet() {
    let (_dir, mut conn) = open_temp();
    let owner = users::get(&mut conn, SHOP, OWNER).unwrap();
    assert_eq!(owner.name, "Propriétaire");
    assert_eq!(owner.role, Role::Owner);
    assert!(owner.active);
    // The sentinel the first migration writes is not a credential, and the
    // fiche says so rather than reading as "a PIN is set".
    assert!(!owner.has_pin, "the seeded owner reads as having a PIN");
    assert!(!owner.has_password);
}

#[test]
fn the_seeded_owner_cannot_be_signed_in_as_before_a_pin_is_set() {
    let (_dir, mut conn) = open_temp();
    // '!unset' is not a hash any verifier accepts, so nothing matches it.
    for guess in ["!unset", "0000", "", "1357"] {
        assert!(matches!(
            users::verify_pin(&mut conn, SHOP, OWNER, guess, noon()),
            Err(CoreError::AuthRefused)
        ));
    }
}

// ---- hashing ----

#[test]
fn a_pin_round_trips_through_the_stored_hash() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    let signed_in = users::verify_pin(&mut conn, SHOP, id, "1357", noon()).unwrap();
    assert_eq!(signed_in.id, id);
    assert!(signed_in.has_pin);
    assert!(matches!(
        users::verify_pin(&mut conn, SHOP, id, "1358", noon()),
        Err(CoreError::AuthRefused)
    ));
}

#[test]
fn a_password_round_trips_and_is_found_by_name() {
    let (_dir, mut conn) = open_temp();
    users::set_password(&mut conn, SHOP, OWNER, OWNER, "correcte-batterie", None).unwrap();
    let signed_in =
        users::verify_password(&mut conn, SHOP, "Propriétaire", "correcte-batterie", noon())
            .unwrap();
    assert_eq!(signed_in.id, OWNER);
    assert!(signed_in.has_password);
    assert!(matches!(
        users::verify_password(&mut conn, SHOP, "Propriétaire", "correcte-batteri", noon()),
        Err(CoreError::AuthRefused)
    ));
    // A name nobody answers to is the same refusal, so a login box cannot be
    // read as a staff list.
    assert!(matches!(
        users::verify_password(&mut conn, SHOP, "Nabil", "correcte-batterie", noon()),
        Err(CoreError::AuthRefused)
    ));
}

#[test]
fn the_stored_hash_is_argon2id_with_the_parameters_this_app_chose() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    let stored = stored_pin_hash(&mut conn, id);
    // Read off the string rather than described in a comment: the parameters
    // travel inside every hash, and this is where a version bump that changed
    // them would be noticed. argon2id, version 19, 19 MiB, two passes, one
    // lane: the OWASP configuration for argon2id and the crate's own default.
    assert!(
        stored.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
        "the stored hash is not the argon2id this app chose: {stored}"
    );
    // Nothing in the stored string is the PIN.
    assert!(!stored.contains("1357"));
}

#[test]
fn two_users_with_the_same_pin_do_not_share_a_hash() {
    let (_dir, mut conn) = open_temp();
    let one = a_cashier(&mut conn, "Karim", "1357");
    let two = a_cashier(&mut conn, "Nabil", "1357");
    // A per-user salt from the OS random source, so a rainbow table over the
    // ten thousand four-digit PINs is worth nothing against the shop file.
    assert_ne!(
        stored_pin_hash(&mut conn, one),
        stored_pin_hash(&mut conn, two)
    );
}

// ---- the shape of a PIN and of a password ----

#[test]
fn a_pin_that_is_the_wrong_shape_is_refused_and_nothing_is_stored() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    let before = stored_pin_hash(&mut conn, id);
    for bad in [
        // Too short, too long.
        "123", "1234567", // A repeat.
        "1111", "000000", // A run, up and down.
        "1234", "4321", "345678", "987654", // Not digits.
        "12a4", "12 4", "١٢٣٤", "",
    ] {
        let refused = users::set_pin(&mut conn, SHOP, OWNER, id, bad, None);
        assert!(
            matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "pin"),
            "{bad} was not refused as a PIN: {refused:?}"
        );
    }
    assert_eq!(
        before,
        stored_pin_hash(&mut conn, id),
        "a refused PIN still wrote over the stored one"
    );
}

#[test]
fn a_pin_of_four_to_six_digits_that_is_neither_a_run_nor_a_repeat_is_taken() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    for good in ["1357", "90210", "428513", "1123", "0102"] {
        assert!(
            users::set_pin(&mut conn, SHOP, OWNER, id, good, None).is_ok(),
            "{good} was refused as a PIN"
        );
        users::verify_pin(&mut conn, SHOP, id, good, noon()).unwrap();
    }
}

#[test]
fn a_password_under_eight_characters_is_refused() {
    let (_dir, mut conn) = open_temp();
    for bad in ["", "court", "sept ca"] {
        let refused = users::set_password(&mut conn, SHOP, OWNER, OWNER, bad, None);
        assert!(
            matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "password"),
            "{bad:?} was not refused as a password: {refused:?}"
        );
    }
    assert!(users::set_password(&mut conn, SHOP, OWNER, OWNER, "huit car", None).is_ok());
}

// ---- the fiche ----

#[test]
fn two_users_of_one_shop_cannot_share_a_name() {
    let (_dir, mut conn) = open_temp();
    a_cashier(&mut conn, "Karim", "1357");
    let refused = users::create(
        &mut conn,
        SHOP,
        OWNER,
        NewUser {
            name: "  Karim  ".to_string(),
            role: Role::Manager,
        },
    );
    assert!(
        matches!(&refused, Err(CoreError::Conflict { field, .. }) if field == "name"),
        "{refused:?}"
    );
}

#[test]
fn a_deactivated_user_is_kept_and_refused() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    let off = users::deactivate(&mut conn, SHOP, OWNER, id).unwrap();
    assert!(!off.active);
    // Kept: the fiche still reads back, because every document and ledger row
    // they wrote names this id.
    assert_eq!(users::get(&mut conn, SHOP, id).unwrap().id, id);
    assert_eq!(users::list(&mut conn, SHOP).unwrap().len(), 2);
    assert!(matches!(
        users::verify_pin(&mut conn, SHOP, id, "1357", noon()),
        Err(CoreError::AuthRefused)
    ));
    // And back on again: nothing was destroyed, so nothing has to be rebuilt.
    users::reactivate(&mut conn, SHOP, OWNER, id).unwrap();
    assert_eq!(
        users::verify_pin(&mut conn, SHOP, id, "1357", noon())
            .unwrap()
            .id,
        id
    );
}

#[test]
fn the_last_owner_is_neither_switched_off_nor_moved_off_the_role() {
    let (_dir, mut conn) = open_temp();
    let manager = users::create(
        &mut conn,
        SHOP,
        OWNER,
        NewUser {
            name: "Nabil".to_string(),
            role: Role::Manager,
        },
    )
    .unwrap();
    // The rule lives in the service, not on the settings screen: a second
    // caller could not get it wrong even if it wanted to.
    let refused = users::deactivate(&mut conn, SHOP, manager.id, OWNER);
    assert!(
        matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "active"),
        "{refused:?}"
    );
    let refused = users::set_role(&mut conn, SHOP, manager.id, OWNER, Role::Cashier);
    assert!(
        matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "role"),
        "{refused:?}"
    );
    // With a second owner in the shop, the first one may go.
    users::set_role(&mut conn, SHOP, OWNER, manager.id, Role::Owner).unwrap();
    let off = users::deactivate(&mut conn, SHOP, manager.id, OWNER).unwrap();
    assert!(!off.active);
    // And now the second one is the last one.
    assert!(users::deactivate(&mut conn, SHOP, OWNER, manager.id).is_err());
}

#[test]
fn nobody_switches_their_own_fiche_off() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    let refused = users::deactivate(&mut conn, SHOP, id, id);
    assert!(
        matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "active"),
        "{refused:?}"
    );
}

#[test]
fn a_user_of_another_shop_is_not_found() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    // Rule 3: every query is scoped by the shop, so an id that exists in the
    // file is still not this shop's user.
    assert!(matches!(
        users::get(&mut conn, 2, id),
        Err(CoreError::NotFound { entity: "user", .. })
    ));
    assert!(matches!(
        users::verify_pin(&mut conn, 2, id, "1357", noon()),
        Err(CoreError::AuthRefused)
    ));
}

// ---- the first owner, before anybody has signed in ----

const FIRST_PASSWORD: &str = "huit caracteres";

#[test]
fn a_virgin_shop_still_needs_its_first_setup() {
    let (_dir, mut conn) = open_temp();
    assert!(users::shop_needs_first_setup(&mut conn, SHOP).unwrap());
    users::claim_first_owner(&mut conn, SHOP, "Anouar", FIRST_PASSWORD).unwrap();
    assert!(!users::shop_needs_first_setup(&mut conn, SHOP).unwrap());
}

#[test]
fn a_virgin_shop_gives_its_seeded_owner_the_first_password_and_it_signs_them_in() {
    let (_dir, mut conn) = open_temp();
    let owner = users::claim_first_owner(&mut conn, SHOP, "Anouar", FIRST_PASSWORD).unwrap();
    assert_eq!(owner.id, OWNER);
    assert_eq!(owner.name, "Anouar");
    assert!(owner.has_password);
    assert!(!owner.has_pin);
    assert!(
        users::verify_password(&mut conn, SHOP, "Anouar", FIRST_PASSWORD, noon()).is_ok(),
        "the password this call set does not sign the owner in"
    );
    assert_eq!(audit_actions(&mut conn), vec!["user.claim_first_owner"]);
}

#[test]
fn a_password_the_shape_rule_refuses_is_refused_here_too_and_nothing_is_claimed() {
    let (_dir, mut conn) = open_temp();
    assert!(users::claim_first_owner(&mut conn, SHOP, "Anouar", "short").is_err());
    assert!(!users::get(&mut conn, SHOP, OWNER).unwrap().has_password);
}

#[test]
fn the_door_shuts_for_good_once_any_credential_in_the_shop_exists() {
    let (_dir, mut conn) = open_temp();
    users::claim_first_owner(&mut conn, SHOP, "Anouar", FIRST_PASSWORD).unwrap();
    let refused = users::claim_first_owner(&mut conn, SHOP, "Samir", "autre mot de passe");
    assert!(
        matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "password"),
        "{refused:?}"
    );
    assert!(users::verify_password(&mut conn, SHOP, "Anouar", FIRST_PASSWORD, noon()).is_ok());
    assert_eq!(audit_actions(&mut conn), vec!["user.claim_first_owner"]);
}

#[test]
fn a_pin_set_the_ordinary_way_shuts_the_door_too() {
    let (_dir, mut conn) = open_temp();
    users::set_pin(&mut conn, SHOP, OWNER, OWNER, "2580", None).unwrap();
    let refused = users::claim_first_owner(&mut conn, SHOP, "Anouar", FIRST_PASSWORD);
    assert!(
        matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "password"),
        "{refused:?}"
    );
}

#[test]
fn more_than_one_owner_with_no_credential_yet_is_refused_rather_than_guessed() {
    let (_dir, mut conn) = open_temp();
    let manager = users::create(
        &mut conn,
        SHOP,
        OWNER,
        NewUser {
            name: "Nabil".to_string(),
            role: Role::Manager,
        },
    )
    .unwrap();
    users::set_role(&mut conn, SHOP, OWNER, manager.id, Role::Owner).unwrap();
    // Two active owners, and neither has a credential yet: nothing here says
    // which one this password was meant for.
    let refused = users::claim_first_owner(&mut conn, SHOP, "Anouar", FIRST_PASSWORD);
    assert!(
        matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "name"),
        "{refused:?}"
    );
}

#[test]
fn a_shop_with_no_active_owner_at_all_has_nobody_to_give_the_password_to() {
    let (_dir, mut conn) = open_temp();
    diesel::sql_query("UPDATE users SET role = 'cashier' WHERE id = ?")
        .bind::<diesel::sql_types::Integer, _>(OWNER)
        .execute(&mut conn)
        .unwrap();
    let refused = users::claim_first_owner(&mut conn, SHOP, "Anouar", FIRST_PASSWORD);
    assert!(
        matches!(&refused, Err(CoreError::Validation { field, .. }) if field == "name"),
        "{refused:?}"
    );
}

// ---- the lockout ----

#[test]
fn five_wrong_pins_make_the_user_wait_and_the_right_one_is_refused_while_they_do() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    for _ in 0..5 {
        assert!(matches!(
            users::verify_pin(&mut conn, SHOP, id, "9999", noon()),
            Err(CoreError::AuthRefused)
        ));
    }
    // The right PIN now, and the counter refuses it: this is the whole point
    // of the rule, and it is why the counter cannot be written inside the
    // transaction the refusal rolls back.
    let waiting = users::verify_pin(&mut conn, SHOP, id, "1357", noon());
    assert!(
        matches!(
            waiting,
            Err(CoreError::LockedOut {
                retry_after_seconds: 30
            })
        ),
        "{waiting:?}"
    );
}

#[test]
fn the_wait_grows_with_every_further_wrong_pin_and_stops_at_a_quarter_of_an_hour() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    // How long the right PIN is refused for, read without ever letting it
    // through: a correct PIN under a lockout is answered by the counter and
    // neither clears it nor adds to it.
    let wait_now =
        |conn: &mut SqliteConnection, now| match users::verify_pin(conn, SHOP, id, "1357", now) {
            Err(CoreError::LockedOut {
                retry_after_seconds,
            }) => retry_after_seconds,
            other => panic!("the user is not locked out: {other:?}"),
        };
    let mut seen = Vec::new();
    let mut now = noon();
    for _ in 0..5 {
        assert!(matches!(
            users::verify_pin(&mut conn, SHOP, id, "9999", now),
            Err(CoreError::AuthRefused)
        ));
    }
    seen.push(wait_now(&mut conn, now));
    for step in 1..=7 {
        // Each further attempt is made after the previous wait has run out,
        // which is what a patient stranger at the counter would do.
        now = noon() + Duration::hours(step);
        assert!(matches!(
            users::verify_pin(&mut conn, SHOP, id, "9999", now),
            Err(CoreError::AuthRefused)
        ));
        seen.push(wait_now(&mut conn, now));
    }
    assert_eq!(
        seen,
        vec![30, 60, 120, 240, 480, 900, 900, 900],
        "the wait after the fifth wrong PIN is not the doubling this shop chose"
    );
}

#[test]
fn a_correct_pin_after_the_wait_works_and_clears_the_count() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    for _ in 0..5 {
        let _ = users::verify_pin(&mut conn, SHOP, id, "9999", noon());
    }
    let after = noon() + Duration::seconds(31);
    assert_eq!(
        users::verify_pin(&mut conn, SHOP, id, "1357", after)
            .unwrap()
            .id,
        id
    );
    // Cleared: the next mistake starts the allowance again rather than
    // locking the till on the first fat finger of the afternoon.
    for _ in 0..4 {
        assert!(matches!(
            users::verify_pin(&mut conn, SHOP, id, "9999", after),
            Err(CoreError::AuthRefused)
        ));
    }
    assert_eq!(
        users::verify_pin(&mut conn, SHOP, id, "1357", after)
            .unwrap()
            .id,
        id
    );
}

#[test]
fn guessing_at_a_locked_user_does_not_extend_the_wait() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    for _ in 0..5 {
        let _ = users::verify_pin(&mut conn, SHOP, id, "9999", noon());
    }
    // Anybody at the counter could otherwise hold the owner out of their own
    // till for as long as they cared to keep typing.
    for _ in 0..20 {
        assert!(matches!(
            users::verify_pin(&mut conn, SHOP, id, "9999", noon()),
            Err(CoreError::LockedOut { .. })
        ));
    }
    let after = noon() + Duration::seconds(31);
    assert_eq!(
        users::verify_pin(&mut conn, SHOP, id, "1357", after)
            .unwrap()
            .id,
        id
    );
}

#[test]
fn an_owner_resetting_the_pin_lets_a_locked_out_cashier_back_in() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    for _ in 0..5 {
        let _ = users::verify_pin(&mut conn, SHOP, id, "9999", noon());
    }
    users::set_pin(&mut conn, SHOP, OWNER, id, "2468", None).unwrap();
    // The one thing somebody in the shop can actually do about a lockout.
    assert_eq!(
        users::verify_pin(&mut conn, SHOP, id, "2468", noon())
            .unwrap()
            .id,
        id
    );
}

// ---- the audit log ----

#[test]
fn every_mutating_call_writes_one_audit_row_naming_the_actor_and_the_target() {
    let (_dir, mut conn) = open_temp();
    let created = users::create(
        &mut conn,
        SHOP,
        OWNER,
        NewUser {
            name: "Karim".to_string(),
            role: Role::Cashier,
        },
    )
    .unwrap();
    users::set_pin(&mut conn, SHOP, OWNER, created.id, "1357", None).unwrap();
    users::set_password(&mut conn, SHOP, OWNER, created.id, "huit car", None).unwrap();
    users::rename(&mut conn, SHOP, OWNER, created.id, "Karim B.").unwrap();
    users::set_role(&mut conn, SHOP, OWNER, created.id, Role::Manager).unwrap();
    users::deactivate(&mut conn, SHOP, OWNER, created.id).unwrap();
    users::reactivate(&mut conn, SHOP, OWNER, created.id).unwrap();

    assert_eq!(
        audit_actions(&mut conn),
        vec![
            "user.create",
            "user.set_pin",
            "user.set_password",
            "user.rename",
            "user.set_role",
            "user.deactivate",
            "user.reactivate",
        ]
    );
    let entries = audit::list(&mut conn, SHOP).unwrap();
    for entry in &entries {
        assert_eq!(entry.user_id, OWNER, "{} named another actor", entry.action);
        assert_eq!(entry.entity, "user");
        assert_eq!(entry.entity_id, Some(created.id));
    }
    // The log says a credential was set; it never says what it is.
    let written = entries
        .iter()
        .filter_map(|e| e.after.as_deref())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(!written.contains("argon2"), "the audit log holds a hash");
    assert!(!written.contains("1357"), "the audit log holds a PIN");
    assert!(written.contains("\"has_pin\":true"));
}

#[test]
fn a_lockout_leaves_a_row_in_the_log_and_the_wrong_pins_before_it_do_not() {
    let (_dir, mut conn) = open_temp();
    let id = a_cashier(&mut conn, "Karim", "1357");
    for _ in 0..4 {
        let _ = users::verify_pin(&mut conn, SHOP, id, "9999", noon());
    }
    assert_eq!(
        audit_actions(&mut conn)
            .iter()
            .filter(|a| *a == "user.locked_out")
            .count(),
        0,
        "a mistyped digit wrote a row"
    );
    let _ = users::verify_pin(&mut conn, SHOP, id, "9999", noon());
    // The refusal returns an error and the row survives it: a counter and a
    // log row written inside the transaction the refusal unwinds would both
    // roll back, which is the M2 credit-block finding on another table.
    let locked: Vec<String> = audit_actions(&mut conn)
        .into_iter()
        .filter(|a| a == "user.locked_out")
        .collect();
    assert_eq!(locked.len(), 1, "the crossing wrote {} rows", locked.len());
    // And not once per further attempt.
    for _ in 0..3 {
        let _ = users::verify_pin(&mut conn, SHOP, id, "9999", noon());
    }
    assert_eq!(
        audit_actions(&mut conn)
            .iter()
            .filter(|a| *a == "user.locked_out")
            .count(),
        1
    );
}
