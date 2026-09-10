// Contract: what an expense and a cash position look like on the wire. The
// amounts are exact integers, the days are `YYYY-MM-DD` and the month is
// `YYYY-MM`, because a screen that took a day for a month would show a figure
// for the wrong range and say nothing about it.

import { describe, expect, test } from "vitest";

import type { CashPositionDto } from "../generated/CashPositionDto";
import type { ExpenseDto } from "../generated/ExpenseDto";
import type { ExpensesDto } from "../generated/ExpensesDto";
import type { NewExpenseDto } from "../generated/NewExpenseDto";
import {
  cashPositionSchema,
  expenseCategorySchema,
  expenseSchema,
  expensesSchema,
  newExpenseSchema,
} from "./expense";

const expense: ExpenseDto = {
  id: 3,
  category_id: 1,
  amount_centimes: 3_000_000,
  expense_date: "2026-09-10",
  note: "loyer septembre",
};

const month: ExpensesDto = {
  month: "2026-09",
  total_centimes: 3_000_000,
  expenses: [expense],
};

const position: CashPositionDto = {
  from: "2026-09-01",
  to: "2026-09-30",
  cash_in: {
    sales_centimes: 502_000,
    stamp_centimes: 2_000,
    customer_payments_centimes: 30_000,
    total_centimes: 532_000,
  },
  cash_out: {
    refunds_centimes: 0,
    supplier_payments_centimes: 40_000,
    expenses_centimes: 3_000_000,
    total_centimes: 3_040_000,
  },
  cash_centimes: -2_508_000,
  card_in: {
    sales_centimes: 50_000,
    stamp_centimes: 0,
    customer_payments_centimes: 0,
    total_centimes: 50_000,
  },
};

describe("an expense", () => {
  test("a row from the API parses whole", () => {
    expect(expenseSchema.parse(expense)).toEqual(expense);
    expect(expenseSchema.parse({ ...expense, note: null }).note).toBeNull();
  });

  test("a category carries its key and whether it is still in use", () => {
    const category = { id: 1, key: "rent", sort_order: 1, active: false };
    expect(expenseCategorySchema.parse(category)).toEqual(category);
  });

  test("an amount JSON.parse had to round is refused", () => {
    expect(() =>
      expenseSchema.parse({ ...expense, amount_centimes: Number.MAX_SAFE_INTEGER + 2 }),
    ).toThrow();
  });

  test("a day that is not a day is refused", () => {
    expect(() => expenseSchema.parse({ ...expense, expense_date: "10/09/2026" })).toThrow();
    expect(() => expenseSchema.parse({ ...expense, expense_date: "2026-09" })).toThrow();
  });

  test("the form's body carries the four fields and nothing else", () => {
    const body: NewExpenseDto = {
      category_id: 1,
      amount_centimes: 1_000,
      expense_date: "2026-09-10",
      note: null,
    };
    expect(newExpenseSchema.parse(body)).toEqual(body);
    // z.object strips what it has no field for, so an extra key never
    // reaches the API as a field the server would refuse.
    expect(newExpenseSchema.parse({ ...body, user_id: 1 })).toEqual(body);
  });
});

describe("a month of expenses", () => {
  test("the month, its total and its rows come back together", () => {
    expect(expensesSchema.parse(month)).toEqual(month);
  });

  test("an empty month is a total of zero and no rows", () => {
    const empty: ExpensesDto = { month: "2026-07", total_centimes: 0, expenses: [] };
    expect(expensesSchema.parse(empty)).toEqual(empty);
  });

  test("a month written as a day is refused", () => {
    expect(() => expensesSchema.parse({ ...month, month: "2026-09-01" })).toThrow();
    expect(() => expensesSchema.parse({ ...month, month: "2026-9" })).toThrow();
  });
});

describe("the cash position", () => {
  test("both sides and the card figure parse whole", () => {
    expect(cashPositionSchema.parse(position)).toEqual(position);
  });

  test("a day that paid out more than it took is a figure below zero", () => {
    expect(cashPositionSchema.parse(position).cash_centimes).toBe(-2_508_000);
  });

  test("the stamp is inside the takings and not beside them", () => {
    const parsed = cashPositionSchema.parse(position);
    expect(parsed.cash_in.stamp_centimes).toBe(2_000);
    // The total is sales plus the debt payments; adding the stamp again
    // would count the tax twice.
    expect(parsed.cash_in.total_centimes).toBe(
      parsed.cash_in.sales_centimes + parsed.cash_in.customer_payments_centimes,
    );
  });

  test("a side that arrived without the stamp figure is refused", () => {
    const { stamp_centimes: _dropped, ...short } = position.cash_in;
    expect(() => cashPositionSchema.parse({ ...position, cash_in: short })).toThrow();
  });

  test("a side that arrived short of a figure is refused rather than read as zero", () => {
    const { total_centimes: _dropped, ...short } = position.cash_in;
    expect(() => cashPositionSchema.parse({ ...position, cash_in: short })).toThrow();
  });
});
