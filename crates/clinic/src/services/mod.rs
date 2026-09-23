//! Business rules for the clinic. Every caller (HTTP, Tauri, tests) enters
//! here; the shared helpers (`optional_field`, `bounded_field`, the clock,
//! the audit service) are the kernel's.
pub mod appointments;
pub mod patients;
pub mod queue;
pub mod slot_length;
