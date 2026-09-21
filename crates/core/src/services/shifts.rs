//! Till shifts (features.md §1, the cash position). A cashier opens the till
//! with what is in the drawer, sells all day, and at close types what is in
//! the drawer again; this module is what turns the two figures into an answer.
//!
//! The word is shift and never session: `services::sessions` is the auth
//! session, and a second meaning on that word in the same crate costs a reader
//! every time.
//!
//! **What a shift answers, and what it must not.** A shift answers whose count
//! is short. It does not answer whose sales, which `user_id` on the document
//! already answers, and it does not answer what the shop took today, which
//! `cash::position` already answers. Nothing here enters that figure, which is
//! what keeps a 15 000 DA handover from reading as the shop losing 3 000 on a
//! day it took 12 000.
//!
//! **What the expected figure is made of.**
//!
//! ```text
//! opening_cash + that user's cash sales + that user's cash debt payments
//! ```
//!
//! and nothing else. Every term is checked. Card sales never reach a drawer;
//! expenses and supplier payments are the shop's money going out rather than
//! this drawer's, and both are `commit_money` work a cashier does not hold;
//! and another cashier's takings are another cashier's, because two people
//! hold overlapping shifts on purpose and each drawer is physically their own.
//!
//! **Which moment.** `issued_at` on a document and `created_at` on a debt
//! movement, both stamped by a service from the shop's clock. Never
//! `documents.created_at`, which takes SQLite's `CURRENT_TIMESTAMP` and is
//! therefore UTC while Algiers is an hour ahead: a sale placed off that column
//! would fall on the wrong side of every shift boundary for one hour a day.
//!
//! **The window** is half open, `opened_at <= the moment < closed_at`, and an
//! open shift has no upper bound. A sale rung at the second the drawer opened
//! is in it; one rung at the second it closed is not.
//!
//! **A sale with no shift open** is accepted and tagged, never refused. The
//! tag is derived and stored nowhere: a sale belongs to no shift when its
//! `issued_at` falls outside every window of that `user_id`. A phone whose
//! queue replays after its cashier closed lands here too, because `issued_at`
//! is the server's to say and it says the moment the sale arrived.

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::shift::{ShiftCloseWrite, ShiftRowWrite};
use crate::money::{Money, PaymentMode};
use crate::repos::shifts as repo;
use crate::services::audit;
use crate::services::cash::{self, Takings};
use crate::services::clock;
use crate::services::documents;
use crate::services::optional_field;

pub use crate::models::document::RungSale;
pub use crate::models::shift::{NewShift, Shift, ShiftClose};

/// What the closer typed and why. The expected figure is not on it and never
/// comes from a caller: `close` works it out itself at the moment of closing,
/// because a caller that could supply it could erase a short drawer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TillCount {
    /// What was counted in the drawer.
    pub counted: Money,
    /// Why the two figures differ, and whatever else the closer wanted to say.
    /// Refused below when the figures differ and this is empty.
    pub note: Option<String>,
    /// The moment the drawer was counted. `None` is now, on the shop's clock,
    /// which is what every till closing at the counter wants; the same moment
    /// ends the window the expected figure is read over and is what the column
    /// stores, so no sale can fall between the sum and the row.
    pub at: Option<NaiveDateTime>,
}

/// A shift with the figures a screen puts beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftReport {
    pub shift: Shift,
    /// The cash this person took over the window, read off the ledgers now.
    pub takings: Takings,
    /// What the shop expects them to be holding: `opening_cash` plus the
    /// takings while the shift is open, and the figure stored at the close
    /// once it is closed.
    ///
    /// The two are not the same read and are not meant to be. A ticket
    /// annulled on Wednesday drops out of Monday's takings, so a derived
    /// expected figure would move a count somebody signed on Monday; the
    /// stored one is what the closer saw. Where they disagree, `takings`
    /// beside it is what says so.
    pub expected: Money,
    /// `counted - expected`, negative when the drawer was short. None while
    /// the shift is open: nothing has been counted yet.
    pub difference: Option<Money>,
    /// The moment the takings window ends: the close, or now for an open
    /// shift. On the report so a screen showing a live figure can say as of
    /// when.
    pub until: NaiveDateTime,
}

/// This person's sales that fell inside none of their own shifts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideShifts {
    /// Oldest first.
    pub sales: Vec<RungSale>,
    /// The cash among them, checked. The close screen shows it beside the
    /// expected figure: a morning rung up before the drawer was opened is
    /// money physically in it, and without this figure it reads as the count
    /// being over by an amount nobody can account for.
    pub cash: Money,
}

