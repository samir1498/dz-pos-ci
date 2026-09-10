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
//
// On the kit: the list is `DataTable`, the entry form is a `Sheet` of
// `FormField`s, every amount is `Money` and the one amount typed in is
// `MoneyInput`, so no float is made anywhere on the way through. The
// category is a chip rather than a `StatusPill`: a pill is one of the five
// states a row can be in, and a category is not a state.

import { useForm } from "@tanstack/react-form";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { ApiError, parseAmountToCentimes } from "@dzpos/shared";
import type {
  CashPositionDto,
  ExpenseCategoryDto,
  ExpenseDto,
  ExpensesDto,
} from "@dzpos/shared";
import { Plus, Receipt } from "lucide-react";
import { useState } from "react";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
import { PageHeader } from "@/components/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { api, cashQueryKey, expenseCategoriesQueryKey, expensesQueryKey } from "@/api";
import { isKey, useTranslation, type Key } from "@/i18n";
import { useShopToday } from "@/lib/clock";
import { cn } from "@/lib/utils";

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

/** The refusal, in the screen's own words, wherever one has to be shown. */
function Refusal({ error }: { error: unknown }) {
  const { t } = useTranslation();
  return (
    <p role="alert" className="text-sm text-fg-danger">
      {t(errorKey(error))}
    </p>
  );
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
        <PageHeader title={t("expenses_title")} />
        <Refusal error={today.error} />
        <Button variant="outline" className="self-start" onClick={today.retry}>
          {t("action_retry")}
        </Button>
      </section>
    );
  }
  if (chosen === null || today.today === undefined) {
    return (
      <section className="flex flex-col gap-4">
        <PageHeader title={t("expenses_title")} />
        <p className="sr-only">{t("expenses_loading")}</p>
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-64 w-full" />
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
  const { t, dir } = useTranslation();
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
    <section className="flex flex-col gap-6">
      <PageHeader
        title={t("expenses_title")}
        actions={
          <>
            <FormField label={t("expenses_month")} className="flex-row items-center gap-2">
              {(parts) => (
                <Input
                  {...parts}
                  type="month"
                  dir="ltr"
                  data-testid="expenses-month"
                  className="w-44 font-numeric tabular-nums"
                  value={month}
                  onChange={(event) => onMonth(event.target.value)}
                />
              )}
            </FormField>
            <Button data-testid="expenses-add" onClick={() => setAdding(true)}>
              <Icon as={Plus} size={18} />
              {t("expenses_add")}
            </Button>
          </>
        }
      />

      <div className="grid gap-4 lg:grid-cols-3">
        <TotalCard month={expenses.data} pending={expenses.isPending} />
        <div className="lg:col-span-2">
          {cash.isPending ? (
            <>
              <p className="sr-only">{t("cash_loading")}</p>
              <Skeleton className="h-56 w-full" />
            </>
          ) : null}
          {cash.isError ? <Refusal error={cash.error} /> : null}
          {cash.isSuccess ? <CashPanel position={cash.data} /> : null}
        </div>
      </div>

      {expenses.isPending ? <Skeleton className="h-64 w-full" /> : null}
      {expenses.isError ? <Refusal error={expenses.error} /> : null}
      {expenses.isSuccess ? (
        <ExpenseTable
          month={expenses.data}
          categories={categories.isSuccess ? categories.data : []}
          onAdd={() => setAdding(true)}
        />
      ) : null}

      <Sheet open={adding} onOpenChange={setAdding}>
        {/* The side is physical on purpose: the panel's edge, its border and
            the half it slides in from have to agree, so the caller picks it
            from the page direction the way AppShell does. */}
        <SheetContent side={dir === "rtl" ? "left" : "right"} data-testid="expense-sheet">
          <SheetHeader>
            <SheetTitle>{t("expenses_add")}</SheetTitle>
            <SheetDescription>{t("expenses_form_hint")}</SheetDescription>
          </SheetHeader>
          {categories.isPending ? (
            <div className="flex flex-col gap-3 px-4">
              <p className="sr-only">{t("expenses_loading")}</p>
              <Skeleton className="h-9 w-full" />
              <Skeleton className="h-9 w-full" />
              <Skeleton className="h-9 w-full" />
            </div>
          ) : null}
          {categories.isError ? (
            <div className="px-4">
              <Refusal error={categories.error} />
            </div>
          ) : null}
          {categories.isSuccess ? (
            <ExpenseForm
              categories={categories.data}
              month={month}
              today={today}
              onDone={() => setAdding(false)}
            />
          ) : null}
        </SheetContent>
      </Sheet>
    </section>
  );
}

