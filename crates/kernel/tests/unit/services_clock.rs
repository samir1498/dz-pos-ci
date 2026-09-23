use super::{shop_time, Month};
use chrono::{NaiveDate, TimeZone, Utc};

#[test]
fn a_leap_february_ends_on_the_twenty_ninth_and_an_ordinary_one_on_the_twenty_eighth() {
    // The reason the last day is found as the day before the first of the
    // next month: a table of lengths is where a leap year goes wrong, and
    // 2028 is one while 2027 is not.
    assert_eq!(
        Month::new(2028, 2).ok().map(|m| m.last_day()),
        NaiveDate::from_ymd_opt(2028, 2, 29)
    );
    assert_eq!(
        Month::new(2027, 2).ok().map(|m| m.last_day()),
        NaiveDate::from_ymd_opt(2027, 2, 28)
    );
}

#[test]
fn december_ends_on_the_thirty_first_and_rolls_into_the_next_year() {
    // The one month whose next one is in another year, which is the arm
    // that would silently answer 30 November if the year did not carry.
    let december = Month::new(2026, 12).unwrap();
    assert_eq!(
        december.first_day(),
        NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()
    );
    assert_eq!(
        december.last_day(),
        NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
    );
    assert_eq!(december.as_text(), "2026-12");
}

#[test]
fn the_first_hour_of_the_algerian_day_is_already_the_new_day() {
    let local = Utc
        .with_ymd_and_hms(2026, 12, 31, 23, 30, 0)
        .single()
        .map(shop_time);
    assert_eq!(local.map(|l| l.date()), NaiveDate::from_ymd_opt(2027, 1, 1));
    assert_eq!(
        local.map(|l| l.format("%H:%M").to_string()),
        Some("00:30".to_string())
    );
}

#[test]
fn the_rest_of_the_day_reads_the_same_date_as_utc() {
    let local = Utc
        .with_ymd_and_hms(2026, 6, 15, 12, 0, 0)
        .single()
        .map(shop_time);
    assert_eq!(
        local.map(|l| l.date()),
        NaiveDate::from_ymd_opt(2026, 6, 15)
    );
}
