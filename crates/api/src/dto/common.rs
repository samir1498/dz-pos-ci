//! Helpers every domain's wire types use: the JavaScript safe-integer bound
//! every amount is checked against, the date formats, and the error payload.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

pub(crate) const fn yes() -> bool {
    true
}

/// The largest integer a JSON `number` carries without loss
/// (`Number.MAX_SAFE_INTEGER`). Anything past it would be rounded by every
/// JavaScript caller, so the API refuses it as a request error.
pub const MAX_SAFE_INTEGER: i64 = (1 << 53) - 1;

pub(crate) fn within_js_safe_range(field: &'static str, value: i64) -> Result<i64, ApiError> {
    if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value) {
        return Err(ApiError::Request(CoreError::validation(
            field,
            "beyond what a JSON number carries without loss (2^53 - 1)",
        )));
    }
    Ok(value)
}

/// The shape every failure takes. Generated so the client can narrow on
/// `code` without repeating the string list.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "ApiErrorDto.ts")]
pub struct ApiErrorDto {
    pub error: ApiErrorPayloadDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "ApiErrorPayloadDto.ts")]
pub struct ApiErrorPayloadDto {
    pub code: String,
    pub message: String,
    /// Only on `credit_limit`: what the customer would owe once this sale
    /// landed, and the limit that refused it. Absent from every other error,
    /// so the till reads them as optional and never as a zero somebody meant
    /// (crates/api/src/error.rs writes them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub balance_after_centimes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub credit_limit_centimes: Option<i64>,
    /// The field of the request a refusal is about, when it is about one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub field: Option<String>,
    /// Only on a payment refused for being more than the debt: what the
    /// customer actually owes. "Too much" is useless without the amount that
    /// would not have been.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub outstanding_centimes: Option<i64>,
    /// Only on `party_ids`: which half of the facture is short (`seller` or
    /// `buyer`) and which identifiers it is short of (`rc`, `nis`, `name`,
    /// `address`). The till sends the cashier to the settings or to the
    /// fiche on the side, and names the fields from the list; neither is
    /// re-derived from the code (architecture.md rule 2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub party_side: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub missing_ids: Option<Vec<String>>,
    /// Only on `locked_out`: how long the user has to wait before the till
    /// will look at their PIN again. The sign-in screen counts it down.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub retry_after_seconds: Option<i64>,
    /// Only on `forbidden`: the permission the route wanted, spelled the way
    /// `PermissionDto` spells it. The screen says which thing this role may
    /// not do without working it out from the route (M4 T2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub permission: Option<PermissionDto>,
}

/// An optional amount on the wire, checked against the safe-integer bound
/// like every other one: `None` stays `None`, which is the field left empty.
pub fn money_field(field: &'static str, value: Option<i64>) -> Result<Option<Money>, ApiError> {
    value
        .map(|c| within_js_safe_range(field, c))
        .transpose()
        .map(|c| c.map(Money::centimes))
}

pub const DATE_FORMAT: &str = "%Y-%m-%d";

/// A stored timestamp, the shape every TEXT timestamp column holds.
pub const DATE_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// A time on the wire: a day, `T`, and a clock. Seconds, never fractions;
/// the backup name is only that precise.
pub const STAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// `YYYY-MM-DD` and nothing else: "2026-1-5", a time, or a month 13 are the
/// caller's mistake and answer 422 naming the field.
pub fn parse_day(field: &'static str, text: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(text, DATE_FORMAT)
        .ok()
        .filter(|d| d.format(DATE_FORMAT).to_string() == text)
        .ok_or_else(|| {
            ApiError::Request(CoreError::validation(field, "a day is written YYYY-MM-DD"))
        })
}

/// `YYYY-MM` and nothing else: a month is the range a figure is asked over,
/// and "2026-9" or a day would be answered for another one.
///
/// The month is written back out and compared with what came in, the way
/// `parse_day` compares its day. Rust's integer parser takes a sign, so
/// "+026-09" is four characters of year that read as 26 and "2026-+9" two of
/// month that read as 9: both pass every check on shape and on length, and
/// only the round trip catches them. A figure answered for the year 26 is one
/// nobody would think to doubt.
pub fn parse_month(field: &'static str, text: &str) -> Result<Month, ApiError> {
    let refuse = || ApiError::Request(CoreError::validation(field, "a month is written YYYY-MM"));
    let (year, month) = text.split_once('-').ok_or_else(refuse)?;
    let year: i32 = year.parse().map_err(|_| refuse())?;
    let month: u32 = month.parse().map_err(|_| refuse())?;
    let parsed = Month::new(year, month).map_err(ApiError::Request)?;
    if parsed.as_text() != text {
        return Err(refuse());
    }
    Ok(parsed)
}
