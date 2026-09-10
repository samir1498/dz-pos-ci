// The expenses screen: what the shop paid out that was not stock, one month
// at a time, with the cash position of that same month beside it.
//
// The month comes from the shop's clock and never from `new Date()`: the core
// dates every row on Algeria's calendar, and a machine in another zone would
// show a month the ledger has not reached. The screen waits for `/clock`
// rather than guessing and correcting itself a moment later.
//
// Nothing here edits or deletes a row. An expense is written once
// (features.md §1), so the list is a list and there is no pencil on it.

import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { useState } from "react";
import { ApiError, formatCentimes, parseAmountToCentimes } from "@dzpos/shared";
import type { CashPositionDto, ExpenseCategoryDto, ExpensesDto } from "@dzpos/shared";
import { api, cashQueryKey, expenseCategoriesQueryKey, expensesQueryKey } from "@/api";
import { isKey, useTranslation, type Key } from "@/i18n";
import { useShopToday } from "@/lib/clock";

export const Route = createFileRoute("/expenses")({ component: ExpensesScreen });

/** The label of each seeded category, by the key the row carries. The row
 *  never holds a label, so a shop switching language reads its own months in
 *  the new one without a single row being rewritten. */
const CATEGORY_KEY: Record<string, Key> = {
  rent: "expense_category_rent",
  electricity: "expense_category_electricity",
  water: "expense_category_water",
  salaries: "expense_category_salaries",
  transport: "expense_category_transport",
  maintenance: "expense_category_maintenance",
  other: "expense_category_other",
};

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  not_found: "error_not_found",
  money: "error_money",
  storage: "error_storage",
  restart_needed: "error_restart_needed",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unreachable: "error_unreachable",
  unauthorized: "error_unauthorized",
};

/** The server sends a code, never a sentence; the UI owns the wording. */
function errorKey(error: unknown): Key {
  if (error instanceof ApiError) {
    return ERROR_KEY[error.code] ?? "error_unknown";
  }
  return "error_unknown";
}

/** The month a day falls in, `YYYY-MM`. Cut off the day rather than built
 *  from a Date: the string already comes from the shop's clock. */
function monthOf(day: string): string {
  return day.slice(0, 7);
}

export function ExpensesScreen() {
  const { t } = useTranslation();
  const today = useShopToday();
  // Null until the clock has answered. The month picker is rendered with the
  // shop's month already in it rather than with a wrong one it corrects.
  const [month, setMonth] = useState<string | null>(null);
  const chosen = month ?? (today.today === undefined ? null : monthOf(today.today));

  if (today.error !== null) {
    return (
      <section className="flex flex-col gap-4">
        <h1 className="text-xl font-semibold">{t("expenses_title")}</h1>
        <p role="alert" className="text-red-700">
          {t(errorKey(today.error))}
        </p>
        <button type="button" className="self-start rounded border px-3 py-1.5" onClick={today.retry}>
          {t("action_retry")}
        </button>
      </section>
    );
  }
  if (chosen === null || today.today === undefined) {
    return (
      <section className="flex flex-col gap-4">
        <h1 className="text-xl font-semibold">{t("expenses_title")}</h1>
        <p>{t("expenses_loading")}</p>
      </section>
    );
  }
  return <Month month={chosen} today={today.today} onMonth={setMonth} />;
}

