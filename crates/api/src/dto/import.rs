//! A dry run of an import and what applying it did.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// What the import would do with one row of the file, flattened for the
/// wire: three words rather than a tagged union, with the field and the
/// reason beside them.
///
/// `field` and `reason` are the core's stable keys, not sentences: the
/// screen translates them, the same way it translates an error code
/// (architecture.md, error policy). A row that is created or updated
/// carries neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "ImportOutcomeDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum ImportOutcomeDto {
    Created,
    Updated,
    Refused,
}

/// One line of the dry run, named the way a person reading the spreadsheet
/// beside it would: the row number the spreadsheet shows, header counted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "ImportRowDto.ts")]
pub struct ImportRowDto {
    pub row: u32,
    pub name: String,
    pub outcome: ImportOutcomeDto,
    /// The column the refusal is about, and why. Null on a row that stands.
    pub field: Option<String>,
    pub reason: Option<String>,
}

impl From<&RowReport> for ImportRowDto {
    fn from(r: &RowReport) -> Self {
        let (outcome, field, reason) = match r.outcome {
            Outcome::Created => (ImportOutcomeDto::Created, None, None),
            Outcome::Updated => (ImportOutcomeDto::Updated, None, None),
            Outcome::Refused { field, reason } => (
                ImportOutcomeDto::Refused,
                Some(field.to_owned()),
                Some(reason.to_owned()),
            ),
        };
        ImportRowDto {
            row: r.row,
            name: r.name.clone(),
            outcome,
            field,
            reason,
        }
    }
}

/// The whole dry run: every row with its verdict, and the two counts the
/// screen puts above the table. Nothing was written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "ImportDryRunDto.ts")]
pub struct ImportDryRunDto {
    pub rows: Vec<ImportRowDto>,
    pub accepted: i64,
    pub refused: i64,
}

impl From<DryRun> for ImportDryRunDto {
    fn from(d: DryRun) -> Self {
        ImportDryRunDto {
            rows: d.rows.iter().map(ImportRowDto::from).collect(),
            accepted: i64::try_from(d.accepted).unwrap_or(i64::MAX),
            refused: i64::try_from(d.refused).unwrap_or(i64::MAX),
        }
    }
}

/// What an apply wrote: the counts the audit row carries, so the screen and
/// the log say the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export_to = "ImportAppliedDto.ts")]
pub struct ImportAppliedDto {
    pub created: i64,
    pub updated: i64,
    pub categories_created: i64,
}

impl From<Applied> for ImportAppliedDto {
    fn from(a: Applied) -> Self {
        ImportAppliedDto {
            created: i64::try_from(a.created).unwrap_or(i64::MAX),
            updated: i64::try_from(a.updated).unwrap_or(i64::MAX),
            categories_created: i64::try_from(a.categories_created).unwrap_or(i64::MAX),
        }
    }
}
