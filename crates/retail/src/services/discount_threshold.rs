//! The discount-threshold setting: the rate above which a discount needs
//! `dzpos_kernel::services::permissions::Permission::DiscountAboveThreshold`
//! (the milestone's brief: "the discount threshold becomes a settings value
//! with a dated history, exactly the way the régime fiscal already does
//! it"). Moved out of `dzpos_kernel::services::settings` by S4 of
//! `a-kernel-crate-and-retail-as-the-first-module`: unlike the régime
//! fiscal beside it there, which every trade this app could ever carry
//! would need the same way, a discount rule is a shop's own. Built on four
//! thin, generic wrappers that module still exposes over its own
//! dated-settings repo, which this crate cannot reach directly
//! (`crates/kernel/src/repos` is `pub(crate)`).

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;
use dzpos_kernel::error::CoreError;
use dzpos_kernel::money::Bps;
use dzpos_kernel::services::{audit, settings};

use crate::audit_actions;

/// The key holding the rate above which a discount needs the permission
/// above. Unlike `settings::REGIME_FISCAL`, no migration seeds a first row,
/// so a shop that has never set one reads as `Bps::ZERO`
/// (`discount_threshold_as_of`), the conservative default that needs the
/// permission for any discount at all until an owner raises it.
pub const DISCOUNT_THRESHOLD_BPS: &str = "discount_threshold_bps";

/// A discount threshold and the moment it took, or takes, effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatedThreshold {
    pub threshold: Bps,
    pub valid_from: NaiveDateTime,
}

/// The threshold current at `at`, and since when. `None` when the shop has
/// never set one: there is no seeded row to fall back on the way the
/// régime fiscal has, and inventing a "since" date for a default nobody
/// chose would be a second, silent decision.
pub fn discount_threshold_current(
    conn: &mut SqliteConnection,
    shop_id: i32,
    at: NaiveDateTime,
) -> Result<Option<DatedThreshold>, CoreError> {
    settings::current_as_of(conn, shop_id, DISCOUNT_THRESHOLD_BPS, at)?
        .map(|(value, valid_from)| {
            Ok(DatedThreshold {
                threshold: parse_bps(&value)?,
                valid_from,
            })
        })
        .transpose()
}

/// A threshold change dated after `at` that has not taken effect yet, the
/// same shape as `settings::regime_planned`.
pub fn discount_threshold_planned(
    conn: &mut SqliteConnection,
    shop_id: i32,
    at: NaiveDateTime,
) -> Result<Option<DatedThreshold>, CoreError> {
    settings::next_after(conn, shop_id, DISCOUNT_THRESHOLD_BPS, at)?
        .map(|(value, valid_from)| {
            Ok(DatedThreshold {
                threshold: parse_bps(&value)?,
                valid_from,
            })
        })
        .transpose()
}

/// The threshold a discount given at `at` is checked against. What
/// `dzpos_kernel::services::permissions::discount_needs_permission` is
/// called with. `Bps::ZERO` when the shop has never set one, see
/// `DISCOUNT_THRESHOLD_BPS`.
pub fn discount_threshold_as_of(
    conn: &mut SqliteConnection,
    shop_id: i32,
    at: NaiveDateTime,
) -> Result<Bps, CoreError> {
    match settings::value_as_of(conn, shop_id, DISCOUNT_THRESHOLD_BPS, at)? {
        Some(value) => parse_bps(&value),
        None => Ok(Bps::ZERO),
    }
}

fn parse_bps(value: &str) -> Result<Bps, CoreError> {
    let raw: u32 = value.parse().map_err(|_| {
        CoreError::validation(
            DISCOUNT_THRESHOLD_BPS,
            &format!("{value} is not a discount threshold"),
        )
    })?;
    Ok(Bps::new(raw)?)
}

/// Records that a discount above `threshold` needs the permission from
/// `valid_from`. Earlier rows stay: a sale rung up yesterday is judged
/// against yesterday's threshold, the same reason `settings::set_regime`
/// never updates a row in place.
pub fn set_discount_threshold(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    threshold: Bps,
    valid_from: NaiveDateTime,
) -> Result<(), CoreError> {
    conn.transaction(|conn| {
        let before = settings::value_as_of(conn, shop_id, DISCOUNT_THRESHOLD_BPS, valid_from)?;
        // Same value, same day: nothing to record. See settings::set_regime
        // for why.
        if before.as_deref() == Some(threshold.as_u32().to_string().as_str()) {
            return Err(CoreError::validation(
                DISCOUNT_THRESHOLD_BPS,
                "the shop is already under that discount threshold on that day",
            ));
        }
        settings::append(
            conn,
            shop_id,
            DISCOUNT_THRESHOLD_BPS,
            &threshold.as_u32().to_string(),
            valid_from,
        )?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit_actions::ACTION_SET_DISCOUNT_THRESHOLD,
                entity: DISCOUNT_THRESHOLD_BPS,
                entity_id: Some(shop_id),
                before: before.map(|value| {
                    serde_json::json!({ "discount_threshold_bps": value }).to_string()
                }),
                after: Some(
                    serde_json::json!({
                        "discount_threshold_bps": threshold.as_u32(),
                        "valid_from": valid_from.to_string(),
                    })
                    .to_string(),
                ),
            },
        )
    })
}
