//! Business rules for the clinic. Every caller (HTTP, Tauri, tests) enters
//! here; the shared helpers (`optional_field`, `bounded_field`, the clock,
//! the audit service) are the kernel's.
pub mod absence_blocks;
pub mod appointments;
pub mod day_list;
pub mod free_slot;
pub mod patients;
pub mod queue;
pub mod slot_length;
pub mod visit_types;
pub mod working_hours;
