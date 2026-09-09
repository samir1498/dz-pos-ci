//! The shop's clock. A dated setting, a document's `issued_at` and the
//! régime a document is read under are all on the shop's calendar, so the
//! offset lives here rather than once per caller.

use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};

/// Algeria is UTC+1 all year: no daylight saving. A régime dated 1 January
/// is in force at 00:30 in Algiers, when UTC still reads 31 December.
pub const SHOP_UTC_OFFSET_SECONDS: i32 = 3600;

/// `utc` read on the shop's calendar. Separate from `now` so the wall clock
/// never enters a test.
pub fn shop_time(utc: DateTime<Utc>) -> NaiveDateTime {
    match FixedOffset::east_opt(SHOP_UTC_OFFSET_SECONDS) {
        Some(offset) => utc.with_timezone(&offset).naive_local(),
        // 3600 is inside the range east_opt accepts, so this arm is never
        // taken; UTC is the honest fallback rather than a panic.
        None => utc.naive_utc(),
    }
}

/// This moment on the shop's calendar.
pub fn now() -> NaiveDateTime {
    shop_time(Utc::now())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::shop_time;
    use chrono::{NaiveDate, TimeZone, Utc};

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
}