function Month({
  month,
  today,
  onMonth,
}: {
  month: string;
  today: string;
  onMonth: (month: string) => void;
}) {
  const { t } = useTranslation();
  const [adding, setAdding] = useState(false);
  const expenses = useQuery({
    queryKey: expensesQueryKey(month),
    queryFn: () => api.listExpenses(month),
  });
  const categories = useQuery({
    queryKey: expenseCategoriesQueryKey,
    queryFn: () => api.listExpenseCategories(),
  });
  const cash = useQuery({
    queryKey: cashQueryKey({ month }),
    queryFn: () => api.cashPosition({ month }),
  });

  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("expenses_title")}</h1>
        <label className="flex items-center gap-2">
          <span>{t("expenses_month")}</span>
          <input
            type="month"
            dir="ltr"
            data-testid="expenses-month"
            className="rounded border px-2 py-1 font-mono"
            value={month}
            onChange={(e) => onMonth(e.target.value)}
          />
        </label>
        <button
          type="button"
          className="rounded border px-3 py-1.5"
          onClick={() => setAdding((open) => !open)}
        >
          {adding ? t("action_cancel") : t("expenses_add")}
        </button>
      </header>

      {adding && categories.isSuccess ? (
        <ExpenseForm
          categories={categories.data}
          month={month}
          today={today}
          onDone={() => setAdding(false)}
        />
      ) : null}
      {adding && categories.isPending ? <p>{t("expenses_loading")}</p> : null}
      {adding && categories.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(categories.error))}
        </p>
      ) : null}

      {cash.isPending ? <p>{t("cash_loading")}</p> : null}
      {cash.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(cash.error))}
        </p>
      ) : null}
      {cash.isSuccess ? <CashPanel position={cash.data} /> : null}

      {expenses.isPending ? <p>{t("expenses_loading")}</p> : null}
      {expenses.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(expenses.error))}
        </p>
      ) : null}
      {expenses.isSuccess ? (
        <ExpenseTable
          month={expenses.data}
          categories={categories.isSuccess ? categories.data : []}
        />
      ) : null}
    </section>
  );
}

/**
 * The cash position of the month. Every figure is the server's: the panel
 * shows what the core summed and adds nothing up, so the box and the
 * dashboard can never be two answers to one question.
 */
function CashPanel({ position }: { position: CashPositionDto }) {
  const { t } = useTranslation();
  return (
    <div data-testid="cash-position" className="flex flex-col gap-2 rounded border p-4">
      <h2 className="font-semibold">{t("cash_title")}</h2>
      <div className="grid gap-4 sm:grid-cols-2">
        <div className="flex flex-col gap-1">
          <h3 className="text-sm uppercase">{t("cash_in")}</h3>
          <Figure label={t("cash_sales")} centimes={position.cash_in.sales_centimes} />
          {/* Inside the line above, not beside it: the drawer took the stamp
              with the rest, and this says how much of what it took is tax
              the shop is holding for the state. */}
          <Figure
            label={t("cash_stamp")}
            centimes={position.cash_in.stamp_centimes}
            testId="cash-in-stamp"
          />
          <Figure
            label={t("cash_customer_payments")}
            centimes={position.cash_in.customer_payments_centimes}
          />
          <Figure
            label={t("cash_total")}
            centimes={position.cash_in.total_centimes}
            testId="cash-in-total"
            strong
          />
        </div>
        <div className="flex flex-col gap-1">
          <h3 className="text-sm uppercase">{t("cash_out")}</h3>
          <Figure
            label={t("cash_supplier_payments")}
            centimes={position.cash_out.supplier_payments_centimes}
          />
          <Figure
            label={t("cash_expenses")}
            centimes={position.cash_out.expenses_centimes}
            testId="cash-out-expenses"
          />
          <Figure
            label={t("cash_refunds")}
            centimes={position.cash_out.refunds_centimes}
            hint={t("cash_refunds_hint")}
          />
          <Figure
            label={t("cash_total")}
            centimes={position.cash_out.total_centimes}
            testId="cash-out-total"
            strong
          />
        </div>
      </div>
      <Figure
        label={t("cash_net")}
        centimes={position.cash_centimes}
        testId="cash-net"
        strong
      />
      <Figure
        label={t("cash_card_in")}
        centimes={position.card_in.total_centimes}
        testId="card-in-total"
      />
    </div>
  );
}

/** One labelled amount. `dir="ltr"` on the figure: an amount is read left to
 *  right with Western digits whatever the screen's language. */
