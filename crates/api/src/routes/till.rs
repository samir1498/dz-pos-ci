//! The till's shifts: opening a drawer with a float, counting it at close,
//! and reading either one back (features.md §1, the cash position; plan
//! `till-shifts-a-float-and-a-count` T4). They translate; every rule lives
//! in `dzpos_core::services::shifts`.
//!
//! One file and never a folder. `crates/api/tests/one_handler_decides.rs`
//! walks `src/routes` one directory deep and skips every `mod.rs`, which is
//! exactly where a folder route's handler would live, so a route split into
//! a folder is a handler that walk never opens.
//!
//! There is no movements route: ruling 1 of the plan took the movement out.
//! Cash handed to the owner is the note at close, and money paid out of the
//! shop's own money is an `expenses` row written by a `commit_money` holder.
//!
//! **Who may call what.** `gates/table.rs` decides, once per route, as it
//! does for every other route here. Opening and counting are
//! `OpenAndCloseTill`, which all three roles hold (ruling 10): the person
//! who counts a drawer is the person standing at it. Reading somebody's
//! shift by id, or the shop's shifts as a list, is `SeeReports`, because
//! both are a report a manager runs the floor off. `GET /till/shifts/open`
//! carries no row at all: it answers about the caller and nobody else, and a
//! cashier who cannot read their own open drawer cannot be shown the
//! expected figure before they count it.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::services::clock;
use dzpos_core::services::shifts::{self as service, NewShift, TillCount};
use serde::Deserialize;

use crate::dto::{parse_day, NewShiftDto, ShiftDto, ShiftReportDto, TillCountDto};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// Opens the caller's own drawer with what is in it.
///
/// The opener is the caller and is never read off the body: a route that
/// took an opener would let one person open a drawer in another's name, and
/// the expected figure is summed over that name's own sales.
pub async fn open(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<NewShiftDto>, JsonRejection>,
) -> Result<(StatusCode, Json<ShiftDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let fields = NewShift::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    let made = state
        .blocking(move |c| service::open(c, shop, user, fields))
        .await?;
    Ok((StatusCode::CREATED, Json(ShiftDto::try_from(made)?)))
}

/// Counts the drawer and closes it. The expected figure is the core's, worked
/// out at the moment of closing and never accepted from the caller.
///
/// One route for both cases, the cashier counting their own and somebody
/// else counting a drawer that was walked away from: which of the two it is
/// is `services::shifts::close`'s to decide, and the row it writes names
/// both people.
pub async fn close(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<i32>,
    body: Result<Json<TillCountDto>, JsonRejection>,
) -> Result<Json<ShiftDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let count = TillCount::try_from(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    let closed = state
        .blocking(move |c| service::close(c, shop, id, user, count))
        .await?;
    Ok(Json(ShiftDto::try_from(closed)?))
}

/// The caller's own open drawer with its live figures, or `null`.
///
/// `null` rather than a 404: "no drawer is open" is an answer the till
/// screen asks for on every sign-in and acts on, not a missing row. The
/// same shape `routes::stock::last` gives for "no recount yet".
///
/// The caller's own and nobody else's. Whose drawer is open is not a
/// question this route takes a parameter for, which is what lets it stay
/// ungated: there is nothing here a role could be refused.
pub async fn open_shift(
    State(state): State<AppState>,
    who: CurrentUser,
) -> Result<Json<Option<ShiftReportDto>>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let found = state
        .blocking(move |c| match service::open_for(c, shop, user)? {
            Some(shift) => service::report(c, shop, shift.id).map(Some),
            None => Ok(None),
        })
        .await?;
    Ok(Json(found.map(ShiftReportDto::try_from).transpose()?))
}

/// One shift by id, with the figures a screen puts beside it. A shift of
/// another shop is a 404, the way every other read by id is.
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ShiftReportDto>, ApiError> {
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::report(c, shop, id))
        .await?;
    Ok(Json(ShiftReportDto::try_from(found)?))
}

/// A day window, both ends optional, and an optional person. Left out, the
/// window is today alone, on the shop's clock: the manager's screen opens on
/// what is happening now rather than the whole shop's history.
#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    user_id: Option<i32>,
}

/// The shop's shifts opened over a day window, newest first: the row only,
/// never the report each carries — a report per row is a query per row, and
/// `GET /till/shifts/{id}` is already what a screen asks for one at a time.
pub async fn list(
    State(state): State<AppState>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Vec<ShiftDto>>, ApiError> {
    let Query(ListQuery { from, to, user_id }) = query.map_err(|_| {
        ApiError::BadRequest("from and to are days written YYYY-MM-DD, user_id is a number".into())
    })?;
    let today = clock::now().date();
    let from = match from.as_deref() {
        Some(text) => parse_day("from", text)?,
        None => today,
    };
    let to = match to.as_deref() {
        Some(text) => parse_day("to", text)?,
        None => today,
    };
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| service::list(c, shop, from, to, user_id))
        .await?;
    Ok(Json(
        found
            .into_iter()
            .map(ShiftDto::try_from)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}
