//! The three print files every trade shares: a shop's chosen paper size and
//! its chosen thermal path, both stored by `services::preferences`, and the
//! printed word list `print::strings` carries in three languages. The
//! boundary walk (`crates/core/tests/services_go_through_services.rs`,
//! `the_shared_kernel_does_not_name_the_shop`) used to pin this file with
//! fourteen shop words on purpose, for S4 to split out; S4 moved the
//! twenty-four `Key` variants those words sat on to
//! `dzpos_retail::print::strings::ShopKey`, so `print::strings` carries none
//! of the fourteen any more and the row for this file is off the walk's
//! list entirely. The rest of the print engine (the module tree, the escpos
//! byte format, the papers themselves) moved to `dzpos-retail` in the kernel
//! crate split (S3 of `a-kernel-crate-and-retail-as-the-first-module`):
//! `print/mod.rs` and `print/escpos.rs` both name `models::document::
//! Document` on a code line, and `print/raster.rs` reaches into
//! `print/ticket.rs` for `Align`, `Item` and `WIDTH`, so neither of the two
//! can live in a crate that depends on nothing. `print/strings.rs` names no
//! such type; its only import is `crate::lang::Lang`.
pub mod layout;
pub mod strings;
pub mod thermal;

pub use layout::{FactureLayout, Page, Paper};
pub use thermal::ThermalMode;
