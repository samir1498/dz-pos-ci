//! The shop itself: the seller block every ticket and facture prints
//! (features.md §3, seller block). One row per shop; the process is
//! started with its id (architecture.md rule 3).

use diesel::prelude::*;

use crate::schema::shops;

/// The shop as the rest of the app sees it. Every identifier is optional
/// on the row: a shop under IFU prints a ticket with its name alone, and
/// the facture rule is what demands RC and NIS at issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shop {
    pub id: i32,
    pub name: String,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
}

/// The store block as a caller writes it. Blank strings are stored as
/// nothing (the service trims and empties them), so a cleared field on the
/// screen clears the column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreBlock {
    pub name: String,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = shops, treat_none_as_null = true)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct ShopRow {
    pub id: i32,
    pub name: String,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
}

/// The whole block again on every write. `treat_none_as_null` so a field
/// the caller emptied is written as NULL rather than skipped; without it an
/// identifier could never be cleared (the product row learned this first).
#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = shops, treat_none_as_null = true)]
pub(crate) struct ShopRowWrite {
    pub name: String,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
}

impl From<ShopRow> for Shop {
    fn from(r: ShopRow) -> Self {
        Shop {
            id: r.id,
            name: r.name,
            rc: r.rc,
            nif: r.nif,
            nis: r.nis,
            ai: r.ai,
            address: r.address,
            phone: r.phone,
        }
    }
}
