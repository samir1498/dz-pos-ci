//! The second module
//! (`context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md`),
//! beside `dzpos-retail` on the kernel: a cabinet's patient file, a
//! waiting-room queue in arrival order, and a plain appointment book for one
//! doctor. No money (no fee, no receipt, no fund as payer) and no printed
//! paper. Ids are a text UUID v7 from the first migration, unlike retail's
//! integer ids, because the cloud relay and the patient app both need one
//! unique across machines. Depends on `dzpos-kernel` only.
//!
//! C2 is the crate itself, empty: the tables, the repos, the services and
//! the two permissions (`ViewPatients`, `EditPatients`) arrive in C3. This
//! module carries one public item so a build that links this crate in
//! (`crates/core`'s `clinic` feature) has something to prove it: the
//! crate's own name, read back through `env!` rather than hand-typed, so it
//! can never drift from `Cargo.toml`.

/// This crate's package name, for the one test in `tests/` that proves a
/// clinic-feature build actually links `dzpos-clinic` in.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
