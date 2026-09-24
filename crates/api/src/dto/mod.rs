//! The wire types. These structs are the source for
//! `packages/shared/src/generated`; nothing hand-writes them twice.
//!
//! Money crosses as an integer number of centimes in a JSON `number`
//! (architecture.md, contract between Rust and TypeScript). ts-rs would
//! call an `i64` a `bigint`, so the exporter in `tests/export_bindings.rs`
//! configures large ints as `number`: centimes are safe below 2^53, which
//! is 90 trillion dinars. The bound is enforced at this edge, not only
//! written down: an amount beyond it would round silently in JavaScript.
//!
//! One file per domain, matching `routes/`. Everything is re-exported here, so
//! no import outside this folder names a domain file: `crate::dto::SaleDto` is
//! what it always was. The split exists because this was one 3071-line file that
//! every feature appended to, which made it the first place two branches
//! collided.

use crate::error::ApiError;
use chrono::NaiveDate;
use dzpos_core::error::CoreError;
use dzpos_core::lang::Lang;
#[cfg(feature = "retail")]
use dzpos_core::models::category::Category;
#[cfg(feature = "retail")]
use dzpos_core::models::document::{Document, DocumentKind, DocumentLine, DocumentStatus};
#[cfg(feature = "retail")]
use dzpos_core::models::product::{Contenance, ContenanceUnit, NewProduct, Product, Unit};
use dzpos_core::models::shop::{Shop, StoreBlock};
#[cfg(feature = "retail")]
use dzpos_core::models::stock::Drift;
#[cfg(feature = "retail")]
use dzpos_core::money::{Bps, PaymentMode, TvaLine};
use dzpos_core::money::{Money, Regime};
use dzpos_core::print::{FactureLayout, ThermalMode};
#[cfg(feature = "retail")]
use dzpos_core::services::avoir::AvoirLine;
use dzpos_core::services::backup::Backup;
#[cfg(feature = "retail")]
use dzpos_core::services::cancellation::CancelEffect;
#[cfg(feature = "retail")]
use dzpos_core::services::cash::{CashPosition, Outgoings, Takings};
#[cfg(feature = "retail")]
use dzpos_core::services::cash_refunds::Refund;
use dzpos_core::services::clock::Month;
#[cfg(feature = "retail")]
use dzpos_core::services::customers::{CustomerWithBalance, NewCustomer, PartyKind};
#[cfg(feature = "retail")]
use dzpos_core::services::dashboard::{
    Dashboard, Figures, LowStock, Owed, Series, SeriesPoint, TopProduct,
};
#[cfg(feature = "retail")]
use dzpos_core::services::debt::{DebtKind, LedgerLine, Payment, PaymentMethod};
#[cfg(feature = "retail")]
use dzpos_core::services::expenses::{Expense, ExpenseCategory, NewExpense};
#[cfg(feature = "retail")]
use dzpos_core::services::import::{Applied, DryRun, Outcome, RowReport};
use dzpos_core::services::permissions::{Permission, Role};
use dzpos_core::services::preferences::Theme;
#[cfg(feature = "retail")]
use dzpos_core::services::purchases::{
    NewLine, NewPurchase, Paid, Purchase, PurchaseLine, PurchaseStatus, PurchaseView, ReceiveLine,
};
#[cfg(feature = "retail")]
use dzpos_core::services::sales::{NewSale, NewSaleLine, Sale, SaleKind, Warning};
use dzpos_core::services::settings::DatedRegime;
#[cfg(feature = "retail")]
use dzpos_core::services::shifts::{NewShift, Shift, ShiftReport, TillCount};
#[cfg(feature = "retail")]
use dzpos_core::services::stock::{LastRecount, Report};
#[cfg(feature = "retail")]
use dzpos_core::services::supplier_debt::SupplierDebtKind;
#[cfg(feature = "retail")]
use dzpos_core::services::suppliers::{NewSupplier, SupplierWithBalance};
use dzpos_core::services::users::User;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// The clinic's appointment book (C5 of the clinic plan).
#[cfg(feature = "clinic")]
mod appointments;
mod audit;
mod auth;
mod backups;
// The book tools (C5b of the clinic plan), beside the book.
#[cfg(feature = "clinic")]
mod book_tools;
// The wholly-retail DTO files (S5): each names only types `crates/retail`
// owns, so the module has nothing to export once the feature is off.
// `settings` stays unconditional: it is mostly kernel settings, with one
// retail-only field and one retail-only DTO gated inside it.
#[cfg(feature = "retail")]
mod categories;
mod common;
#[cfg(feature = "retail")]
mod customers;
#[cfg(feature = "retail")]
mod dashboard;
#[cfg(feature = "retail")]
mod expenses;
#[cfg(feature = "retail")]
mod import;
mod meta;
mod pairing;
// The clinic's patient file (C3 of the clinic plan), gated on its own
// feature the way the wholly-retail files above are on theirs.
#[cfg(feature = "clinic")]
mod patients;
#[cfg(feature = "retail")]
mod products;
#[cfg(feature = "retail")]
mod purchases;
// The clinic's waiting queue (C4 of the clinic plan), beside the patient file.
#[cfg(feature = "clinic")]
mod queue;
#[cfg(feature = "retail")]
mod sales;
mod settings;
#[cfg(feature = "retail")]
mod stock;
#[cfg(feature = "retail")]
mod suppliers;
#[cfg(feature = "retail")]
mod till;
mod users;

#[cfg(feature = "clinic")]
pub use appointments::*;
pub use audit::*;
pub use auth::*;
pub use backups::*;
#[cfg(feature = "clinic")]
pub use book_tools::*;
#[cfg(feature = "retail")]
pub use categories::*;
pub use common::*;
#[cfg(feature = "retail")]
pub use customers::*;
#[cfg(feature = "retail")]
pub use dashboard::*;
#[cfg(feature = "retail")]
pub use expenses::*;
#[cfg(feature = "retail")]
pub use import::*;
pub use meta::*;
pub use pairing::*;
#[cfg(feature = "clinic")]
pub use patients::*;
#[cfg(feature = "retail")]
pub use products::*;
#[cfg(feature = "retail")]
pub use purchases::*;
#[cfg(feature = "clinic")]
pub use queue::*;
#[cfg(feature = "retail")]
pub use sales::*;
pub use settings::*;
#[cfg(feature = "retail")]
pub use stock::*;
#[cfg(feature = "retail")]
pub use suppliers::*;
#[cfg(feature = "retail")]
pub use till::*;
pub use users::*;
