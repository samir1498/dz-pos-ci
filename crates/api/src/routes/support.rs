//! The support bundle route (M5 T3). Every rule about what the zip holds
//! lives in `dzpos_core::services::support_bundle`; this file names the
//! file and sets the two headers a browser needs to save it, the same shape
//! `routes::export::workbook` uses for the four spreadsheets.

use axum::extract::State;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};

use crate::error::ApiError;
use crate::routes::settings::now;
use crate::session::CurrentUser;
use crate::AppState;

const ZIP: &str = "application/zip";

/// The zip, named for the shop's own day (`routes::settings::now`, the same
/// clock a backup is named for), never the machine's UTC clock.
pub async fn bundle(State(state): State<AppState>, who: CurrentUser) -> Result<Response, ApiError> {
    let actor = who.id;
    let bytes = tokio::task::spawn_blocking(move || state.support_bundle(actor))
        .await
        .map_err(|_| ApiError::Unavailable)??;
    let filename = format!("dzpos-support-{}.zip", now().date().format("%Y-%m-%d"));
    Ok((
        [
            (CONTENT_TYPE, ZIP.to_owned()),
            (
                CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}
