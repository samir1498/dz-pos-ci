// Money out that is not stock, and the cash position the same screen shows
// beside it.

import { z } from "zod";

import type { CashPositionDto } from "../generated/CashPositionDto";
import type { ExpenseCategoryDto } from "../generated/ExpenseCategoryDto";
import type { ExpenseDto } from "../generated/ExpenseDto";
import type { ExpensesDto } from "../generated/ExpensesDto";
import type { NewExpenseDto } from "../generated/NewExpenseDto";
import type { OutgoingsDto } from "../generated/OutgoingsDto";
import type { TakingsDto } from "../generated/TakingsDto";
import { day, exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

/** A category carries an i18n key and never a label: the desktop holds the
 *  three languages under that key, so a shop switching language does not
 *  rewrite its rows. `active` is here because a retired category still names
 *  the expenses filed under it. */
export const expenseCategorySchema = z.object({
  id: z.number(),
  key: z.string(),
  sort_order: z.number(),
  active: z.boolean(),
}) satisfies z.ZodType<ExpenseCategoryDto>;
type _ExpenseCategory = Assert<Matches<ExpenseCategoryDto, typeof expenseCategorySchema>>;

/** The amount is an exact integer: a figure JSON.parse had to round is one a
 *  shop would read off its own month. */
export const expenseSchema = z.object({
  id: z.number(),
  category_id: z.number(),
  amount_centimes: exactInteger,
  expense_date: day,
  note: z.string().nullable(),
}) satisfies z.ZodType<ExpenseDto>;
type _Expense = Assert<Matches<ExpenseDto, typeof expenseSchema>>;

/** The month as the API writes it, `YYYY-MM`. Narrower than `day`, and its
 *  own shape: a month that arrived as a day would be a range the screen and
 *  the server disagree about. */
export const month = z.string().regex(/^\d{4}-\d{2}$/);

export const expensesSchema = z.object({
  month,
  total_centimes: exactInteger,
  expenses: z.array(expenseSchema),
}) satisfies z.ZodType<ExpensesDto>;
type _Expenses = Assert<Matches<ExpensesDto, typeof expensesSchema>>;

export const newExpenseSchema = z.object({
  category_id: z.number(),
  amount_centimes: exactInteger,
  expense_date: day,
  note: z.string().nullable(),
}) satisfies z.ZodType<NewExpenseDto>;
type _NewExpense = Assert<Matches<NewExpenseDto, typeof newExpenseSchema>>;

/** `stamp_centimes` is the droit de timbre inside `sales_centimes`, not a
 *  figure beside it: a screen showing the shop's own takings subtracts it,
 *  and one counting the till does not. */
export const takingsSchema = z.object({
  sales_centimes: exactInteger,
  stamp_centimes: exactInteger,
  customer_payments_centimes: exactInteger,
  total_centimes: exactInteger,
}) satisfies z.ZodType<TakingsDto>;
type _Takings = Assert<Matches<TakingsDto, typeof takingsSchema>>;

export const outgoingsSchema = z.object({
  refunds_centimes: exactInteger,
  supplier_payments_centimes: exactInteger,
  expenses_centimes: exactInteger,
  total_centimes: exactInteger,
}) satisfies z.ZodType<OutgoingsDto>;
type _Outgoings = Assert<Matches<OutgoingsDto, typeof outgoingsSchema>>;

/** `cash_centimes` is the net of the two sides and goes below zero on a day
 *  the shop paid out more than it took, so nothing here is a positive
 *  integer. */
export const cashPositionSchema = z.object({
  from: day,
  to: day,
  cash_in: takingsSchema,
  cash_out: outgoingsSchema,
  cash_centimes: exactInteger,
  card_in: takingsSchema,
}) satisfies z.ZodType<CashPositionDto>;
type _CashPosition = Assert<Matches<CashPositionDto, typeof cashPositionSchema>>;
