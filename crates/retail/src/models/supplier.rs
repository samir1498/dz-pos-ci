//! The supplier fiche (features.md §1, Supplier). Who the shop buys from,
//! and the party a supplier debt belongs to.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::schema::suppliers;

/// A supplier as the rest of the app sees it.
///
/// The opening debt features.md lists among the fields is not here, for the
/// reason a customer's is not: it is the first `opening` row of the supplier
/// ledger, so the balance has one source and a correction to it is a movement
/// somebody can read.
///
/// There is no credit limit and no warning threshold either. Those are what
/// the shop grants somebody; here the shop is the one being granted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Supplier {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// The fields a caller hands over, on a create and on an update alike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSupplier {
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = suppliers)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct SupplierRow {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// `updated_at` is set on every write, the way a customer's is: SQLite's
/// DEFAULT only fires on the insert.
///
/// `treat_none_as_null`: an update carries the whole fiche, so an RC somebody
/// cleared has to reach the column. Without it diesel reads `None` as "leave
/// this one alone" and a field could never be emptied.
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = suppliers, treat_none_as_null = true)]
pub(crate) struct SupplierRowWrite {
    pub shop_id: i32,
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    pub updated_at: NaiveDateTime,
}

impl From<SupplierRow> for Supplier {
    fn from(r: SupplierRow) -> Self {
        Supplier {
            id: r.id,
            shop_id: r.shop_id,
            name: r.name,
            phone: r.phone,
            address: r.address,
            rc: r.rc,
            nif: r.nif,
            nis: r.nis,
            ai: r.ai,
            notes: r.notes,
            active: r.active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