/// Opens a drawer with what is in it.
///
/// The file refuses a second open shift for the same person (a partial unique
/// index on `(shop_id, opened_by)`), and the repo turns that into a `Conflict`
/// with a field on it. Per person and not per shop, because shifts of
/// different people overlap on purpose.
///
/// Two further refusals the file cannot make, because its index only looks at
/// rows with no `closed_at`:
///
/// - A drawer opened in the future. Its window would keep taking in sales
///   nobody has rung yet, so the expected figure would move after it was
///   signed.
/// - A drawer opened at or before the moment this person's last one was
///   counted. Two closed windows of one person that overlap each sum the sale
///   in the overlap into their own stored expected figure, and the cashier
///   held that money once. One guard closes it, and it has to be here:
///   nothing about a row carrying a `closed_at` is held by the index.
pub fn open(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewShift,
) -> Result<Shift, CoreError> {
    // The migration's CHECK is the backstop; what an API needs back is a
    // typed refusal naming the field, not a diesel error surfacing as a 500.
    if new.opening_cash.as_centimes() < 0 {
        return Err(CoreError::validation(
            "opening_cash_centimes",
            "a drawer opens with nothing in it or with money in it, never with less",
        ));
    }
    let opened_at = new.opened_at.unwrap_or_else(clock::now);
    if opened_at > clock::now() {
        return Err(CoreError::validation(
            "opened_at",
            "a till cannot be opened later than now",
        ));
    }
    conn.transaction(|conn| {
        if let Some(last) = repo::last_close_for(conn, shop_id, user_id)? {
            if opened_at <= last {
                return Err(CoreError::validation(
                    "opened_at",
                    &format!(
                        "that person's last till was counted at {}, and a new \
                         one starts after that",
                        last.format("%Y-%m-%d %H:%M")
                    ),
                ));
            }
        }
        let made = repo::insert(
            conn,
            &ShiftRowWrite {
                shop_id,
                opened_by: user_id,
                opened_at,
                opening_cash_centimes: new.opening_cash.as_centimes(),
            },
        )?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_OPEN_TILL,
                entity: "shift",
                entity_id: Some(made.id),
                before: None,
                after: Some(
                    serde_json::json!({
                        "opened_at": made.opened_at,
                        "opening_cash_centimes": made.opening_cash.as_centimes(),
                    })
                    .to_string(),
                ),
            },
        )?;
        Ok(made)
    })
}

/// The one open shift this person holds, or None.
pub fn open_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<Option<Shift>, CoreError> {
    repo::open_for(conn, shop_id, user_id)
}

/// Counts the drawer and closes it.
///
/// The expected figure is worked out here and never accepted from the caller.
/// A close that took it on trust would let whoever calls it hand in the figure
/// they counted and record a clean evening over a short drawer, which is the
/// one thing this feature exists to notice.
///
/// It is read over `[opened_at, closed_at)` with the same `closed_at` the row
/// stores, so a sale rung in the second between the sum and the UPDATE is on
/// one side of the line or the other and never on both.
///
/// A difference with no note is refused before the UPDATE runs. The
/// migration's `shifts_a_difference_carries_a_reason` CHECK is the backstop
/// and would refuse the row too, as a diesel error an API has no field to hang
/// on an input; this is the same rule with a sentence a screen can print.
pub fn close(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    closed_by: i32,
    count: TillCount,
) -> Result<Shift, CoreError> {
    if count.counted.as_centimes() < 0 {
        return Err(CoreError::validation(
            "counted_centimes",
            "a drawer holds nothing or money, never less than nothing",
        ));
    }
    let note = optional_field("note", count.note.as_deref())?;
    conn.transaction(|conn| {
        let shift = repo::get(conn, shop_id, id)?;
        if !shift.is_open() {
            return Err(CoreError::Conflict {
                field: "closed_at".to_string(),
                message: "that till was already counted and closed".to_string(),
            });
        }
        let closed_at = count.at.unwrap_or_else(clock::now);
        if closed_at < shift.opened_at {
            return Err(CoreError::validation(
                "closed_at",
                "a till cannot be counted before it was opened",
            ));
        }
        // Equal is fine: a drawer opened by mistake and counted straight away
        // is a real evening with an empty window. Later than now is not, for
        // the reason `open` gives above.
        if closed_at > clock::now() {
            return Err(CoreError::validation(
                "closed_at",
                "a till cannot be counted later than now",
            ));
        }
        let takings =
            cash::takings_for(conn, shop_id, shift.opened_by, shift.opened_at, closed_at)?;
        let expected = shift.opening_cash.checked_add(takings.total()?)?;
        if count.counted != expected && note.is_none() {
            return Err(CoreError::validation(
                "note",
                "a drawer that does not match what was expected needs a reason",
            ));
        }
        let closed = repo::close(
            conn,
            shop_id,
            id,
            &ShiftCloseWrite {
                closed_at,
                closed_by,
                counted_centimes: count.counted.as_centimes(),
                expected_at_close_centimes: expected.as_centimes(),
                note: note.clone(),
            },
        )?;
        let difference = count.counted.checked_sub(expected)?;
        audit::record(
            conn,
            shop_id,
            closed_by,
            audit::Change {
                action: audit::ACTION_CLOSE_TILL,
                entity: "shift",
                entity_id: Some(closed.id),
                before: None,
                after: Some(
                    serde_json::json!({
                        "closed_at": closed_at,
                        "expected_at_close_centimes": expected.as_centimes(),
                        "counted_centimes": count.counted.as_centimes(),
                        "difference_centimes": difference.as_centimes(),
                        "note": note,
                        // Only when the two differ: on an ordinary evening the
                        // row's own `user_id` is already the opener, and
                        // writing it twice would read as two people.
                        "opened_by": (closed_by != closed.opened_by).then_some(closed.opened_by),
                    })
                    .to_string(),
                ),
            },
        )?;
        Ok(closed)
    })
}

