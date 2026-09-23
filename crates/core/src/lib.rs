//! Domain core: models, repos, services. Every caller (Tauri, HTTP, tests)
//! goes through `services`; nothing outside this crate touches diesel.
//!
//! Since the kernel crate split (S3 of
//! `a-kernel-crate-and-retail-as-the-first-module`), this crate holds no
//! source of its own: `crates/kernel` is the eleven domain-free services,
//! the money module, the error enum and the storage-side enums, and
//! `crates/retail` is the twenty-five retail services and the print engine.
//! This is a thin re-export facade so `crates/api`, the Tauri app and this
//! crate's own 59 test files under `tests/` keep reading `dzpos_core::x`
//! without every one of them being rewritten in the same change that moved
//! the source; `crates/core/tests/services_go_through_services.rs` still
//! walks the two crates' source directly rather than through this facade.
//!
//! S5 gives this facade its own `retail` feature (`default = ["retail"]`),
//! forwarded from `crates/api`'s feature of the same name: with it off,
//! `dzpos-retail` is not even a dependency of the build, and every module
//! below that merged its two crates' names instead re-exports the kernel's
//! alone.

pub use dzpos_kernel::{build_info, db, lang, log, money, schema};

#[cfg(feature = "retail")]
pub mod audit_actions {
    //! The shop's own `ACTION_*` tags for `dzpos_core::services::audit`,
    //! moved out of the kernel's copy of that file by S4 of
    //! `a-kernel-crate-and-retail-as-the-first-module`. A plain re-export,
    //! the same shape as `print` below: the kernel's own `ACTION_*`
    //! constants stay reachable at `dzpos_core::services::audit::ACTION_X`
    //! through `services` below, unchanged. Retail-only (S5): a kernel-only
    //! build never records one of these actions.
    pub use dzpos_retail::audit_actions::*;
}

pub mod error {
    //! The kernel's `CoreError` and, with the `retail` feature, the
    //! retail's `RetailError` merged the way `models` is below:
    //! `dzpos_retail::error` already re-exports the kernel's `CoreError`
    //! itself (S4 of `a-kernel-crate-and-retail-as-the-first-module` moved
    //! eight variants out of it), so importing retail's alone carries both
    //! without a name colliding. Without the feature there is only the one
    //! enum to carry.
    #[cfg(not(feature = "retail"))]
    pub use dzpos_kernel::error::*;
    #[cfg(feature = "retail")]
    pub use dzpos_retail::error::*;
}

pub mod models {
    //! The kernel's six domain-free models and, with the `retail` feature,
    //! the retail's fourteen, merged: `dzpos_retail::models` already
    //! re-exports the kernel's six itself (a retail file's
    //! `crate::models::sql_types::Role` has to resolve inside that crate
    //! too), so importing both here would double every kernel name.
    //! Importing retail's alone carries both. Without the feature there is
    //! only the kernel's own six.
    #[cfg(not(feature = "retail"))]
    pub use dzpos_kernel::models::*;
    #[cfg(feature = "retail")]
    pub use dzpos_retail::models::*;
}

pub mod services {
    //! The kernel's eleven domain-free services and, with the `retail`
    //! feature, the retail's twenty-five, merged. No name collides: a
    //! service module is owned by exactly one of the two crates.
    pub use dzpos_kernel::services::*;
    #[cfg(feature = "retail")]
    pub use dzpos_retail::services::*;
}

pub mod print {
    //! The print engine. `dzpos_retail::print` already re-exports the
    //! kernel's `layout` and `thermal` (a shop's chosen paper size and
    //! thermal path are domain-free), so this crate needs only the one with
    //! the feature on; without it, `layout` and `thermal` come from the
    //! kernel directly and there is no paper a retail document draws.
    #[cfg(not(feature = "retail"))]
    pub use dzpos_kernel::print::*;
    #[cfg(feature = "retail")]
    pub use dzpos_retail::print::*;
}

#[cfg(feature = "retail")]
pub mod shop_counts {
    //! The shop's own backup and support-bundle counts, moved out of
    //! `dzpos_core::services::backup` and
    //! `dzpos_core::services::support_bundle` by S4 of
    //! `a-kernel-crate-and-retail-as-the-first-module`. A plain re-export,
    //! the same shape as `audit_actions` above. Retail-only (S5): a
    //! kernel-only build has no shop counts to add to either bundle.
    pub use dzpos_retail::shop_counts::*;
}
