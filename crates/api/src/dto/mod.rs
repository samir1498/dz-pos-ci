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
use dzpos_core::models::category::Category;
use dzpos_core::models::document::{Document, DocumentKind, DocumentLine, DocumentStatus};
use dzpos_core::models::product::{NewProduct, Product, Unit};
use dzpos_core::models::shop::{Shop, StoreBlock};
use dzpos_core::models::stock::Drift;
use dzpos_core::money::{Bps, Money, PaymentMode, Regime, TvaLine};
use dzpos_core::print::FactureLayout;
use dzpos_core::services::avoir::AvoirLine;
use dzpos_core::services::backup::Backup;
use dzpos_core::services::cancellation::CancelEffect;
use dzpos_core::services::cash::{CashPosition, Outgoings, Takings};
use dzpos_core::services::clock::Month;
use dzpos_core::services::customers::{CustomerWithBalance, NewCustomer, PartyKind};
use dzpos_core::services::dashboard::{
    Dashboard, Figures, LowStock, Owed, Series, SeriesPoint, TopProduct,
};
use dzpos_core::services::debt::{DebtAllocation, DebtKind, LedgerLine, Payment, PaymentMethod};
use dzpos_core::services::expenses::{Expense, ExpenseCategory, NewExpense};
use dzpos_core::services::import::{Applied, DryRun, Outcome, RowReport};
use dzpos_core::services::permissions::{Permission, Role};
use dzpos_core::services::preferences::Theme;
use dzpos_core::services::purchases::{
    NewLine, NewPurchase, Paid, Purchase, PurchaseLine, PurchaseStatus, PurchaseView, ReceiveLine,
};
use dzpos_core::services::sales::{NewSale, NewSaleLine, Sale, SaleKind, Warning};
use dzpos_core::services::settings::DatedRegime;
use dzpos_core::services::stock::{LastRecount, Report};
use dzpos_core::services::supplier_debt::{SupplierAllocation, SupplierDebtKind};
use dzpos_core::services::suppliers::{NewSupplier, SupplierWithBalance};
use dzpos_core::services::users::User;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

mod audit;
mod auth;
mod backups;
mod categories;
mod common;
mod customers;
mod dashboard;
mod expenses;
mod import;
mod meta;
mod pairing;
mod products;
mod purchases;
mod sales;
mod settings;
mod stock;
mod suppliers;
mod users;

pub use audit::*;
pub use auth::*;
pub use backups::*;
pub use categories::*;
pub use common::*;
pub use customers::*;
pub use dashboard::*;
pub use expenses::*;
pub use import::*;
pub use meta::*;
pub use pairing::*;
pub use products::*;
pub use purchases::*;
pub use sales::*;
pub use settings::*;
pub use stock::*;
pub use suppliers::*;
pub use users::*;
