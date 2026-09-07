use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::expense_categories)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ExpenseCategory {
    pub id: i32,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = crate::schema::expense_categories)]
pub struct NewExpenseCategory {
    pub name: String,
}
