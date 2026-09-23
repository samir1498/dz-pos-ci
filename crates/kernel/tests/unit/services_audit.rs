// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// The column is on the shop's calendar, so a day is its own midnight to
/// the next, and the row written at 00:30 in Algiers falls in the day it
/// was written rather than in the one before. That row is the whole
/// reason this function exists: before `record` stamped from the shop
/// clock it was stored as 23:30 the previous day and the filter had to
/// reach back an hour to find it, while the screen printed the earlier
/// date beside it.
#[test]
fn a_shop_day_runs_from_its_own_midnight_to_the_next() {
    let day = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
    let (start, end) = day_range(day);
    assert_eq!(
        start,
        NaiveDate::from_ymd_opt(2026, 9, 11)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    );
    assert_eq!(
        end,
        NaiveDate::from_ymd_opt(2026, 9, 12)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    );

    let just_after_midnight_in_algiers = NaiveDate::from_ymd_opt(2026, 9, 11)
        .unwrap()
        .and_hms_opt(0, 30, 0)
        .unwrap();
    assert!(just_after_midnight_in_algiers >= start && just_after_midnight_in_algiers < end);
    let (prev_start, prev_end) = day_range(day.pred_opt().unwrap());
    assert!(
        !(just_after_midnight_in_algiers >= prev_start
            && just_after_midnight_in_algiers < prev_end)
    );
}
