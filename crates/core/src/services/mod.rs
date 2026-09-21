//! Business rules. Every caller (HTTP, Tauri, tests) enters here.

use crate::error::CoreError;

/// Longest value any one printed field keeps. An RC is 20-odd characters and
/// an address a few lines; anything past this is a paste gone wrong, and the
/// ticket and facture templates would wrap it into nonsense.
///
/// The seller block and the buyer block print side by side on the same paper,
/// so they hold each other to the same bound from one place.
pub(crate) const MAX_FIELD_CHARS: usize = 200;

/// Trimmed, and blank becomes `None`: the column is cleared, never left
/// holding a space.
pub(crate) fn optional_field(
    field: &str,
    value: Option<&str>,
) -> Result<Option<String>, CoreError> {
    let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    bounded_field(field, value)?;
    Ok(Some(value.to_string()))
}

pub(crate) fn bounded_field(field: &str, value: &str) -> Result<(), CoreError> {
    if value.chars().count() > MAX_FIELD_CHARS {
        return Err(CoreError::validation(
            field,
            "longer than a ticket or a facture can print",
        ));
    }
    Ok(())
}

pub mod audit;
pub mod avoir;
pub mod backup;
pub mod cancellation;
pub mod cash;
pub mod categories;
pub mod clock;
pub mod customers;
pub mod dashboard;
pub mod debt;
pub mod documents;
pub mod expenses;
pub mod export;
pub mod import;
pub mod pairing;
pub mod permissions;
pub mod preferences;
pub mod pricing;
pub mod products;
pub mod proforma;
pub mod purchases;
pub mod sales;
pub mod seed;
pub mod sessions;
pub mod settings;
pub mod shifts;
pub mod shops;
pub mod stock;
pub mod supplier_debt;
pub mod suppliers;
pub mod support_bundle;
pub mod users;
