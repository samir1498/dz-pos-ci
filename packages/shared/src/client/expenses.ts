// A month of expenses, the seeded categories, writing one, and the cash
// position (`routes::expenses::cash`).

import { z } from "zod";

import type { CashPositionDto } from "../generated/CashPositionDto";
import type { ExpenseCategoryDto } from "../generated/ExpenseCategoryDto";
import type { ExpenseDto } from "../generated/ExpenseDto";
import type { ExpensesDto } from "../generated/ExpensesDto";
import type { NewExpenseDto } from "../generated/NewExpenseDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import {
  cashPositionSchema,
  expenseCategorySchema,
  expenseSchema,
  expensesSchema,
} from "../schemas/expense";

export function expensesClient({ send }: Transport) {
  return {
    /** One month of expenses and what it came to, `YYYY-MM` on the shop's
     * calendar. The total is the server's: a screen that added the rows up
     * would be a second answer to the same question. */
    async listExpenses(month: string): Promise<ExpensesDto> {
      const query = new URLSearchParams({ month });
      return narrow(await send(`/expenses?${query.toString()}`), expensesSchema, "expenses");
    },

    /** The seven seeded categories, in the order the screen lists them. Each
     * carries an i18n key, and the label comes from the app's own language
     * files. */
    async listExpenseCategories(): Promise<ExpenseCategoryDto[]> {
      return narrow(
        await send("/expense-categories"),
        z.array(expenseCategorySchema),
        "expense categories",
      );
    },

    async createExpense(input: NewExpenseDto): Promise<ExpenseDto> {
      const body = await send("/expenses", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, expenseSchema, "expense");
    },

    /** The cash position over one day or one month, never both: the server
     * refuses a call that names the two, so the range a figure covers is
     * always the one that was asked for. */
    async cashPosition(period: { day: string } | { month: string }): Promise<CashPositionDto> {
      const query = new URLSearchParams(period);
      return narrow(await send(`/cash?${query.toString()}`), cashPositionSchema, "cash position");
    },
  };
}
