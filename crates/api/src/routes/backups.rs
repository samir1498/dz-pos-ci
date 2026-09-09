//! The backup routes: what copies there are, one more now, and putting the
//! shop file back from one of them. Every rule lives in
//! `dzpos_core::services::backup`; this file translates, and it owns exactly
//! one decision of its own, which is that a caller names a copy and never a
//! path.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use dzpos_core::error::CoreError;
use dzpos_core::services::backup;

use crate::dto::{BackupDto, RestoreDto};
use crate::error::ApiError;
use crate::AppState;

/// The shop's clock: Algeria, UTC+1, no daylight saving. The same reading
/// `routes::settings` takes for a régime day; a backup is named for the
/// moment the shop had, not the moment UTC had, or the copy taken just after
/// midnight would carry yesterday's date on the screen that lists it.
const SHOP_UTC_OFFSET_SECONDS: i32 = 3600;

/// `utc` read on the shop's calendar. Separate from `now` so the wall clock
/// never enters a test.
fn shop_time(utc: DateTime<Utc>) -> NaiveDateTime {
    match FixedOffset::east_opt(SHOP_UTC_OFFSET_SECONDS) {
        Some(offset) => utc.with_timezone(&offset).naive_local(),
        // 3600 is inside the range east_opt accepts, so this arm is never
        // taken; UTC is the honest fallback rather than a panic.
        None => utc.naive_utc(),
    }
}

pub fn now() -> NaiveDateTime {
    shop_time(Utc::now())
}

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<BackupDto>>, ApiError> {
    let dir = state.backup_dir().to_path_buf();
    let found = tokio::task::spawn_blocking(move || backup::list(&dir))
        .await
        .map_err(|_| ApiError::Unavailable)?
        .map_err(ApiError::from)?;
    Ok(Json(found.into_iter().map(BackupDto::from).collect()))
}

pub async fn create(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<BackupDto>), ApiError> {
    let at = now();
    let made = tokio::task::spawn_blocking(move || state.create_backup(at))
        .await
        .map_err(|_| ApiError::Unavailable)??;
    Ok((StatusCode::CREATED, Json(BackupDto::from(made))))
}

/// The name is checked against the pattern the service writes, and only then
/// joined to the folder the server owns. `..%2Fx` reaches this handler
/// percent-decoded, as `../x`, so the decoded name is what the check sees:
/// nothing with a separator, a second dot or a `..` in it parses as a
/// backup name, and the join never happens.
pub async fn restore(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<RestoreDto>, ApiError> {
    if backup::taken_at(&name).is_none() {
        return Err(ApiError::Request(CoreError::validation(
            "name",
            "is not the name of a backup this shop wrote",
        )));
    }
    let path = state.backup_dir().join(&name);
    let restored_from = name;
    let summary = tokio::task::spawn_blocking(move || state.restore(&path))
        .await
        .map_err(|_| ApiError::Unavailable)??;
    Ok(Json(RestoreDto {
        restored_from,
        products: summary.products,
        documents: summary.documents,
    }))
}
