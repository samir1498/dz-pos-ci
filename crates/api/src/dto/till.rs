//! The till's shifts on the wire (features.md §1, the cash position; plan
//! `till-shifts-a-float-and-a-count` T4).
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.
//!
//! **Neither moment is on the wire.** `NewShift::opened_at` and
//! `TillCount::at` are both `Option` in the core and the two DTOs below pass
//! `None` for each, the way `NewSaleDto` hardcodes `issued_at: None`
//! (`dto/sales.rs`: the moment a thing happened is the server's to say, on
//! the shop's calendar). A field for either would be a client that can
//! backdate a window, and a window is what the expected figure is summed
//! over: a shift opened an hour early swallows an hour of somebody else's
//! morning, and a close dated back drops the last hour of takings out of the
//! figure the cashier signs. `services::shifts` refuses a moment in the
//! future and one reaching back into that person's own last closed shift,
//! but those guards are the floor under a wrong clock rather than a licence
//! to accept the field. `deny_unknown_fields` below is what makes the
//! absence a refusal rather than a silent drop.

use super::*;

/// One shift as a screen reads it. The four close columns travel together or
/// not at all, the way `models::shift::ShiftClose` holds them, so no screen
/// has to decide for itself whether a row carrying a count and no moment is
/// open.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ShiftDto.ts")]
pub struct ShiftDto {
    pub id: i32,
    pub opened_by: i32,
    /// The shop's clock, `YYYY-MM-DD HH:MM:SS`, as the column holds it.
    pub opened_at: String,
    pub opening_cash_centimes: i64,
    /// None while the drawer is still open.
    pub closed_at: Option<String>,
    pub closed_by: Option<i32>,
    pub counted_centimes: Option<i64>,
    /// What the shop expected that person to be holding, as it stood at the
    /// close. The stored snapshot and never a fresh sum: a ticket annulled
    /// next Wednesday must not move a figure somebody signed on Monday.
    pub expected_at_close_centimes: Option<i64>,
    /// `counted - expected`, negative when the drawer was short. Computed by
    /// the core's checked subtraction and carried, rather than left to a
    /// screen to work out: two subtractions are two answers.
    pub difference_centimes: Option<i64>,
    pub note: Option<String>,
}

impl TryFrom<Shift> for ShiftDto {
    type Error = ApiError;

    fn try_from(s: Shift) -> Result<Self, ApiError> {
        let difference = match &s.close {
            Some(close) => Some(close.difference().map_err(CoreError::from)?.as_centimes()),
            None => None,
        };
        Ok(ShiftDto {
            id: s.id,
            opened_by: s.opened_by,
            opened_at: s.opened_at.format(DATE_TIME_FORMAT).to_string(),
            opening_cash_centimes: s.opening_cash.as_centimes(),
            closed_at: s
                .close
                .as_ref()
                .map(|c| c.closed_at.format(DATE_TIME_FORMAT).to_string()),
            closed_by: s.close.as_ref().map(|c| c.closed_by),
            counted_centimes: s.close.as_ref().map(|c| c.counted.as_centimes()),
            expected_at_close_centimes: s.close.as_ref().map(|c| c.expected.as_centimes()),
            difference_centimes: difference,
            note: s.note,
        })
    }
}

/// A shift with the figures a screen puts beside it: what that person took
/// over the window, what the shop expects them to hold, and the difference
/// once it is counted.
///
/// `takings` is `TakingsDto`, the same three figures the cash panel already
/// reads, about one person instead of the shop. It is read live on every
/// call and `expected_at_close_centimes` on the shift beside it is the
/// stored snapshot; where the two disagree, the live figure is what says so.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ShiftReportDto.ts")]
pub struct ShiftReportDto {
    pub shift: ShiftDto,
    pub takings: TakingsDto,
    /// Cash this person handed back over the window on a reversal, read live
    /// like `takings`. By whoever handed the notes over and never by whoever
    /// rang the sale.
    pub refunds_centimes: i64,
    /// `opening_cash` plus the takings less the refunds while the shift is
    /// open, and the stored figure once it is closed.
    pub expected_centimes: i64,
    /// None while the shift is open: nothing has been counted yet.
    pub difference_centimes: Option<i64>,
    /// The moment the takings window ends: the close, or now for an open
    /// shift, so a screen showing a live figure can say as of when.
    pub until: String,
    /// How many sales this person rang while holding no drawer at all, over
    /// the stretch from their last close before this shift, or midnight of the
    /// day it opened when they have never closed one. Zero on a shift
    /// where the drawer was open the whole time, which is why the close
    /// screen shows it only when it is not.
    ///
    /// Not a term in `expected_centimes`: those sales' cash is physically in
    /// the drawer but fell outside the window the expected figure is summed
    /// over, so this is the sentence that says why the count may read over,
    /// not a number that moves it.
    pub rung_outside_shift: i64,
}

impl TryFrom<ShiftReport> for ShiftReportDto {
    type Error = ApiError;

    fn try_from(r: ShiftReport) -> Result<Self, ApiError> {
        Ok(ShiftReportDto {
            shift: ShiftDto::try_from(r.shift)?,
            takings: TakingsDto::try_from(r.takings)?,
            refunds_centimes: r.refunds.as_centimes(),
            expected_centimes: r.expected.as_centimes(),
            difference_centimes: r.difference.map(Money::as_centimes),
            until: r.until.format(DATE_TIME_FORMAT).to_string(),
            rung_outside_shift: r.rung_outside_shift,
        })
    }
}

/// Opening a drawer: what is in it, and nothing else.
///
/// The opener is not here — it comes from the caller's identity like every
/// other write — and neither is the moment, for the reason this module's own
/// doc gives.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewShiftDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewShiftDto {
    pub opening_cash_centimes: i64,
}

impl TryFrom<NewShiftDto> for NewShift {
    type Error = ApiError;

    fn try_from(d: NewShiftDto) -> Result<Self, ApiError> {
        Ok(NewShift {
            opening_cash: Money::centimes(within_js_safe_range(
                "opening_cash_centimes",
                d.opening_cash_centimes,
            )?),
            // The server says when the drawer was taken over
            // (core, `services::clock`).
            opened_at: None,
        })
    }
}

/// Counting a drawer at close: what was in it, and why it differs if it
/// does. The expected figure is not here and never comes from a caller —
/// `services::shifts::close` works it out at the moment of closing, because
/// a caller that could supply it could record a clean evening over a short
/// drawer.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "TillCountDto.ts")]
#[serde(deny_unknown_fields)]
pub struct TillCountDto {
    pub counted_centimes: i64,
    #[serde(default)]
    pub note: Option<String>,
}

impl TryFrom<TillCountDto> for TillCount {
    type Error = ApiError;

    fn try_from(d: TillCountDto) -> Result<Self, ApiError> {
        Ok(TillCount {
            counted: Money::centimes(within_js_safe_range(
                "counted_centimes",
                d.counted_centimes,
            )?),
            note: d.note,
            // The server says when the drawer was counted, and the same
            // moment ends the window the expected figure is summed over.
            at: None,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../../tests/unit/dto_till.rs"]
mod tests;
