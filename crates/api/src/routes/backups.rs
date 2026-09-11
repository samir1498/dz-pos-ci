//! The backup routes: what copies there are, one more now, and putting the
//! shop file back from one of them. Every rule lives in
//! `dzpos_core::services::backup`; this file translates, and it owns exactly
//! one decision of its own, which is that a caller names a copy and never a
//! path.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use dzpos_core::error::CoreError;
use dzpos_core::services::backup;

use crate::dto::{BackupDto, BackupsDto, RestoreDto};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

// The shop's clock lives in `routes::settings` (Algeria, UTC+1, no daylight
// saving) and is read from there rather than restated: a backup is named for
// the moment the shop had, not the moment UTC had, or the copy taken just
// after midnight would carry yesterday's date on the screen that lists it.
pub(crate) use crate::routes::settings::now;

pub async fn list(State(state): State<AppState>) -> Result<Json<BackupsDto>, ApiError> {
    let (daily, safety) = tokio::task::spawn_blocking(move || state.list_backups())
        .await
        .map_err(|_| ApiError::Unavailable)??;
    Ok(Json(BackupsDto {
        backups: daily.into_iter().map(BackupDto::from).collect(),
        safety_copies: safety.into_iter().map(BackupDto::from).collect(),
    }))
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
    who: CurrentUser,
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
    let actor = who.id;
    let done = tokio::task::spawn_blocking(move || state.restore(&path, actor))
        .await
        .map_err(|_| ApiError::Unavailable)??;
    Ok(Json(RestoreDto {
        restored_from,
        safety_copy: done.safety_copy,
        products: done.summary.products,
        documents: done.summary.documents,
    }))
}
