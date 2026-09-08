//! Storage-side types. `money/mod.rs` stays free of diesel, so the
//! centimes conversion for `Money` lives in `product.rs` next to the row it
//! maps. `Unit` is defined here and carries its own diesel derives.

use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;

/// Unit of measure. The products CHECK constraint allows exactly these.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    AsExpression,
    FromSqlRow,
)]
#[diesel(sql_type = Text)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Piece,
    Kg,
    Litre,
    Box,
}

impl Unit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Unit::Piece => "piece",
            Unit::Kg => "kg",
            Unit::Litre => "litre",
            Unit::Box => "box",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "piece" => Some(Unit::Piece),
            "kg" => Some(Unit::Kg),
            "litre" => Some(Unit::Litre),
            "box" => Some(Unit::Box),
            _ => None,
        }
    }
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromSql<Text, Sqlite> for Unit {
    fn from_sql(
        value: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let raw = <String as FromSql<Text, Sqlite>>::from_sql(value)?;
        Unit::parse(&raw).ok_or_else(|| format!("unknown unit of measure: {raw}").into())
    }
}

impl ToSql<Text, Sqlite> for Unit {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.as_str().to_string());
        Ok(serialize::IsNull::No)
    }
}
