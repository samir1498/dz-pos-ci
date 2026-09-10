//! The shop's clock. A dated setting, a document's `issued_at` and the
//! régime a document is read under are all on the shop's calendar, so the
//! offset lives here rather than once per caller.

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, Utc};

use crate::error::CoreError;

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

/// One month on the shop's calendar. A screen asks for a month and a figure
/// is summed over one, so the pair of days it stands for is worked out here
/// rather than once per caller: February and a leap year are exactly the
/// place where two callers would disagree.
///
/// Held as its own first day rather than as a year and a number, so a month
/// that cannot exist is refused once, where it is built, and every reader
/// below has a real date to work from instead of an option to unwrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Month(NaiveDate);

impl Month {
    /// A month between 1 and 12 of a year the calendar has. Anything else is
    /// refused rather than read as January: a screen sending `0` means a bug,
    /// and a figure answered for the wrong month is one nobody would think to
    /// doubt.
    pub fn new(year: i32, month: u32) -> Result<Self, CoreError> {
        NaiveDate::from_ymd_opt(year, month, 1)
            .map(Month)
            .ok_or_else(|| {
                CoreError::validation(
                    "month",
                    "a month is written YYYY-MM, with a month between 01 and 12",
                )
            })
    }

    /// The month a day falls in. Infallible: the day is already on the
    /// calendar, so the first of its own month is too.
    pub fn of(day: NaiveDate) -> Self {
        Month(day.with_day(1).unwrap_or(day))
    }

    pub const fn first_day(&self) -> NaiveDate {
        self.0
    }

    /// The last day of the month, found as the day before the first of the
    /// next one: a table of lengths is where a leap year goes wrong.
    pub fn last_day(&self) -> NaiveDate {
        let (year, month) = match self.0.month() {
            12 => (self.0.year().saturating_add(1), 1),
            other => (self.0.year(), other.saturating_add(1)),
        };
        NaiveDate::from_ymd_opt(year, month, 1)
            .and_then(|first| first.pred_opt())
            // Only December of chrono's last year reaches this, and there is
            // no day after it to be the last one instead.
            .unwrap_or(self.0)
    }

    /// `YYYY-MM`, the shape the month travels in on the wire.
    pub fn as_text(&self) -> String {
        format!("{:04}-{:02}", self.0.year(), self.0.month())
    }
}

/// The stretch of days a figure is asked over: one day, or one month. The two
/// questions the dashboard and the expenses screen ask, held as one type so
/// every sum below takes one range rather than two overloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Day(NaiveDate),
    Month(Month),
}

impl Period {
    /// The first and the last day of the period, both included.
    pub fn days(&self) -> (NaiveDate, NaiveDate) {
        match self {
            Period::Day(day) => (*day, *day),
            Period::Month(month) => (month.first_day(), month.last_day()),
        }
    }
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
