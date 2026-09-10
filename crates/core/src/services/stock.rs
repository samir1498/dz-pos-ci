//! The stock ledger (features.md §1). Every change to a quantity on hand is
//! a row here, and `products.qty_on_hand_milli` is a cache of the sum. The
//! ledger is the truth; the recount is what proves the cache still matches
//! it, and puts it back when it does not.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::stock::{Drift, Movement, StockMovement, StockMovementRowWrite};
use crate::money::Money;
use crate::repos::{audit as audit_repo, jobs, stock as repo};
use crate::services::{audit, clock};

/// The name the `jobs` table keeps the recount's last run under. One row per
/// shop, so two shops on one file are counted on their own days.
pub const JOB_STOCK_RECOUNT: &str = "stock_recount";

/// A day on the shop's calendar, the only shape a marker is written in.
const DAY_FORMAT: &str = "%Y-%m-%d";

/// What one run of the recount found and did: every product compared, the
/// ones whose cache the ledger did not explain, and the day the run was
/// marked under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub day: String,
    /// How many products were compared, drifting or not. A shop owner
    /// reading an empty drift list wants to know something was looked at.
    pub checked: usize,
    pub drifts: Vec<Drift>,
}

/// The last run as the file remembers it: the day the marker holds, and the
/// drifts that day corrected, read back out of the audit log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastRecount {
    /// `None` when this shop has never recounted.
    pub last_run_day: Option<String>,
    pub drifts: Vec<Drift>,
}

/// Writes one movement and moves the product's cached quantity by the same
/// amount. Call it inside the caller's transaction: the row and the cache
/// have to commit together, and a sale writes one movement per line.
///
/// Stock may end up below zero. A shop's count is often wrong before its
/// first inventory, and refusing the sale would stop the till over a number
/// nobody typed in.
pub fn record(
    conn: &mut SqliteConnection,
    shop_id: i32,
    movement: &Movement,
) -> Result<StockMovement, CoreError> {
    if movement.unit_cost.is_negative() {
        return Err(CoreError::validation(
            "unit_cost_centimes",
            "a unit cost cannot be negative",
        ));
    }
    // The cache update carries the shop scope, so a product of another shop
    // is reported before the ledger row exists.
    repo::add_to_cached_quantity(conn, shop_id, movement.product_id, movement.qty_milli)?;
    repo::insert(
        conn,
        &StockMovementRowWrite {
            shop_id,
            product_id: movement.product_id,
            kind: movement.kind,
            qty_milli: movement.qty_milli,
            unit_cost_centimes: movement.unit_cost.as_centimes(),
            document_id: movement.document_id,
            user_id: movement.user_id,
        },
    )
}

pub fn list_for_product(
    conn: &mut SqliteConnection,
    shop_id: i32,
    product_id: i32,
) -> Result<Vec<StockMovement>, CoreError> {
    repo::list_for_product(conn, shop_id, product_id)
}

/// What each product cost when it left on this document, per product.
///
/// A reversal reads this rather than the fiche. The goods a credit note puts
/// back are worth what they were worth when they left: a delivery between
/// the sale and the credit note moves the fiche's cost, and a reversal
/// written at the new one would move the month's margin with every purchase
/// (`an_avoir_returns_the_goods_at_the_cost_of_the_sale_it_reverses`). A
/// product the map has no entry for moved no stock on that document.
pub fn sale_costs(
    conn: &mut SqliteConnection,
    shop_id: i32,
    document_id: i32,
) -> Result<std::collections::HashMap<i32, Money>, CoreError> {
    repo::sale_costs_of_document(conn, shop_id, document_id)
}

