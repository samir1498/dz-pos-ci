//! A stretch at the till: who opened the drawer, with what in it, and what
//! was counted when it closed (features.md §1, the cash position).
//!
//! A shift answers whose count is short. It does not answer whose sales,
//! which `user_id` on the document already answers, and it does not answer
//! what the shop took, which the derived cash position already answers.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::money::{Money, MoneyError};
use crate::schema::shifts;

/// The close half of a shift, all four columns together. The migration's
/// `shifts_close_is_whole` CHECK says the file holds them that way; this type
/// says the same thing in Rust, so no reader downstream has to decide for
/// itself whether a row carrying a counted figure and no moment is open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftClose {
    pub closed_at: NaiveDateTime,
    /// Whoever closed it, which is the opener on every ordinary evening and
    /// somebody else when a cashier walked away from an open drawer.
    pub closed_by: i32,
    /// What was counted in the drawer.
    pub counted: Money,
    /// What the shop expected that person to hold, as it stood at the close.
    /// Stored rather than derived: a ticket annulled next Wednesday must not
    /// move a figure somebody signed on Monday.
    pub expected: Money,
}

impl ShiftClose {
    /// `counted - expected`: negative when the drawer is short, positive when
    /// it is over. Checked, so a figure at the end of the range is an error
    /// and never a wrapped one.
    pub fn difference(&self) -> Result<Money, MoneyError> {
        self.counted.checked_sub(self.expected)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shift {
    pub id: i32,
    pub shop_id: i32,
    pub opened_by: i32,
    /// The shop's clock (`services::clock`), like a document's `issued_at`.
    pub opened_at: NaiveDateTime,
    pub opening_cash: Money,
    /// None while the shift is open.
    pub close: Option<ShiftClose>,
    /// Why the two figures differ, and whatever else the closer wanted to
    /// say. The file refuses a difference that carries none.
    pub note: Option<String>,
}

impl Shift {
    pub const fn is_open(&self) -> bool {
        self.close.is_none()
    }
}

/// A shift as a caller asks for it to be opened. The opener is not here; it
/// comes from the caller's identity, the way it does on every other write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewShift {
    /// The moment the drawer was taken over. `None` is now, on the shop's
    /// clock, which is what a till opening at the counter wants and what the
    /// route passes.
    ///
    /// Optional and not required, the same shape `NewSale::issued_at` takes
    /// (`services::sales::issue` defaults it from `clock::now`, and the DTO
    /// hardcodes `None`). A required field would make every caller name a
    /// moment, which is the primitive this type exists to keep off the wire:
    /// when a shift began is the server's to say.
    pub opened_at: Option<NaiveDateTime>,
    pub opening_cash: Money,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = shifts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct ShiftRow {
    pub id: i32,
    pub shop_id: i32,
    pub opened_by: i32,
    pub opened_at: NaiveDateTime,
    pub opening_cash_centimes: i64,
    pub closed_at: Option<NaiveDateTime>,
    pub closed_by: Option<i32>,
    pub counted_centimes: Option<i64>,
    pub expected_at_close_centimes: Option<i64>,
    pub note: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = shifts)]
pub(crate) struct ShiftRowWrite {
    pub shop_id: i32,
    pub opened_by: i32,
    pub opened_at: NaiveDateTime,
    pub opening_cash_centimes: i64,
}

/// The four close columns and the note, written in one UPDATE. The note is on
/// this struct and not on `ShiftClose` because the column is the row's and
/// not the close's: the file lets an open shift carry one.
///
/// `treat_none_as_null` and not diesel's default, which skips a `None` field
/// and leaves the column as it was. The file lets an open shift carry a note,
/// so the default would close a short drawer against whatever was written
/// when it opened and satisfy `shifts_a_difference_carries_a_reason` with a
/// sentence that is not the reason. A close says what it says, and a clean
/// one says nothing.
#[derive(Debug, AsChangeset)]
#[diesel(table_name = shifts, treat_none_as_null = true)]
pub(crate) struct ShiftCloseWrite {
    pub closed_at: NaiveDateTime,
    pub closed_by: i32,
    pub counted_centimes: i64,
    pub expected_at_close_centimes: i64,
    pub note: Option<String>,
}

impl From<ShiftRow> for Shift {
    fn from(r: ShiftRow) -> Self {
        // All four or none: the CHECK in migration 000017 is what makes the
        // `else` arm unreachable for a row this app wrote, and matching on
        // all four rather than on `closed_at` alone means a row written past
        // the file by hand reads as open instead of half closed.
        let close = match (
            r.closed_at,
            r.closed_by,
            r.counted_centimes,
            r.expected_at_close_centimes,
        ) {
            (Some(closed_at), Some(closed_by), Some(counted), Some(expected)) => Some(ShiftClose {
                closed_at,
                closed_by,
                counted: Money::centimes(counted),
                expected: Money::centimes(expected),
            }),
            _ => None,
        };
        Shift {
            id: r.id,
            shop_id: r.shop_id,
            opened_by: r.opened_by,
            opened_at: r.opened_at,
            opening_cash: Money::centimes(r.opening_cash_centimes),
            close,
            note: r.note,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../../tests/unit/models_shift.rs"]
mod tests;
