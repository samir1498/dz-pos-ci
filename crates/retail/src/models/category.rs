//! A category and the TVA rate a product inherits from it when it names no
//! rate of its own. The rate is a `Bps`, so a row outside 0..=10 000 bps
//! becomes an error here rather than a rate nobody can compute with.

use diesel::prelude::*;

use crate::error::CoreError;
use crate::money::Bps;
use crate::schema::categories;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub default_rate_bps: Bps,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = categories)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct CategoryRow {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub default_rate_bps: i32,
}

/// A category as it goes into the file. Only the Excel import writes one:
/// there is no categories screen yet, and a product import that named a
/// category nobody had opened would otherwise refuse the whole file
/// (features.md §1).
#[derive(Debug, Insertable)]
#[diesel(table_name = categories)]
pub(crate) struct CategoryRowWrite {
    pub shop_id: i32,
    pub name: String,
    pub default_rate_bps: i32,
}

impl TryFrom<CategoryRow> for Category {
    type Error = CoreError;

    fn try_from(r: CategoryRow) -> Result<Self, CoreError> {
        let rate = u32::try_from(r.default_rate_bps)
            .ok()
            .map(Bps::new)
            .transpose()?
            .ok_or(crate::money::MoneyError::RateOutOfRange)?;
        Ok(Category {
            id: r.id,
            shop_id: r.shop_id,
            name: r.name,
            default_rate_bps: rate,
        })
    }
}
