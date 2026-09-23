//! HTTP handlers. They translate, they do not decide: every rule lives in
//! `dzpos_core::services`, and a permission is a row in `gates/` that
//! answers for a whole route.
//!
//! One handler decides, and only one: `products.rs::redact_cost` blanks
//! `cost_centimes` and `wholesale_centimes` for a role without
//! `SeeCostAndMargin`. `GET /products` cannot be gated as a route because
//! the till needs the catalogue to ring a sale up, and a gate row can only
//! say yes or no to the whole answer, so the field is stripped where the
//! `ProductDto` is built instead. `gates/mod.rs`'s own header names it as the
//! thing a route-level gate cannot do (M4 T5 review, 2026-09-11). A second
//! one would mean a second place to read before trusting what a role sees,
//! which is the cost this rule exists to avoid, so
//! `crates/api/tests/one_handler_decides.rs` walks this folder and fails on
//! it.
//!
//! `auth.rs::held_by` also asks `can`, and is not a second decision: it
//! reports which permissions a role holds by walking `Permission::ALL`, so
//! a screen can grey a button out. It refuses nothing and blanks nothing,
//! and the answer it builds is the same one the gates would give.

// The clinic's appointment book (C5 of the clinic plan), on the same feature.
#[cfg(feature = "clinic")]
pub mod appointments;
pub mod audit;
pub mod auth;
pub mod backups;
// The eleven wholly-retail route files (S5 of
// `a-kernel-crate-and-retail-as-the-first-module`): every handler in each
// goes through a `dzpos_core::services` module `crates/retail` owns, so the
// whole file has nothing to compile once the feature is off. `settings`
// stays unconditional beside them: it is mostly kernel settings, with one
// retail-only route gated inside it instead.
#[cfg(feature = "retail")]
pub mod categories;
#[cfg(feature = "retail")]
pub mod customers;
#[cfg(feature = "retail")]
pub mod dashboard;
#[cfg(feature = "retail")]
pub mod expenses;
#[cfg(feature = "retail")]
pub mod export;
#[cfg(feature = "retail")]
pub mod import;
pub mod pairing;
// The clinic's patient file (C3 of the clinic plan), on its own feature.
#[cfg(feature = "clinic")]
pub mod patients;
#[cfg(feature = "retail")]
pub mod products;
#[cfg(feature = "retail")]
pub mod purchases;
// The clinic's waiting queue (C4 of the clinic plan), on the same feature.
#[cfg(feature = "clinic")]
pub mod queue;
#[cfg(feature = "retail")]
pub mod sales;
pub mod settings;
#[cfg(feature = "retail")]
pub mod stock;
#[cfg(feature = "retail")]
pub mod suppliers;
pub mod support;
#[cfg(feature = "retail")]
pub mod till;
pub mod users;

use axum::extract::State;
use axum::Json;

use crate::dto::{BuildInfoDto, ClockDto, HealthDto};
use crate::error::ApiError;
use crate::AppState;

pub async fn health(State(state): State<AppState>) -> Result<Json<HealthDto>, ApiError> {
    let shop = state.shop_id;
    let needs_first_setup = state
        .blocking(move |c| dzpos_core::services::users::shop_needs_first_setup(c, shop))
        .await?;
    Ok(Json(HealthDto {
        status: "ok".to_string(),
        shop_id: shop,
        needs_first_setup,
    }))
}

/// The About screen's one source (M5 T1): the version, the git short hash
/// and the build date `crates/core::build_info` baked in at compile time,
/// read back here rather than a second copy kept in `apps/desktop`.
pub async fn build_info() -> Json<BuildInfoDto> {
    Json(BuildInfoDto::from(dzpos_core::build_info::BUILD_INFO))
}

/// The shop's calendar. Read from the same `clock` the services date
/// documents with, so a screen and a stored row never disagree about which
/// day it is.
pub async fn clock() -> Json<ClockDto> {
    Json(ClockDto {
        today: dzpos_core::services::clock::now()
            .date()
            .format("%Y-%m-%d")
            .to_string(),
    })
}

pub async fn not_found() -> ApiError {
    ApiError::NoRoute
}

/// A known path with a method it does not take (`DELETE /products`).
/// axum's default answer is an empty 405 the client reads as unreachable.
pub async fn method_not_allowed() -> ApiError {
    ApiError::MethodNotAllowed
}
