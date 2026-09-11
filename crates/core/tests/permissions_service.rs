// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! `services::permissions`. The table itself is the single statement of the
//! rule, so this file does not restate it row by row: it asserts the shape
//! (every permission answered for every role, the roles nest) and a handful
//! of named cases a human would check by eye, plus `require`'s typed
//! refusal and the discount-threshold helper `discount_needs_permission`.
//! What it deliberately does not do: assert `can(Owner, X) == can(Manager,
//! X)` for all eleven permissions as if that equality were the rule being
//! tested. It is today's answer, not a guarantee this table makes; the
//! "manager a superset of cashier" and "owner a superset of manager"
//! assertions below hold whether or not a future ruling splits the two.

use std::collections::HashSet;

use dzpos_core::error::CoreError;
use dzpos_core::money::{Bps, Money};
use dzpos_core::services::permissions::{
    can, discount_needs_permission, require, Permission, Role,
};

fn granted(role: Role) -> HashSet<Permission> {
    Permission::ALL
        .into_iter()
        .filter(|&p| can(role, p))
        .collect()
}

#[test]
fn every_permission_is_answered_for_every_role() {
    // `can` returns a plain `bool`, never an `Option`, so every one of the
    // three roles times eleven permissions below is either granted or
    // refused and never left unanswered; this asserts the count reaches all
    // 33 rather than trusting the type alone.
    let mut answered = 0;
    for role in Role::ALL {
        for permission in Permission::ALL {
            let first = can(role, permission);
            let second = can(role, permission);
            assert_eq!(first, second, "can must be a pure function of its inputs");
            answered += 1;
        }
    }
    assert_eq!(answered, Role::ALL.len() * Permission::ALL.len());
}

#[test]
fn owner_is_a_superset_of_manager() {
    let owner = granted(Role::Owner);
    let manager = granted(Role::Manager);
    assert!(
        manager.is_subset(&owner),
        "manager holds a permission owner does not: {:?}",
        manager.difference(&owner).collect::<Vec<_>>()
    );
}

#[test]
fn manager_is_a_superset_of_cashier() {
    let manager = granted(Role::Manager);
    let cashier = granted(Role::Cashier);
    assert!(
        cashier.is_subset(&manager),
        "cashier holds a permission manager does not: {:?}",
        cashier.difference(&manager).collect::<Vec<_>>()
    );
}

#[test]
fn the_cashier_list_is_exactly_sell() {
    // features.md §5 and the T1 brief: a cashier rings sales up. Everything
    // past that (discount above threshold, cost and margin, editing
    // anything, seeing reports, money, users, ledger corrections, export
    // and import) is a manager's or an owner's.
    let cashier: HashSet<Permission> = [Permission::Sell].into_iter().collect();
    assert_eq!(granted(Role::Cashier), cashier);
}

#[test]
fn named_cases_a_human_would_check_by_eye() {
    assert!(can(Role::Cashier, Permission::Sell));
    assert!(!can(Role::Cashier, Permission::SeeCostAndMargin));
    assert!(!can(Role::Cashier, Permission::OverrideCreditBlock));
    assert!(!can(Role::Cashier, Permission::DiscountAboveThreshold));
    assert!(!can(Role::Cashier, Permission::ManageUsers));

    assert!(can(Role::Manager, Permission::CommitMoney));
    assert!(can(Role::Manager, Permission::CorrectLedger));
    assert!(can(Role::Manager, Permission::ExportAndImport));
    assert!(can(Role::Manager, Permission::ManageUsers));

    assert!(can(Role::Owner, Permission::EditSettings));
    assert!(can(Role::Owner, Permission::ManageUsers));
}

#[test]
fn require_lets_a_granted_role_through() {
    require(Role::Owner, Permission::ManageUsers).expect("the owner may manage users");
    require(Role::Cashier, Permission::Sell).expect("the cashier may sell");
}

#[test]
fn require_refuses_a_role_without_the_permission_and_names_it() {
    let err = require(Role::Cashier, Permission::ManageUsers).unwrap_err();
    assert!(matches!(
        err,
        CoreError::Forbidden {
            permission: Permission::ManageUsers
        }
    ));
    assert_eq!(err.code(), "forbidden");
}

#[test]
fn a_discount_at_the_threshold_needs_no_permission() {
    // "A sale at or below the threshold needs no permission" (T1 brief).
    let base = Money::centimes(10_000);
    let threshold = Bps::new(500).unwrap(); // 5 %
    let discount_at_threshold = base.pct(threshold).unwrap();
    assert!(!discount_needs_permission(base, discount_at_threshold, threshold).unwrap());
}

#[test]
fn a_discount_past_the_threshold_needs_the_permission() {
    let base = Money::centimes(10_000);
    let threshold = Bps::new(500).unwrap(); // 5 %
    let one_centime_over = base
        .pct(threshold)
        .unwrap()
        .checked_add(Money::centimes(1))
        .unwrap();
    assert!(discount_needs_permission(base, one_centime_over, threshold).unwrap());
}

#[test]
fn a_zero_threshold_needs_the_permission_for_any_discount_at_all() {
    // The default a shop reads before it has ever set one
    // (`services::settings::discount_threshold_as_of`).
    let base = Money::centimes(10_000);
    assert!(!discount_needs_permission(base, Money::ZERO, Bps::ZERO).unwrap());
    assert!(discount_needs_permission(base, Money::centimes(1), Bps::ZERO).unwrap());
}

#[test]
fn as_str_and_the_wire_spelling_never_drift() {
    // `as_str` is what the wire and the UI translate; serde's `snake_case`
    // is what actually goes over JSON. Nothing else checks the two agree, so
    // a variant renamed on one side and not the other would only show up as
    // a silent mismatch on whatever screen reads it.
    for role in Role::ALL {
        assert_eq!(
            serde_json::to_value(role).unwrap(),
            serde_json::Value::String(role.as_str().to_string())
        );
    }
    for permission in Permission::ALL {
        assert_eq!(
            serde_json::to_value(permission).unwrap(),
            serde_json::Value::String(permission.as_str().to_string())
        );
    }
}
