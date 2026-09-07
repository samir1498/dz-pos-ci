use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::suppliers)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Supplier {
    pub id: i32,
    pub name: String,
    pub phone: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = crate::schema::suppliers)]
pub struct NewSupplier {
    pub name: String,
    pub phone: Option<String>,
    pub notes: Option<String>,
}
