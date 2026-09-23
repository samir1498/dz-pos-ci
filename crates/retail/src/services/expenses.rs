//! Money out that is not stock (features.md §1, Expense): rent, electricity,
//! water, salaries, transport, maintenance and whatever else the shop pays
//! for out of the drawer.
//!
//! An expense is written once and never edited or deleted. The amount
//! stays above zero, so a wrong one is not corrected by a negative twin: the
//! table carries no cancellation block (migration 000008), so until one
//! exists a mistake is a row a comptable reads and asks about.
//!
//! The category is one of the seven the migration seeded per shop. It is
//! stored as an id pointing at a row that carries an i18n key, and the label
//! lives in the desktop's three language files: a shop switching language
//! never rewrites its rows, and a category cannot exist in French and be
//! missing in Arabic.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::audit_actions;
use crate::error::CoreError;
use crate::models::expense::ExpenseRowWrite;
use crate::money::Money;
use crate::repos::expenses as repo;
use dzpos_kernel::services::clock::{self, Month};
use dzpos_kernel::services::{audit, optional_field};

pub use crate::models::expense::{Expense, ExpenseCategory, NewExpense};

/// The day as the column holds it. One format for the two directions, so a
/// row written here and a bound compared against it can never disagree.
const DAY_FORMAT: &str = "%Y-%m-%d";

/// The shop's categories in the order the screen lists them, the retired ones
/// among them. A retired one is still the category older expenses are filed
/// under, so a list that hid it would leave those rows pointing at nothing a
/// reader can name; `create` is where an inactive one is refused.
pub fn categories(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<Vec<ExpenseCategory>, CoreError> {
    repo::categories(conn, shop_id)
}

/// One month's expenses, newest first (the shop's calendar, `clock::Month`).
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    month: Month,
) -> Result<Vec<Expense>, CoreError> {
    let (from, to) = bounds(month);
    repo::list_between(conn, shop_id, &from, &to)
}

/// What the month came to. Summed by the database over the same days the list
/// covers rather than by adding the list up, so the figure the screen shows
/// beside the rows and the figure the cash position subtracts are one query
/// with one filter.
pub fn total(conn: &mut SqliteConnection, shop_id: i32, month: Month) -> Result<Money, CoreError> {
    let (from, to) = bounds(month);
    repo::total_between(conn, shop_id, &from, &to)
}

/// What one day came to, over the same column and the same format the list
/// reads. The dashboard's day column asks for this; the month's asks for
/// `total` above.
pub fn total_on(
    conn: &mut SqliteConnection,
    shop_id: i32,
    day: chrono::NaiveDate,
) -> Result<Money, CoreError> {
    let text = day.format(DAY_FORMAT).to_string();
    repo::total_between(conn, shop_id, &text, &text)
}

/// What a stretch of days, both included, came to. The range version beside
/// `total` (a month) and `total_on` (a day): three functions over one
/// column rather than two plus an exception a sibling service reaches
/// around. Takes `NaiveDate` bounds, like `total_on`, rather than the
/// formatted strings a caller may already hold, so the day format stays
/// this file's own detail and a caller never has to know it to ask this
/// column a question.
pub fn total_between(
    conn: &mut SqliteConnection,
    shop_id: i32,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<Money, CoreError> {
    let (from_text, to_text) = (
        from.format(DAY_FORMAT).to_string(),
        to.format(DAY_FORMAT).to_string(),
    );
    repo::total_between(conn, shop_id, &from_text, &to_text)
}

/// Writes the expense and the audit entry in one transaction: money the shop
/// spent with nobody's name on it is the failure the log exists to prevent
/// (features.md §5).
pub fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    fields: NewExpense,
) -> Result<Expense, CoreError> {
    if fields.amount.as_centimes() <= 0 {
        return Err(CoreError::validation(
            "amount",
            "an expense is money somebody paid, and nobody pays nothing",
        ));
    }
    let note = optional_field("note", fields.note.as_deref())?;
    conn.transaction(|conn| {
        // Read inside the transaction and by the shop's own list: a category
        // id from another shop's file, or one retired between the form
        // opening and the save, must not become a row.
        let category = repo::categories(conn, shop_id)?
            .into_iter()
            .find(|c| c.id == fields.category_id)
            .filter(|c| c.active)
            .ok_or_else(|| {
                CoreError::validation(
                    "category_id",
                    "an expense is filed under one of this shop's categories, and that one is not in use",
                )
            })?;
        let made = repo::insert(
            conn,
            &ExpenseRowWrite {
                shop_id,
                category_id: category.id,
                amount_centimes: fields.amount.as_centimes(),
                expense_date: fields.expense_date.format(DAY_FORMAT).to_string(),
                note: note.clone(),
                user_id,
                // The shop's clock, not the column's own default. SQLite
                // answers CURRENT_TIMESTAMP in UTC and Algiers is an hour
                // ahead all year, so an expense filed at 00:30 was stored as
                // 23:30 the day before. `expense_date` is written by this
                // service and was always right; this is the moment beside it.
                created_at: clock::now(),
            },
        )?;
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit_actions::ACTION_CREATE_EXPENSE,
                entity: "expense",
                entity_id: Some(made.id),
                before: None,
                after: Some(as_json(&made, &category.key)),
            },
        )?;
        Ok(made)
    })
}

/// The expense as the audit log stores it. The category's key travels beside
/// its id because the log is read by a person, and an id is a number they
/// would have to look up in a table the row does not carry.
fn as_json(expense: &Expense, category_key: &str) -> String {
    serde_json::json!({
        "category_id": expense.category_id,
        "category_key": category_key,
        "amount_centimes": expense.amount.as_centimes(),
        "expense_date": expense.expense_date,
        "note": expense.note,
    })
    .to_string()
}

/// The month's first and last day as the column writes them.
fn bounds(month: Month) -> (String, String) {
    (
        month.first_day().format(DAY_FORMAT).to_string(),
        month.last_day().format(DAY_FORMAT).to_string(),
    )
}