/// One shift and the figures a screen puts beside it.
pub fn report(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
) -> Result<ShiftReport, CoreError> {
    let shift = repo::get(conn, shop_id, id)?;
    let until = match &shift.close {
        Some(close) => close.closed_at,
        None => clock::now(),
    };
    let takings = cash::takings_for(conn, shop_id, shift.opened_by, shift.opened_at, until)?;
    let (expected, difference) = match &shift.close {
        // The snapshot, not a fresh sum: see `ShiftReport::expected`.
        Some(close) => (close.expected, Some(close.difference()?)),
        None => (shift.opening_cash.checked_add(takings.total()?)?, None),
    };
    Ok(ShiftReport {
        shift,
        takings,
        expected,
        difference,
        until,
    })
}

/// This person's sales over a stretch of the clock that fall inside none of
/// this person's own shifts, oldest first, with the cash among them.
///
/// Their own and not the shop's: a shop-wide list of windows would say a sale
/// belonged to a shift somebody else was holding, which is the same mistake as
/// a shop-wide expected figure wearing different clothes.
pub fn sales_outside_a_shift(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    from: NaiveDateTime,
    until: NaiveDateTime,
) -> Result<OutsideShifts, CoreError> {
    let windows = repo::list_for_user(conn, shop_id, user_id)?;
    let mut sales = Vec::new();
    let mut cash = Money::ZERO;
    for sale in documents::rung_by(conn, shop_id, user_id, from, until)? {
        if windows.iter().any(|shift| covers(shift, sale.issued_at)) {
            continue;
        }
        if sale.payment_mode == PaymentMode::Cash {
            cash = cash.checked_add(sale.net_to_pay)?;
        }
        sales.push(sale);
    }
    Ok(OutsideShifts { sales, cash })
}

/// Writes the `till.sale_outside_shift` row when the sale falls inside none of
/// its ringer's own windows, and answers whether it did.
///
/// The one place that row is written, so the moment a sale is tagged is
/// decided here rather than at whichever call site rang it up. The sale is
/// never refused: seven e2e specs, `just seed`, the demo recordings and the
/// Maestro flows all ring sales with no shift open, and a shop that forgot to
/// open the till in the morning has sold real goods either way.
pub fn tag_if_outside_a_shift(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    document_id: i32,
    issued_at: NaiveDateTime,
) -> Result<bool, CoreError> {
    let windows = repo::list_for_user(conn, shop_id, user_id)?;
    if windows.iter().any(|shift| covers(shift, issued_at)) {
        return Ok(false);
    }
    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action: audit::ACTION_SALE_OUTSIDE_SHIFT,
            entity: "document",
            entity_id: Some(document_id),
            before: None,
            after: Some(
                serde_json::json!({
                    "issued_at": issued_at,
                })
                .to_string(),
            ),
        },
    )?;
    Ok(true)
}

/// Whether a moment falls inside this shift's window: `opened_at <= at` and,
/// once the shift is closed, `at < closed_at`. An open shift has no upper
/// bound.
///
/// Half open at the top and closed at the bottom so two shifts of one person
/// that meet at a moment claim it once between them, rather than both or
/// neither.
fn covers(shift: &Shift, at: NaiveDateTime) -> bool {
    if at < shift.opened_at {
        return false;
    }
    match &shift.close {
        Some(close) => at < close.closed_at,
        None => true,
    }
}