/// Compares every product's cached quantity with the sum of its movements,
/// writes the ledger back over a cache that disagrees, and logs each
/// correction. The whole run is one transaction: a cache put right without
/// the row that says so would be a quantity that changed with nobody's name
/// on it, and half a repaired shop is worse than none.
///
/// The ledger is the truth (features.md §1), so the repair only ever goes
/// one way. A run that finds nothing still moves the marker, or a till
/// switched on and off all day would recount on every check.
pub fn recount(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<Report, CoreError> {
    let today = clock::now().date().format(DAY_FORMAT).to_string();
    conn.transaction(|conn| run(conn, shop_id, user_id, &today))
}

/// Whether the recount still owes this shop a run today.
///
/// Written as "the marker is older than today" rather than "the marker is
/// not today": a file carried back from a machine whose clock ran ahead
/// holds a day in the future, and `!=` would recount on every check until
/// the calendar caught up. Both are days on the shop's calendar written
/// `YYYY-MM-DD`, which compares as text in date order.
pub fn is_due(last_run_day: Option<&str>, today: &str) -> bool {
    last_run_day.is_none_or(|day| day < today)
}

/// The nightly run: the marker is read and the recount is done under one
/// transaction, so two checks that overlap cannot both decide it is due.
/// `None` means the shop was already counted today.
pub fn recount_if_due(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<Option<Report>, CoreError> {
    let today = clock::now().date().format(DAY_FORMAT).to_string();
    conn.transaction(|conn| {
        let marked = jobs::get(conn, shop_id, JOB_STOCK_RECOUNT)?.and_then(|job| job.last_run_day);
        if !is_due(marked.as_deref(), &today) {
            return Ok(None);
        }
        run(conn, shop_id, user_id, &today).map(Some)
    })
}

/// The day the shop last recounted and what that day corrected. There is no
/// table of runs: the audit rows are the record (no new migration for this),
/// so the drifts are the ones filed under the day the marker holds. Two runs
/// on one day therefore read as one list, which is the honest answer: both
/// corrected the shop on the day the panel is naming.
pub fn last_recount(conn: &mut SqliteConnection, shop_id: i32) -> Result<LastRecount, CoreError> {
    let Some(day) = jobs::get(conn, shop_id, JOB_STOCK_RECOUNT)?.and_then(|job| job.last_run_day)
    else {
        return Ok(LastRecount {
            last_run_day: None,
            drifts: Vec::new(),
        });
    };
    // Backwards from the newest, stopping at the first row that is not of
    // this day: the rows of one day sit together at the end of the log, and
    // a row whose JSON cannot be read says no day, which ends the run the
    // same way rather than refusing the whole screen.
    let mut drifts: Vec<Drift> = audit_repo::by_action(conn, shop_id, audit::ACTION_STOCK_DRIFT)?
        .into_iter()
        .map_while(|entry| drift_of(&entry, &day))
        .collect();
    drifts.reverse();
    Ok(LastRecount {
        last_run_day: Some(day),
        drifts,
    })
}

/// One recount, inside a transaction the caller opened and against a day the
/// caller read from the clock.
fn run(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    day: &str,
) -> Result<Report, CoreError> {
    let counted = repo::cached_and_ledger(conn, shop_id)?;
    let checked = counted.len();
    let mut drifts = Vec::new();
    for product in counted {
        if product.cached_milli == product.ledger_milli {
            continue;
        }
        let drift = Drift {
            product_id: product.product_id,
            name: product.name,
            cached_milli: product.cached_milli,
            ledger_milli: product.ledger_milli,
        };
        repo::set_cached_quantity(conn, shop_id, drift.product_id, drift.ledger_milli)?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_STOCK_DRIFT,
                entity: "product",
                entity_id: Some(drift.product_id),
                before: Some(
                    serde_json::json!({ "qty_on_hand_milli": drift.cached_milli }).to_string(),
                ),
                after: Some(
                    serde_json::json!({
                        "day": day,
                        "name": drift.name,
                        "qty_on_hand_milli": drift.ledger_milli,
                        "difference_milli": drift.difference_milli(),
                    })
                    .to_string(),
                ),
            },
        )?;
        drifts.push(drift);
    }
    jobs::mark_run(conn, shop_id, JOB_STOCK_RECOUNT, day)?;
    Ok(Report {
        day: day.to_string(),
        checked,
        drifts,
    })
}

/// The drift an audit entry holds, or `None` when the entry is not one this
/// run wrote. Everything the panel shows comes out of the entry itself, so a
/// product renamed or removed since does not change what the log says the
/// recount saw.
fn drift_of(entry: &crate::models::audit::AuditEntry, day: &str) -> Option<Drift> {
    let after: serde_json::Value = serde_json::from_str(entry.after.as_deref()?).ok()?;
    let before: serde_json::Value = serde_json::from_str(entry.before.as_deref()?).ok()?;
    if after.get("day").and_then(serde_json::Value::as_str) != Some(day) {
        return None;
    }
    Some(Drift {
        product_id: entry.entity_id?,
        name: after
            .get("name")
            .and_then(serde_json::Value::as_str)?
            .to_string(),
        cached_milli: before.get("qty_on_hand_milli")?.as_i64()?,
        ledger_milli: after.get("qty_on_hand_milli")?.as_i64()?,
    })
}
