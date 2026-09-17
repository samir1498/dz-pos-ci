//! Stock recounts and the drift a recount found.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// One product the recount put right: what the column said it had, what its
/// movements add up to, and the difference between them. The name travels
/// with the id because the panel is read by a person and it is what the log
/// stored, so a past run reads the same after a rename.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "StockDriftDto.ts")]
pub struct StockDriftDto {
    pub product_id: i32,
    pub name: String,
    pub cached_milli: i64,
    pub ledger_milli: i64,
    pub difference_milli: i64,
}

impl From<Drift> for StockDriftDto {
    fn from(d: Drift) -> Self {
        StockDriftDto {
            product_id: d.product_id,
            difference_milli: d.difference_milli(),
            name: d.name,
            cached_milli: d.cached_milli,
            ledger_milli: d.ledger_milli,
        }
    }
}

/// What one run of the recount found and did. `products_checked` is there so
/// an empty drift list reads as "nothing is wrong" rather than as "nothing
/// was looked at".
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "StockRecountDto.ts")]
pub struct StockRecountDto {
    /// The day on the shop's calendar the run was marked under.
    pub day: String,
    pub products_checked: i64,
    pub drifts: Vec<StockDriftDto>,
}

impl From<Report> for StockRecountDto {
    fn from(r: Report) -> Self {
        StockRecountDto {
            day: r.day,
            // A count of this shop's products. The bound is unreachable on
            // any file a shop could have, and a number that is merely
            // bounded beats a refusal on a screen that is only reporting.
            products_checked: i64::try_from(r.checked).unwrap_or(i64::MAX),
            drifts: r.drifts.into_iter().map(StockDriftDto::from).collect(),
        }
    }
}

/// The last run as the file remembers it. `last_run_day` is null when the
/// shop has never recounted, which is what a file opened for the first time
/// says before the daily loop has woken once.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "LastStockRecountDto.ts")]
pub struct LastStockRecountDto {
    pub last_run_day: Option<String>,
    pub drifts: Vec<StockDriftDto>,
}

impl From<LastRecount> for LastStockRecountDto {
    fn from(l: LastRecount) -> Self {
        LastStockRecountDto {
            last_run_day: l.last_run_day,
            drifts: l.drifts.into_iter().map(StockDriftDto::from).collect(),
        }
    }
}
