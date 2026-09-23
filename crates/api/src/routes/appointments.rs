//! The clinic's appointment book routes (C5 of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`).
//! They translate: every rule lives in `dzpos_core::services::appointments`
//! and `slot_length`, and the permissions are rows in `gates/table.rs` (the
//! patient file's two for the book, `EditSettings` for the slot length),
//! not checks here.
//!
//! Every write answers with the appointment as it now stands, the patient's
//! names with it, so the screen redraws one slot without reading the day
//! again.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::services::appointments::{self as service, NewAppointment};
use dzpos_core::services::slot_length;
use serde::Deserialize;

use crate::dto::{
    parse_day, parse_starts_at, AppointmentBookDto, AppointmentDto, AppointmentMoveDto,
    AppointmentsDto, SlotMinutesDto, DATE_FORMAT,
};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// `?day=YYYY-MM-DD` or `?week=YYYY-MM-DD` (any day of the week), exactly
/// one of the two.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookQuery {
    day: Option<String>,
    week: Option<String>,
}

/// One day of the book, or the Sunday-to-Saturday week holding a day.
pub async fn list(
    State(state): State<AppState>,
    query: Result<Query<BookQuery>, QueryRejection>,
) -> Result<Json<AppointmentsDto>, ApiError> {
    let one_of =
        || ApiError::BadRequest("ask for one day=YYYY-MM-DD or one week=YYYY-MM-DD".into());
    let Query(query) = query.map_err(|_| one_of())?;
    let (from, to, week) = match (query.day, query.week) {
        (Some(day), None) => {
            let day = parse_day("day", &day)?;
            (day, day, false)
        }
        (None, Some(week)) => {
            let (first, last) = service::week_of(parse_day("week", &week)?)?;
            (first, last, true)
        }
        _ => return Err(one_of()),
    };
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| {
            if week {
                service::week(c, shop, from)
            } else {
                service::day(c, shop, from)
            }
        })
        .await?;
    Ok(Json(AppointmentsDto {
        from: from.format(DATE_FORMAT).to_string(),
        to: to.format(DATE_FORMAT).to_string(),
        appointments: found.into_iter().map(AppointmentDto::from).collect(),
    }))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<AppointmentDto>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| service::get(c, shop, &id)).await?;
    Ok(Json(AppointmentDto::from(found)))
}

pub async fn book(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<AppointmentBookDto>, JsonRejection>,
) -> Result<(StatusCode, Json<AppointmentDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewAppointment {
        patient_id: dto.patient_id,
        starts_at: parse_starts_at(&dto.starts_at)?,
        note: dto.note,
        visit_type_id: dto.visit_type_id,
    };
    let shop = state.shop_id;
    let user = who.id;
    let made = state
        .blocking(move |c| service::book(c, shop, user, new))
        .await?;
    Ok((StatusCode::CREATED, Json(AppointmentDto::from(made))))
}

pub async fn cancel(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<AppointmentDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::cancel(c, shop, user, &id))
        .await?;
    Ok(Json(AppointmentDto::from(after)))
}

/// The same row at a new start, on a booking's terms.
pub async fn move_to(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
    body: Result<Json<AppointmentMoveDto>, JsonRejection>,
) -> Result<Json<AppointmentDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let starts_at = parse_starts_at(&dto.starts_at)?;
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::move_to(c, shop, user, &id, starts_at))
        .await?;
    Ok(Json(AppointmentDto::from(after)))
}

/// The slot length in force. An ordinary read with no gate row, like
/// `GET /settings`: a number, nobody's name.
pub async fn slot_minutes(State(state): State<AppState>) -> Result<Json<SlotMinutesDto>, ApiError> {
    let shop = state.shop_id;
    let slot_minutes = state
        .blocking(move |c| slot_length::slot_minutes(c, shop))
        .await?;
    Ok(Json(SlotMinutesDto { slot_minutes }))
}

pub async fn set_slot_minutes(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<SlotMinutesDto>, JsonRejection>,
) -> Result<Json<SlotMinutesDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let user = who.id;
    let slot_minutes = state
        .blocking(move |c| slot_length::set_slot_minutes(c, shop, user, dto.slot_minutes))
        .await?;
    Ok(Json(SlotMinutesDto { slot_minutes }))
}

/// Marks a past appointment as missed.
pub async fn mark_no_show(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<AppointmentDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::mark_no_show(c, shop, user, &id))
        .await?;
    Ok(Json(AppointmentDto::from(after)))
}

/// Takes a no-show mark back.
pub async fn clear_no_show(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<AppointmentDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| service::clear_no_show(c, shop, user, &id))
        .await?;
    Ok(Json(AppointmentDto::from(after)))
}
