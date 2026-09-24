// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::money::MoneyError;
use chrono::NaiveDate;

/// An order carrying nothing but the two cost columns; every other field
/// is beside the point here.
fn order(transport: i64, extra_costs: i64) -> Purchase {
    Purchase {
        id: 1,
        shop_id: 1,
        supplier_id: 3,
        supplier_document_number: None,
        purchase_date: "2026-09-10".to_string(),
        due_date: None,
        transport: Money::centimes(transport),
        extra_costs: Money::centimes(extra_costs),
        status: PurchaseStatus::Ordered,
        user_id: 1,
        note: None,
        created_at: NaiveDate::from_ymd_opt(2026, 9, 10)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap(),
        series_year: 2026,
        number: 1,
    }
}

/// 500,00 of transport and 125,00 of other costs is 625,00, written out
/// here rather than added by the same expression under test.
#[test]
fn the_extras_are_the_transport_and_the_other_costs_together() {
    assert_eq!(
        order(50_000, 12_500).extras().unwrap(),
        Money::centimes(62_500)
    );
}

/// An order that cost nothing to bring in answers zero, not nothing: the
/// screen prints a figure on every row.
#[test]
fn an_order_with_neither_cost_answers_zero() {
    assert_eq!(order(0, 0).extras().unwrap(), Money::ZERO);
}

/// Transport alone is the common shape, and the empty column must not
/// take the other one with it.
#[test]
fn transport_alone_is_the_whole_answer() {
    assert_eq!(order(50_000, 0).extras().unwrap(), Money::centimes(50_000));
    assert_eq!(order(0, 12_500).extras().unwrap(), Money::centimes(12_500));
}

/// Both columns are `INTEGER >= 0` and nothing caps them, so a pair at
/// the top of the range is a sum that does not fit. It is an error, not a
/// wrap: a negative total printed on a screen is worse than no answer.
#[test]
fn a_pair_at_the_top_of_the_range_is_an_error_and_never_a_wrap() {
    let err = order(i64::MAX, 1).extras().unwrap_err();
    assert!(
        matches!(err, CoreError::Money(MoneyError::Overflow)),
        "expected an overflow, got {err:?}"
    );
}
