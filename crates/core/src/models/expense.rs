//! Money out that is not stock (features.md §1, Expense), and what it is
//! filed under.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;

use crate::money::Money;
use crate::schema::{expense_categories, expenses};

/// What an expense is filed under. The row carries an i18n key and not a
/// label: the desktop reads the label in the three languages from its own
/// files by that key, so a shop switching language does not rewrite its rows
/// and a category cannot exist in French and be missing in Arabic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpenseCategory {
    pub id: i32,
    pub shop_id: i32,
    pub key: String,
    /// The order the screen lists them in, which is the order the seven
    /// seeded ones were written in and not alphabetical in any one language.
    pub sort_order: i32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewExpenseCategory {
    pub key: String,
    pub sort_order: i32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expense {
    pub id: i32,
    pub shop_id: i32,
    pub category_id: i32,
    pub amount: Money,
    /// A day on the shop's calendar, `YYYY-MM-DD`.
    pub expense_date: String,
    pub note: Option<String>,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
}

/// An expense as a caller asks for it. The day is a `NaiveDate` and not the
/// text the column holds: a day that is not a day is then refused by the type
/// at the edge that parsed it, and the service has one fewer thing to
/// validate. The user is not here; it comes from the caller's identity, the
/// way it does on every other write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewExpense {
    pub category_id: i32,
    pub amount: Money,
    pub expense_date: NaiveDate,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = expense_categories)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct ExpenseCategoryRow {
    pub id: i32,
    pub shop_id: i32,
    pub key: String,
    pub sort_order: i32,
    pub active: bool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = expense_categories)]
pub(crate) struct ExpenseCategoryRowWrite {
    pub shop_id: i32,
    pub key: String,
    pub sort_order: i32,
    pub active: bool,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = expenses)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct ExpenseRow {
    pub id: i32,
    pub shop_id: i32,
    pub category_id: i32,
    pub amount_centimes: i64,
    pub expense_date: String,
    pub note: Option<String>,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = expenses)]
pub(crate) struct ExpenseRowWrite {
    pub shop_id: i32,
    pub category_id: i32,
    pub amount_centimes: i64,
    pub expense_date: String,
    pub note: Option<String>,
    pub user_id: i32,
}

impl From<ExpenseCategoryRow> for ExpenseCategory {
    fn from(r: ExpenseCategoryRow) -> Self {
        ExpenseCategory {
            id: r.id,
            shop_id: r.shop_id,
            key: r.key,
            sort_order: r.sort_order,
            active: r.active,
        }
    }
}

impl From<ExpenseRow> for Expense {
    fn from(r: ExpenseRow) -> Self {
        Expense {
            id: r.id,
            shop_id: r.shop_id,
            category_id: r.category_id,
            amount: Money::centimes(r.amount_centimes),
            expense_date: r.expense_date,
            note: r.note,
            user_id: r.user_id,
            created_at: r.created_at,
        }
    }
}
