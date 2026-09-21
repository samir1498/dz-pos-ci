//! Expenses, their categories, and the cash position they feed.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// What an expense is filed under (features.md §1, Expense). The row carries
/// an i18n key and not a label: the desktop reads the three languages from
/// its own files by that key, so a shop switching language does not rewrite
/// its rows. `active` travels because a retired category still names the
/// expenses filed under it while the form refuses new ones.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ExpenseCategoryDto.ts")]
pub struct ExpenseCategoryDto {
    pub id: i32,
    pub key: String,
    pub sort_order: i32,
    pub active: bool,
}

impl From<ExpenseCategory> for ExpenseCategoryDto {
    fn from(c: ExpenseCategory) -> Self {
        ExpenseCategoryDto {
            id: c.id,
            key: c.key,
            sort_order: c.sort_order,
            active: c.active,
        }
    }
}

/// One expense. The day is `YYYY-MM-DD` on the shop's calendar, which is what
/// the column holds.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ExpenseDto.ts")]
pub struct ExpenseDto {
    pub id: i32,
    pub category_id: i32,
    pub amount_centimes: i64,
    pub expense_date: String,
    pub note: Option<String>,
}

impl From<Expense> for ExpenseDto {
    fn from(e: Expense) -> Self {
        ExpenseDto {
            id: e.id,
            category_id: e.category_id,
            amount_centimes: e.amount.as_centimes(),
            expense_date: e.expense_date,
            note: e.note,
        }
    }
}

/// One month of expenses and what it came to. The total is the core's, summed
/// over the same days the list covers: a screen adding the rows up would be a
/// second answer to the same question.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "ExpensesDto.ts")]
pub struct ExpensesDto {
    /// `YYYY-MM`, as the month was read.
    pub month: String,
    pub total_centimes: i64,
    pub expenses: Vec<ExpenseDto>,
}

/// An expense as the form sends it. The user is not on the wire: it comes
/// from the caller's identity like every other write.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewExpenseDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewExpenseDto {
    pub category_id: i32,
    pub amount_centimes: i64,
    pub expense_date: String,
    #[serde(default)]
    pub note: Option<String>,
}

impl TryFrom<NewExpenseDto> for NewExpense {
    type Error = ApiError;

    fn try_from(d: NewExpenseDto) -> Result<Self, ApiError> {
        Ok(NewExpense {
            category_id: d.category_id,
            amount: Money::centimes(within_js_safe_range("amount_centimes", d.amount_centimes)?),
            expense_date: parse_day("expense_date", &d.expense_date)?,
            note: d.note,
        })
    }
}

/// Money that came in over the period, and what it adds up to. The total
/// travels rather than being added on the screen, for the reason the month's
/// does: one question, one answer.
///
/// `sales_centimes` is what the drawer took, the droit de timbre included.
/// `stamp_centimes` is that tax on its own, a part of the figure above and
/// never a second one to add: a screen showing the shop's own takings
/// subtracts it, and one counting the till does not.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export_to = "TakingsDto.ts")]
pub struct TakingsDto {
    pub sales_centimes: i64,
    pub stamp_centimes: i64,
    pub customer_payments_centimes: i64,
    pub total_centimes: i64,
}

impl TryFrom<Takings> for TakingsDto {
    type Error = ApiError;

    fn try_from(t: Takings) -> Result<Self, ApiError> {
        Ok(TakingsDto {
            sales_centimes: t.sales.as_centimes(),
            stamp_centimes: t.stamp.as_centimes(),
            customer_payments_centimes: t.customer_payments.as_centimes(),
            total_centimes: t.total().map_err(ApiError::from)?.as_centimes(),
        })
    }
}

/// Cash that left over the period.
///
/// `refunds_centimes` is money handed back over the counter on a reversal, on
/// the day the notes changed hands: an avoir against a facture that was paid
/// for, or a cancelled cash sale. A reversal settled on a customer's ledger
/// is not in it, because no notes moved. The sale it reverses stays in
/// `cash_in.sales` on the day it was rung, so a ticket sold and refunded in
/// one month nets to nothing across the two figures rather than being
/// subtracted twice.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[ts(export_to = "OutgoingsDto.ts")]
pub struct OutgoingsDto {
    pub refunds_centimes: i64,
    pub supplier_payments_centimes: i64,
    pub expenses_centimes: i64,
    pub total_centimes: i64,
}

impl TryFrom<Outgoings> for OutgoingsDto {
    type Error = ApiError;

    fn try_from(o: Outgoings) -> Result<Self, ApiError> {
        Ok(OutgoingsDto {
            refunds_centimes: o.refunds.as_centimes(),
            supplier_payments_centimes: o.supplier_payments.as_centimes(),
            expenses_centimes: o.expenses.as_centimes(),
            total_centimes: o.total().map_err(ApiError::from)?.as_centimes(),
        })
    }
}

/// The cash position over a day or a month (features.md §1, Dashboard). Never
/// a stored figure: the core sums the ledgers on every call, and `from` and
/// `to` say which days it read so a screen shows the range it got rather than
/// the one it asked for.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CashPositionDto.ts")]
pub struct CashPositionDto {
    pub from: String,
    pub to: String,
    pub cash_in: TakingsDto,
    pub cash_out: OutgoingsDto,
    pub cash_centimes: i64,
    pub card_in: TakingsDto,
}

impl TryFrom<CashPosition> for CashPositionDto {
    type Error = ApiError;

    fn try_from(p: CashPosition) -> Result<Self, ApiError> {
        Ok(CashPositionDto {
            from: p.from.format(DATE_FORMAT).to_string(),
            to: p.to.format(DATE_FORMAT).to_string(),
            cash_in: TakingsDto::try_from(p.cash_in)?,
            cash_out: OutgoingsDto::try_from(p.cash_out)?,
            cash_centimes: p.cash.as_centimes(),
            card_in: TakingsDto::try_from(p.card_in)?,
        })
    }
}

/// How the money goes back on a reversal (features.md §1, the cash position).
///
/// Here and not beside the two bodies that carry it (`NewAvoirDto` and
/// `CancelDocumentDto` in `sales.rs`), because what it decides is whether a
/// row lands on the cash position this module is about, and `sales.rs` is
/// already at the line limit.
///
/// One variant, and the field that carries it is optional with a serde
/// default, so a body that says nothing settles the way it always did: a
/// credit note goes on the customer's ledger and a cancelled cash sale moves
/// goods alone. `cash` is the caller's statement that notes came out of the
/// drawer, which nothing on the server can work out on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[ts(export_to = "RefundDto.ts")]
#[serde(rename_all = "snake_case")]
pub enum RefundDto {
    Cash,
}

impl RefundDto {
    /// `None` when the field was absent, which is the ledger path.
    pub fn refund(value: Option<Self>) -> Refund {
        match value {
            Some(RefundDto::Cash) => Refund::Cash,
            None => Refund::None,
        }
    }
}
