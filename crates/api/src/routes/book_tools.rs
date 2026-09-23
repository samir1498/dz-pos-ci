//! The book tools' routes (C5b of
//! `context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`).
//! They translate: every rule lives in the clinic's services, and the
//! permissions are rows in `gates/clinic.rs`, not checks here.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::services::absence_blocks::{self, NewBlock};
use dzpos_core::services::day_list;
use dzpos_core::services::free_slot::{self, FreeSlotQuery};
use dzpos_core::services::visit_types::{self, NewVisitType};
use dzpos_core::services::working_hours;
use serde::Deserialize;

use crate::dto::{
    parse_day, parse_moment, parse_week, AbsenceBlockDto, AbsenceBlockWriteDto, AbsenceBlocksDto,
    BlockMadeDto, DayListDto, FreeSlotDto, VisitTypeDto, VisitTypeWriteDto, VisitTypesDto,
    WorkingHoursDto, WorkingHoursWriteDto,
};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// The cabinet's week, or `null` days when it has never set one. An
/// ordinary read with no gate row, like the slot length's: hours, nobody's
/// name.
pub async fn working_hours(
    State(state): State<AppState>,
) -> Result<Json<WorkingHoursDto>, ApiError> {
    let shop = state.shop_id;
    let week = state
        .blocking(move |c| working_hours::week(c, shop))
        .await?;
    Ok(Json(WorkingHoursDto::from(week)))
}

pub async fn set_working_hours(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<WorkingHoursWriteDto>, JsonRejection>,
) -> Result<Json<WorkingHoursDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let days = parse_week(dto)?;
    let shop = state.shop_id;
    let user = who.id;
    let week = state
        .blocking(move |c| working_hours::set_week(c, shop, user, days))
        .await?;
    Ok(Json(WorkingHoursDto::from(Some(week))))
}

/// The blocks not yet over. Open like the working hours: when the doctor
/// is away, nobody's name.
pub async fn absence_blocks(
    State(state): State<AppState>,
) -> Result<Json<AbsenceBlocksDto>, ApiError> {
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| absence_blocks::upcoming(c, shop))
        .await?;
    Ok(Json(AbsenceBlocksDto {
        blocks: found.into_iter().map(AbsenceBlockDto::from).collect(),
    }))
}

pub async fn create_absence_block(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<AbsenceBlockWriteDto>, JsonRejection>,
) -> Result<(StatusCode, Json<BlockMadeDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewBlock {
        starts_at: parse_moment("starts_at", &dto.starts_at)?,
        ends_at: parse_moment("ends_at", &dto.ends_at)?,
        label: dto.label,
    };
    let shop = state.shop_id;
    let user = who.id;
    let made = state
        .blocking(move |c| absence_blocks::create(c, shop, user, new))
        .await?;
    Ok((StatusCode::CREATED, Json(BlockMadeDto::from(made))))
}

/// Removes a block and answers with it as it was.
pub async fn remove_absence_block(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<AbsenceBlockDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let gone = state
        .blocking(move |c| absence_blocks::remove(c, shop, user, &id))
        .await?;
    Ok(Json(AbsenceBlockDto::from(gone)))
}

/// Every visit type, by name. Open like the slot length: a setting, nobody's
/// name.
pub async fn visit_types(State(state): State<AppState>) -> Result<Json<VisitTypesDto>, ApiError> {
    let shop = state.shop_id;
    let found = state.blocking(move |c| visit_types::list(c, shop)).await?;
    Ok(Json(VisitTypesDto {
        visit_types: found.into_iter().map(VisitTypeDto::from).collect(),
    }))
}

pub async fn create_visit_type(
    State(state): State<AppState>,
    who: CurrentUser,
    body: Result<Json<VisitTypeWriteDto>, JsonRejection>,
) -> Result<(StatusCode, Json<VisitTypeDto>), ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewVisitType {
        name: dto.name,
        minutes: dto.minutes,
    };
    let shop = state.shop_id;
    let user = who.id;
    let made = state
        .blocking(move |c| visit_types::create(c, shop, user, new))
        .await?;
    Ok((StatusCode::CREATED, Json(VisitTypeDto::from(made))))
}

pub async fn update_visit_type(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
    body: Result<Json<VisitTypeWriteDto>, JsonRejection>,
) -> Result<Json<VisitTypeDto>, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let new = NewVisitType {
        name: dto.name,
        minutes: dto.minutes,
    };
    let shop = state.shop_id;
    let user = who.id;
    let after = state
        .blocking(move |c| visit_types::update(c, shop, user, &id, new))
        .await?;
    Ok(Json(VisitTypeDto::from(after)))
}

/// Removes a type and answers with it as it was.
pub async fn remove_visit_type(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(id): Path<String>,
) -> Result<Json<VisitTypeDto>, ApiError> {
    let shop = state.shop_id;
    let user = who.id;
    let gone = state
        .blocking(move |c| visit_types::remove(c, shop, user, &id))
        .await?;
    Ok(Json(VisitTypeDto::from(gone)))
}

/// `?from=YYYY-MM-DD&offset_days=15&visit_type_id=...`, each optional.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreeSlotParams {
    from: Option<String>,
    offset_days: Option<u32>,
    visit_type_id: Option<String>,
}

/// The earliest start a booking would be taken at. Gated like the book it
/// reads: the answer tells which slots are taken across 60 days.
pub async fn next_free(
    State(state): State<AppState>,
    params: Result<Query<FreeSlotParams>, QueryRejection>,
) -> Result<Json<FreeSlotDto>, ApiError> {
    let Query(params) = params.map_err(|_| {
        ApiError::BadRequest(
            "ask with from=YYYY-MM-DD, offset_days as a whole number of days and visit_type_id, \
             each optional"
                .into(),
        )
    })?;
    let query = FreeSlotQuery {
        from: params
            .from
            .as_deref()
            .map(|d| parse_day("from", d))
            .transpose()?,
        offset_days: params.offset_days.unwrap_or(0),
        visit_type_id: params.visit_type_id,
    };
    let shop = state.shop_id;
    let found = state
        .blocking(move |c| free_slot::next_free(c, shop, query))
        .await?;
    Ok(Json(FreeSlotDto::from(found)))
}

/// `?day=YYYY-MM-DD`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayListParams {
    day: String,
}

/// One day's appointments and walk-ins, by name: gated like the book.
pub async fn day_list(
    State(state): State<AppState>,
    params: Result<Query<DayListParams>, QueryRejection>,
) -> Result<Json<DayListDto>, ApiError> {
    let Query(params) =
        params.map_err(|_| ApiError::BadRequest("ask for one day=YYYY-MM-DD".into()))?;
    let day = parse_day("day", &params.day)?;
    let shop = state.shop_id;
    let found = state.blocking(move |c| day_list::day(c, shop, day)).await?;
    Ok(Json(DayListDto::from(found)))
}
