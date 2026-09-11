//! The stock recount's two routes: run one now, and read what the last one
//! found. They translate; the comparison, the repair and the once-a-day rule
//! all live in `dzpos_core::services::stock`.
//!
//! There is no route that writes a cached quantity. The ledger is the truth
//! (features.md §1), so the only way a quantity moves is a movement, and the
//! only way the cache moves on its own is this recount putting it back.

use axum::extract::State;
use axum::Json;
use dzpos_core::services::stock as service;

use crate::dto::{LastStockRecountDto, StockRecountDto};
use crate::error::ApiError;
use crate::session::CurrentUser;
use crate::AppState;

/// Runs a recount now, whether or not the shop has already had one today:
/// the owner pressed the button because they want the file looked at again,
/// and answering "already done" would leave them with no way to check.
///
/// A 200 and not a 201. Nothing here is created that a caller can go and
/// read at an address of its own; the answer is the whole of what happened.
pub async fn recount(
    State(state): State<AppState>,
    who: CurrentUser,
) -> Result<Json<StockRecountDto>, ApiError> {
    let shop = state.shop_id;
    // TODO(M4): the user comes from the request identity, not from the state.
    let user = who.id;
    let report = state
        .blocking(move |c| service::recount(c, shop, user))
        .await?;
    Ok(Json(StockRecountDto::from(report)))
}

/// The day the shop last recounted and what that day corrected.
pub async fn last(State(state): State<AppState>) -> Result<Json<LastStockRecountDto>, ApiError> {
    let shop = state.shop_id;
    let last = state
        .blocking(move |c| service::last_recount(c, shop))
        .await?;
    Ok(Json(LastStockRecountDto::from(last)))
}
