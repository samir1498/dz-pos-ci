// Every test binary compiles this whole file and calls part of it, so the
// helpers a given binary has no use for are dead code from where it stands.
#![allow(dead_code)]

//! What the tests in this directory need to name a document. These tests
//! issue through the till, which stamps a document with the shop clock, so
//! the year in a series and in a printed number is read rather than written
//! out: a literal `2026` would go red on 1 January and say nothing about the
//! rule it was pinning.

/// This moment's year on the shop's calendar (`services::clock`, UTC+1).
fn shop_year() -> String {
    dzpos_core::services::clock::now().format("%Y").to_string()
}

/// The counter key a document of this kind is numbered in right now:
/// `doc_facture:2026`. The series carries the year it counts in
/// (features.md §4, Numbering).
pub fn series_of(kind: &str) -> String {
    format!("{kind}:{}", shop_year())
}

/// The same year in the number a customer quotes: `FA-2026-000001`.
pub fn printed(prefix: &str, number: i64) -> String {
    format!("{prefix}-{}-{number:06}", shop_year())
}
