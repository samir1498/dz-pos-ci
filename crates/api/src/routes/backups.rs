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
    let copies = tokio::task::spawn_blocking(move || state.list_backups())
        .await
        .map_err(|_| ApiError::Unavailable)??;
    Ok(Json(BackupsDto {
        backups: copies.daily.into_iter().map(BackupDto::from).collect(),
        safety_copies: copies.safety.into_iter().map(BackupDto::from).collect(),
        upgrade_copies: copies.upgrade.into_iter().map(BackupDto::from).collect(),
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

/// The name is checked against the patterns the service writes, and only
/// then joined to a folder the server owns. `..%2Fx` reaches this handler
/// percent-decoded, as `../x`, so the decoded name is what the check sees:
/// nothing with a separator, a second dot or a `..` in it parses as any of
/// the three names, and the join never happens.
///
/// Which folder is the name's own answer, not the caller's. A daily copy is
/// in the backup folder; the two kinds taken on the way into something, a
/// restore or an upgrade, sit beside the shop file, because the prune walks
/// the folder and must never reach them. A name is one kind or none: the
/// three patterns differ by the word in the middle.
pub async fn restore(
    State(state): State<AppState>,
    who: CurrentUser,
    Path(name): Path<String>,
) -> Result<Json<RestoreDto>, ApiError> {
    let db = state.db_path();
    let path = if backup::taken_at(&name).is_some() {
        state.backup_dir().join(&name)
    } else if backup::safety_taken_at(db, &name).is_some()
        || backup::upgrade_taken_at(db, &name).is_some()
    {
        db.with_file_name(&name)
    } else {
        return Err(ApiError::Request(CoreError::validation(
            "name",
            "is not the name of a copy this shop wrote",
        )));
    };
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
