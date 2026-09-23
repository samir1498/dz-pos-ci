//! The second module
//! (`context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`),
//! beside `dzpos-retail` on the kernel: a cabinet's patient file, a
//! waiting-room queue in arrival order, and a plain appointment book for one
//! doctor. No money (no fee, no receipt, no fund as payer) and no printed
//! paper. Ids are a text UUID v7 from the first migration, unlike retail's
//! integer ids, because the cloud relay and the patient app both need one
//! unique across machines. Depends on `dzpos-kernel` only.
//!
//! C3 is the patient file: the `patients` table (declared in `schema` here,
//! migrated from the one shared folder the kernel owns), its model, repo and
//! service, and the audit tags its writes record. The two permissions it
//! asks for (`ViewPatients`, `EditPatients`) sit in the kernel's one
//! `Permission` enum.
//!
//! C4 is the waiting queue: `queue_entries`, one row per arrival, its model,
//! repo and service. It reuses the patient file's two permissions rather
//! than adding its own.
//!
//! C5 is the appointment book: `appointments`, one row per slot given, its
//! model, repo and service, and the slot length as a dated setting
//! (`services::slot_length`). The same two permissions again.
//!
//! Errors are the kernel's `CoreError`: nothing here has a failure of its
//! own to name. A missing patient is `CoreError::NotFoundText`, the text-id
//! twin of `NotFound`, so the API maps it with no third enum.

// A plain constant list, not a service, outside `services/` for the reason
// `dzpos_retail::audit_actions` gives.
pub mod audit_actions;
pub mod models;
// Crate-internal on purpose, same as the kernel's and the shop's: a caller
// goes through `services`, never diesel.
pub(crate) mod repos;
pub mod schema;
pub mod services;

/// This crate's package name, for the test in `tests/` that proves a
/// clinic-feature build actually links `dzpos-clinic` in.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