/** The month's total, the one figure the screen is opened for. It is the
 *  server's sum and nothing here adds anything up. */
function TotalCard({ month, pending }: { month: ExpensesDto | undefined; pending: boolean }) {
  const { t } = useTranslation();
  return (
    <Card>
      <CardHeader>
        <CardDescription>{t("expenses_total")}</CardDescription>
        <CardTitle>
          {month !== undefined ? (
            <Money centimes={month.total_centimes} data-testid="expenses-total" className="text-2xl" />
          ) : pending ? (
            <Skeleton className="h-8 w-32" />
          ) : null}
        </CardTitle>
      </CardHeader>
    </Card>
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
    <Card data-testid="cash-position" className="h-full">
      <CardHeader>
        <CardTitle>{t("cash_title")}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1">
            <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
              {t("cash_in")}
            </h3>
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
            <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
              {t("cash_out")}
            </h3>
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
        <Separator />
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
      </CardContent>
    </Card>
  );
}

/** One labelled amount. The figure is `Money`, which carries the figure face
 *  and the `dir="ltr"` an amount needs on the Arabic screen too. */
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
    <p className={cn("flex items-baseline justify-between gap-4 text-sm", strong && "font-semibold")}>
      <span className={strong ? "text-foreground" : "text-muted-foreground"} title={hint}>
        {label}
      </span>
      <Money centimes={centimes} data-testid={testId} />
    </p>
  );
}

function ExpenseTable({
  month,
  categories,
  onAdd,
}: {
  month: ExpensesDto;
  categories: readonly ExpenseCategoryDto[];
  onAdd: () => void;
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

  const columns: readonly Column<ExpenseDto>[] = [
    {
      id: "date",
      header: t("col_date"),
      // The cell keeps the page's direction so the column starts where the
      // heading does; only the digits are LTR, in a span of their own.
      cell: (expense) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {expense.expense_date}
        </span>
      ),
    },
    {
      id: "category",
      header: t("col_category"),
      cell: (expense) => <Badge variant="secondary">{label(expense.category_id)}</Badge>,
    },
    {
      id: "amount",
      header: t("col_amount"),
      money: true,
      cell: (expense) => <Money centimes={expense.amount_centimes} />,
    },
    {
      id: "note",
      header: t("col_note"),
      cell: (expense) => expense.note ?? "",
    },
  ];

  return (
    <DataTable
      columns={columns}
      rows={month.expenses}
      rowKey={(expense) => expense.id}
      caption={t("expenses_title")}
      data-testid="expenses-table"
      empty={
        <EmptyState
          icon={Receipt}
          title={t("expenses_empty")}
          description={t("expenses_empty_hint")}
          action={
            <Button onClick={onAdd}>
              <Icon as={Plus} size={18} />
              {t("expenses_add")}
            </Button>
          }
        />
      }
    />
  );
}

/** What the entry sheet holds while it is being filled in. The amount is
 *  integer centimes from the first keystroke; there is no string and no
 *  float stage on the way to the server. */
interface Draft {
  category: string;
  amount: number | null;
  date: string;
  note: string;
}

