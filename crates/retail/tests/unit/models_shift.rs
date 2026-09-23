use super::{Shift, ShiftClose, ShiftRow};
use crate::money::Money;
use chrono::NaiveDate;

fn moment(day: u32, hour: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 9, day)
        .and_then(|d| d.and_hms_opt(hour, 0, 0))
        .unwrap()
}

fn row() -> ShiftRow {
    ShiftRow {
        id: 1,
        shop_id: 1,
        opened_by: 1,
        opened_at: moment(21, 9),
        opening_cash_centimes: 500_000,
        closed_at: None,
        closed_by: None,
        counted_centimes: None,
        expected_at_close_centimes: None,
        note: None,
    }
}

#[test]
fn a_short_drawer_reads_negative_and_an_over_one_positive() {
    // The sign is the whole answer the screen shows, so it is pinned by
    // hand here rather than read back off the subtraction.
    let short = ShiftClose {
        closed_at: moment(21, 19),
        closed_by: 1,
        counted: Money::centimes(1_180_000),
        expected: Money::centimes(1_200_000),
    };
    assert_eq!(short.difference().ok(), Some(Money::centimes(-20_000)));
    let over = ShiftClose {
        counted: Money::centimes(1_230_000),
        ..short
    };
    assert_eq!(over.difference().ok(), Some(Money::centimes(30_000)));
    let exact = ShiftClose {
        counted: Money::centimes(1_200_000),
        ..short
    };
    assert_eq!(exact.difference().ok(), Some(Money::ZERO));
}

#[test]
fn a_difference_at_the_end_of_the_range_is_an_error_and_never_a_wrapped_figure() {
    let absurd = ShiftClose {
        closed_at: moment(21, 19),
        closed_by: 1,
        counted: Money::centimes(i64::MAX),
        expected: Money::centimes(-1),
    };
    assert!(absurd.difference().is_err());
}

#[test]
fn a_row_is_open_until_all_four_close_columns_are_there() {
    assert!(Shift::from(row()).is_open());
    // Three of the four is a row the CHECK refuses, so it can only come
    // from a hand written UPDATE. It reads as open, not as half closed.
    let half = ShiftRow {
        closed_at: Some(moment(21, 19)),
        closed_by: Some(1),
        counted_centimes: Some(1_000),
        ..row()
    };
    assert!(Shift::from(half).is_open());
    let closed = ShiftRow {
        closed_at: Some(moment(21, 19)),
        closed_by: Some(2),
        counted_centimes: Some(1_000),
        expected_at_close_centimes: Some(1_000),
        ..row()
    };
    let read = Shift::from(closed);
    assert!(!read.is_open());
    assert_eq!(read.close.map(|c| c.closed_by), Some(2));
}
