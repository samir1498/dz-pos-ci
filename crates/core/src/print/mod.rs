//! Printed documents. The core renders them, not the UI, so the desktop
//! and a server with no screen print the same bytes (features.md §4).
//!
//! Each template is one file under `crates/core/templates/`, compiled into
//! the binary by askama, and each template × language is pinned by a golden
//! file under `fixtures/print/`. A template change is a reviewed golden
//! diff, never a green run nobody read.

pub mod strings;
pub mod ticket;

pub use ticket::render_ticket;
