//! The only modules that run diesel queries. Every query takes a `shop_id`
//! (rule 3), the same as the kernel's and the shop's.
pub mod absence_blocks;
pub mod appointments;
pub mod patients;
pub mod queue;
pub mod visit_types;
pub mod working_hours;

/// What was typed into a search box, as a LIKE pattern that matches it
/// anywhere, with SQLite's two wildcards and the escape character escaped
/// into characters to match. The same rule `dzpos_retail::repos` applies to
/// its own boxes, written again here because that one is crate-internal to
/// the shop and this crate does not depend on it.
pub(crate) fn contains_pattern(text: &str) -> String {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}