function Figure({
  label,
  centimes,
  testId,
  hint,
  strong = false,
}: {
  label: string;
  centimes: number;
  testId?: string;
  hint?: string;
  strong?: boolean;
}) {
  return (
    <p className={strong ? "flex justify-between gap-4 font-semibold" : "flex justify-between gap-4"}>
      <span title={hint}>{label}</span>
      <span className="font-mono" dir="ltr" data-testid={testId}>
        {formatCentimes(centimes)}
      </span>
    </p>
  );
}

function ExpenseTable({
  month,
  categories,
}: {
  month: ExpensesDto;
  categories: readonly ExpenseCategoryDto[];
}) {
  const { t } = useTranslation();
  const label = (id: number): string => {
    const found = categories.find((c) => c.id === id);
    if (found === undefined) return t("expense_category_unknown");
    const key = CATEGORY_KEY[found.key];
    // A category a later version seeded and this build has no word for:
    // its own key is closer to the truth than a blank cell.
    return key === undefined ? found.key : t(key);
  };
  return (
    <div className="flex flex-col gap-2">
      <p className="flex justify-between gap-4 font-semibold">
        <span>{t("expenses_total")}</span>
        <span className="font-mono" dir="ltr" data-testid="expenses-total">
          {formatCentimes(month.total_centimes)}
        </span>
      </p>
      {month.expenses.length === 0 ? (
        <p>{t("expenses_empty")}</p>
      ) : (
        <table className="w-full text-start">
          <caption className="sr-only">{t("expenses_title")}</caption>
          <thead>
            <tr>
              {/* The same logical padding as the cells under them, so the
                  four headings keep the gaps the rows have in either
                  direction rather than running into each other. */}
              <th scope="col" className="pb-2 pe-3 text-start">
                {t("col_date")}
              </th>
              <th scope="col" className="pb-2 pe-3 text-start">
                {t("col_category")}
              </th>
              <th scope="col" className="pb-2 ps-3 text-end">
                {t("col_amount")}
              </th>
              <th scope="col" className="pb-2 ps-3 text-start">
                {t("col_note")}
              </th>
            </tr>
          </thead>
          <tbody>
            {month.expenses.map((e) => (
              <tr key={e.id} className="border-t" data-testid="expense-row">
                {/* The cell keeps the page's direction so the column starts
                    where the heading does; only the digits are LTR, in a span
                    of their own. `dir="ltr"` on the cell would left-align it
                    inside an RTL row and push the date against the column
                    beside it. */}
                <td className="py-1.5 pe-3">
                  <span className="font-mono" dir="ltr">
                    {e.expense_date}
                  </span>
                </td>
                <td className="py-1.5 pe-3">{label(e.category_id)}</td>
                <td className="py-1.5 ps-3 text-end font-mono" dir="ltr">
                  {formatCentimes(e.amount_centimes)}
                </td>
                <td className="py-1.5 ps-3">{e.note ?? ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

/**
 * The add form. The day starts on the shop's clock, the categories are the
 * seeded ones and a retired category is not offered: the server refuses it,
 * and a form that offered it would be asking for a refusal.
 */
function ExpenseForm({
  categories,
  month,
  today,
  onDone,
}: {
  categories: readonly ExpenseCategoryDto[];
  month: string;
  today: string;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);
  const live = categories.filter((c) => c.active);
  const first = live[0];

  const save = useMutation({
    mutationFn: (input: {
      category_id: number;
      amount_centimes: number;
      expense_date: string;
      note: string | null;
    }) => api.createExpense(input),
    onSuccess: async (made) => {
      setServerError(null);
      // The month the row landed in, which is not always the one on screen:
      // a shop filing last month's electricity bill would otherwise see the
      // list it is looking at stay as it was.
      const landed = monthOf(made.expense_date);
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: expensesQueryKey(month) }),
        queryClient.invalidateQueries({ queryKey: expensesQueryKey(landed) }),
        queryClient.invalidateQueries({ queryKey: cashQueryKey({ month }) }),
        queryClient.invalidateQueries({ queryKey: cashQueryKey({ month: landed }) }),
      ]);
      onDone();
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  const form = useForm({
    defaultValues: {
      category: first === undefined ? "" : String(first.id),
      amount: "",
      // The shop's day when it is inside the month being looked at, and the
      // first of that month otherwise: a form opened on last month's list
      // should not default to filing the row outside it.
      date: monthOf(today) === month ? today : `${month}-01`,
      note: "",
    },
    onSubmit: async ({ value }) => {
      const amount = parseAmountToCentimes(value.amount);
      if (amount === null) return;
      // The rejection is swallowed on purpose: onError has already turned the
      // server's code into a translated message on the form.
      await save
        .mutateAsync({
          category_id: Number(value.category),
          amount_centimes: amount,
          expense_date: value.date,
          note: value.note.trim() === "" ? null : value.note.trim(),
        })
        .catch(() => undefined);
    },
  });

  return (
    <form
      noValidate
      className="flex flex-col gap-3 rounded border p-4"
      onSubmit={(e) => {
        e.preventDefault();
        void form.handleSubmit();
      }}
    >
      <form.Field
        name="category"
        validators={{
          onSubmit: ({ value }) =>
            value === "" ? "error_expense_category_required" : undefined,
        }}
      >
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_expense_category")}</span>
            <select
              data-testid="expense-category"
              className="rounded border px-2 py-1"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            >
              {live.map((c) => {
                const key = CATEGORY_KEY[c.key];
                return (
                  <option key={c.id} value={String(c.id)}>
                    {key === undefined ? c.key : t(key)}
                  </option>
                );
              })}
            </select>
            <FieldError messages={field.state.meta.errors} />
          </label>
        )}
      </form.Field>

      <form.Field
        name="amount"
        validators={{
          onSubmit: ({ value }) => {
            const centimes = parseAmountToCentimes(value);
            if (centimes === null) return "error_expense_amount_invalid";
            // Refused here as well as by the core: a form that let a zero
            // through would show the server's refusal for something it could
            // have said itself.
            if (centimes <= 0) return "error_expense_amount_zero";
            return undefined;
          },
        }}
      >
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_expense_amount")}</span>
            <input
              dir="ltr"
              inputMode="decimal"
              data-testid="expense-amount"
              className="rounded border px-2 py-1 text-end font-mono"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
              onBlur={field.handleBlur}
            />
            <FieldError messages={field.state.meta.errors} />
          </label>
        )}
      </form.Field>

      <form.Field
        name="date"
        validators={{
          onSubmit: ({ value }) =>
            /^\d{4}-\d{2}-\d{2}$/.test(value) ? undefined : "error_day_invalid",
        }}
      >
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_expense_date")}</span>
            <input
              type="date"
              dir="ltr"
              data-testid="expense-date"
              className="rounded border px-2 py-1 font-mono"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
            <FieldError messages={field.state.meta.errors} />
          </label>
        )}
      </form.Field>

      <form.Field name="note">
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_expense_note")}</span>
            <input
              className="rounded border px-2 py-1"
              data-testid="expense-note"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
          </label>
        )}
      </form.Field>

      {serverError === null ? null : (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      )}

      <div className="flex gap-2">
        <button
          type="submit"
          className="rounded border px-3 py-1.5"
          disabled={save.isPending}
        >
          {save.isPending ? t("action_saving") : t("action_save")}
        </button>
        <button type="button" className="rounded border px-3 py-1.5" onClick={onDone}>
          {t("action_cancel")}
        </button>
      </div>
    </form>
  );
}

/** Field validators return translation keys, never sentences. */
function FieldError({ messages }: { messages: unknown[] }) {
  const { t } = useTranslation();
  const key = messages.find((m): m is string => typeof m === "string");
  if (key === undefined) return null;
  return (
    <span role="alert" className="text-sm text-red-700">
      {t(isKey(key) ? key : "error_unknown")}
    </span>
  );
}