/**
 * The add form. The day starts on the shop's clock, the categories are the
 * seeded ones and a retired category is not offered: the server refuses it,
 * and a form that offered it would be asking for a refusal.
 *
 * The amount lives in the form as integer centimes, never as a string and
 * never as a float: `MoneyInput` reads the typed text and hands back the
 * integer, and that integer is what is posted.
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

  // Spelled out rather than inferred from the literal below: the amount is
  // `number | null` and an inferred `null` would make the field's setter
  // refuse every integer `MoneyInput` hands it.
  const draft: Draft = {
    category: first === undefined ? "" : String(first.id),
    amount: null,
    // The shop's day when it is inside the month being looked at, and the
    // first of that month otherwise: a form opened on last month's list
    // should not default to filing the row outside it.
    date: monthOf(today) === month ? today : `${month}-01`,
    note: "",
  };

  const form = useForm({
    defaultValues: draft,
    onSubmit: async ({ value }) => {
      if (value.amount === null) return;
      // The rejection is swallowed on purpose: onError has already turned the
      // server's code into a translated message on the form.
      await save
        .mutateAsync({
          category_id: Number(value.category),
          amount_centimes: value.amount,
          expense_date: value.date,
          note: value.note.trim() === "" ? null : value.note.trim(),
        })
        .catch(() => undefined);
    },
  });

  return (
    <form
      noValidate
      data-testid="expense-form"
      className="flex flex-col gap-4 overflow-y-auto px-4 pb-4"
      onSubmit={(event) => {
        event.preventDefault();
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
          <FormField
            label={t("field_expense_category")}
            required
            error={message(t, field.state.meta.errors)}
          >
            {(parts) => (
              <Select value={field.state.value} onValueChange={field.handleChange}>
                <SelectTrigger
                  id={parts.id}
                  aria-invalid={parts["aria-invalid"]}
                  aria-describedby={parts["aria-describedby"]}
                  data-testid="expense-category"
                  className="w-full"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {live.map((category) => {
                    const key = CATEGORY_KEY[category.key];
                    return (
                      <SelectItem key={category.id} value={String(category.id)}>
                        {key === undefined ? category.key : t(key)}
                      </SelectItem>
                    );
                  })}
                </SelectContent>
              </Select>
            )}
          </FormField>
        )}
      </form.Field>

      <form.Field
        name="amount"
        validators={{
          onSubmit: ({ value }) => {
            if (value === null) return "error_expense_amount_invalid";
            // Refused here as well as by the core: a form that let a zero
            // through would show the server's refusal for something it could
            // have said itself.
            if (value <= 0) return "error_expense_amount_zero";
            return undefined;
          },
        }}
      >
        {(field) => (
          <FormField
            label={t("field_expense_amount")}
            required
            error={message(t, field.state.meta.errors)}
          >
            {(parts) => (
              <MoneyInput
                {...parts}
                data-testid="expense-amount"
                value={field.state.value}
                onChange={field.handleChange}
              />
            )}
          </FormField>
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
          <FormField
            label={t("field_expense_date")}
            required
            error={message(t, field.state.meta.errors)}
          >
            {(parts) => (
              <Input
                {...parts}
                type="date"
                dir="ltr"
                data-testid="expense-date"
                className="font-numeric tabular-nums"
                value={field.state.value}
                onChange={(event) => field.handleChange(event.target.value)}
              />
            )}
          </FormField>
        )}
      </form.Field>

      <form.Field name="note">
        {(field) => (
          <FormField label={t("field_expense_note")} hint={t("expenses_note_hint")}>
            {(parts) => (
              <Input
                {...parts}
                data-testid="expense-note"
                value={field.state.value}
                onChange={(event) => field.handleChange(event.target.value)}
              />
            )}
          </FormField>
        )}
      </form.Field>

      {serverError === null ? null : (
        <p role="alert" className="text-sm text-fg-danger">
          {t(serverError)}
        </p>
      )}

      <div className="flex gap-2">
        <Button type="submit" disabled={save.isPending}>
          {save.isPending ? t("action_saving") : t("action_save")}
        </Button>
        <Button type="button" variant="ghost" onClick={onDone}>
          {t("action_cancel")}
        </Button>
      </div>
    </form>
  );
}

/** Field validators return translation keys, never sentences, so the message
 *  a field shows is looked up here rather than carried through the form. */
function message(translate: (key: Key) => string, errors: unknown[]): string | undefined {
  const key = errors.find((error): error is string => typeof error === "string");
  if (key === undefined) return undefined;
  return translate(isKey(key) ? key : "error_unknown");
}
