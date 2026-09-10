// One customer's account, addressable. What the counter reads: what is owed
// now, every movement behind that figure, the payments and what each one
// settled, and the two papers the shop can hand over. Anything that names a
// customer links here: the till's credit refusal, the documents screen, and
// the list's own rows.
//
// The trailing underscore on `customers_` keeps this out from under the list
// screen's route rather than nesting inside it: `/customers` is a whole page
// of its own and not a layout with an outlet.
//
// No amount on this page is worked out here. The balance and the running
// column of the ledger are the core's (`services::debt`), the statement and
// the debt slip are pages the core rendered, and they go into an iframe as
// they came: the shop is looking at what the printer will put on paper
// rather than at a second rendering of the same balances.

import { ApiError } from "@dzpos/shared";
import type {
  CustomerDto,
  CustomerLedgerDto,
  CustomerPaymentsDto,
  PaymentDto,
  PaymentMethodDto,
} from "@dzpos/shared";
import { useForm } from "@tanstack/react-form";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, createFileRoute } from "@tanstack/react-router";
import { ArrowLeft, Receipt, ScrollText, SquarePen, Wallet } from "lucide-react";
import { useState } from "react";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
import { PageHeader } from "@/components/PageHeader";
import { PayButton } from "@/components/PayButton";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import {
  api,
  customerDebtSlipKeyPrefix,
  customerDebtSlipQueryKey,
  customerLedgerQueryKey,
  customerPaymentsQueryKey,
  customerQueryKey,
  customerStatementQueryKey,
  customersQueryKey,
} from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { useShopToday } from "@/lib/clock";
import { cleared, errorKey } from "@/lib/fields";

import { CustomerFicheSheet } from "./-customers/fiche";
import {
  ChoiceRow,
  CustomerStatus,
  DEBT_KIND_KEY,
  PAYMENT_METHODS,
  PAYMENT_METHOD_KEY,
  balanceLabel,
  useFieldError,
} from "./-customers/parts";

export const Route = createFileRoute("/customers_/$id")({ component: OneCustomer });

function OneCustomer() {
  const { id } = Route.useParams();
  // A path is text, and `/customers/abc` is a link somebody mistyped rather
  // than a customer this shop does not have. `Number` on it is NaN, which
  // would go to the API as `/customers/NaN` and come back a bad request; the
  // page answers it here instead, and says the one thing there is to say.
  const parsed = Number(id);
  if (!Number.isInteger(parsed)) return <NotACustomer />;
  return <CustomerFiche id={parsed} />;
}

function BackToList() {
  const { t } = useTranslation();
  return (
    <Button variant="ghost" asChild>
      <Link to="/customers">
        <Icon as={ArrowLeft} size={18} flip />
        {t("action_back_to_customers")}
      </Link>
    </Button>
  );
}

function NotACustomer() {
  const { t } = useTranslation();
  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t("customers_title")} actions={<BackToList />} />
      <p role="alert" className="text-sm text-fg-danger">
        {t("error_not_found")}
      </p>
    </section>
  );
}

export function CustomerFiche({ id }: { id: number }) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState(false);
  const customer = useQuery({
    queryKey: customerQueryKey(id),
    queryFn: () => api.getCustomer(id),
  });

  if (customer.isPending) {
    return (
      <section className="flex flex-col gap-4">
        <PageHeader title={t("customers_title")} actions={<BackToList />} />
        <span className="sr-only">{t("customers_loading")}</span>
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-64 w-full" />
      </section>
    );
  }

  if (customer.isError) {
    return (
      <section className="flex flex-col gap-4">
        <PageHeader title={t("customers_title")} actions={<BackToList />} />
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(customer.error))}
        </p>
      </section>
    );
  }

  const shown = customer.data;
  const said = [shown.phone, shown.address].filter((part): part is string => part !== null);

  return (
    <section className="flex flex-col gap-6">
      <PageHeader
        title={shown.name}
        description={said.length === 0 ? undefined : said.join(" · ")}
        actions={
          <>
            <BackToList />
            <Button variant="outline" onClick={() => setEditing(true)}>
              <Icon as={SquarePen} size={18} />
              {t("customers_modify")}
            </Button>
          </>
        }
      />

      <Figures customer={shown} />
      <Ledger customer={shown} />
      <PaymentsPanel customer={shown} />
      <div className="grid gap-4 lg:grid-cols-2">
        <StatementPanel customer={shown} />
        <DebtSlipPanel customer={shown} />
      </div>
      <AdjustPanel customer={shown} />

      <CustomerFicheSheet open={editing} initial={shown} onOpenChange={setEditing} />
    </section>
  );
}

/** What is owed, what the till will allow, and where the warning starts. The
 *  three figures the counter asks for before it sells anything on credit. */
function Figures({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-4 sm:grid-cols-3">
      <Card data-testid="customer-balance">
        <CardHeader>
          <CardDescription>{t(balanceLabel(customer.balance_centimes))}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap items-center justify-between gap-2">
          <Money centimes={Math.abs(customer.balance_centimes)} className="text-2xl" />
          <CustomerStatus customer={customer} />
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardDescription>{t("col_limit")}</CardDescription>
        </CardHeader>
        <CardContent>
          {customer.credit_limit_centimes === null ? (
            <span className="text-md text-muted-foreground">{t("customers_no_limit")}</span>
          ) : (
            <Money centimes={customer.credit_limit_centimes} className="text-2xl" />
          )}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardDescription>{t("field_warn_threshold")}</CardDescription>
        </CardHeader>
        <CardContent>
          {customer.warn_threshold_centimes === null ? (
            <span className="text-md text-muted-foreground">{t("customers_no_limit")}</span>
          ) : (
            <Money centimes={customer.warn_threshold_centimes} className="text-2xl" />
          )}
        </CardContent>
      </Card>
    </div>
  );
}

/** The movements, newest first, with the balance each one left behind. */
function Ledger({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  const ledger = useQuery({
    queryKey: customerLedgerQueryKey(customer.id),
    queryFn: () => api.customerLedger(customer.id),
  });

  const columns: readonly Column<CustomerLedgerDto["entries"][number]>[] = [
    {
      id: "date",
      header: t("col_date"),
      cell: (entry) => (
        <span dir="ltr" className="font-numeric text-muted-foreground">
          {entry.created_at}
        </span>
      ),
    },
    { id: "kind", header: t("col_kind"), cell: (entry) => t(DEBT_KIND_KEY[entry.kind]) },
    {
      id: "debit",
      header: t("col_debit"),
      money: true,
      cell: (entry) =>
        entry.debit_centimes === 0 ? null : <Money centimes={entry.debit_centimes} />,
    },
    {
      id: "credit",
      header: t("col_credit"),
      money: true,
      cell: (entry) =>
        entry.credit_centimes === 0 ? null : <Money centimes={entry.credit_centimes} />,
    },
    {
      // The running balance is the core's (services::debt): a column added up
      // here would be a second answer to what a customer owes.
      id: "balance",
      header: t("col_balance"),
      money: true,
      cell: (entry) => <Money centimes={entry.balance_after_centimes} />,
    },
    {
      id: "note",
      header: t("col_note"),
      cell: (entry) => <span className="text-muted-foreground">{entry.note ?? ""}</span>,
    },
  ];

  return (
    <section className="flex flex-col gap-3">
      <h3 className="text-md font-semibold text-foreground">{t("customers_ledger")}</h3>
      {ledger.isPending ? <Skeleton className="h-40 w-full" /> : null}
      {ledger.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(ledger.error))}
        </p>
      ) : null}
      {ledger.isSuccess ? (
        <DataTable
          columns={columns}
          rows={ledger.data.entries}
          rowKey={(entry) => entry.id}
          caption={t("customers_ledger")}
          data-testid="customer-ledger"
          empty={<EmptyState icon={ScrollText} title={t("customers_ledger_empty")} />}
        />
      ) : null}
    </section>
  );
}

/**
 * Money against the debt, and what each payment settled. The server settles
 * the oldest documents first and refuses a payment above what the customer
 * owes, so nothing here caps the figure or picks the documents: a screen that
 * decided either would be a second answer to what the customer owes.
 *
 * The dialog is the confirmation. It says what taking the money means before
 * the brass button is there to press, which is what the browser's own confirm
 * box used to do a step later and with a second window on top of this one.
 */
/** Centimes or nothing at all: `MoneyInput` never makes a float, and a blank
 *  box is "no amount given" rather than a payment of zero. */
interface PaymentValues {
  amount: number | null;
  mode: PaymentMethodDto;
  note: string;
}

function PaymentsPanel({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  const said = useFieldError();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [serverError, setServerError] = useState<Key | null>(null);
  /** What the customer actually owes, as the refused payment reported it.
   *  "Too much" is useless without the amount that would not have been. */
  const [outstanding, setOutstanding] = useState<number | null>(null);
  const [saved, setSaved] = useState(false);

  const payments = useQuery({
    queryKey: customerPaymentsQueryKey(customer.id),
    queryFn: () => api.customerPayments(customer.id),
  });

  const pay = useMutation({
    mutationFn: (input: {
      amount_centimes: number;
      payment_mode: PaymentMethodDto;
      note: string | null;
    }) => api.payCustomer(customer.id, input),
    onSuccess: async (answer: CustomerPaymentsDto) => {
      setServerError(null);
      setOutstanding(null);
      setSaved(true);
      setOpen(false);
      queryClient.setQueryData(customerPaymentsQueryKey(customer.id), answer);
      // The movement is on the ledger too, and the balance on the fiche came
      // from the customers query. The slip is a rendered page carrying the
      // old balance, in whichever languages it has been asked for.
      await queryClient.invalidateQueries({ queryKey: customerLedgerQueryKey(customer.id) });
      await queryClient.invalidateQueries({ queryKey: customersQueryKey });
      await queryClient.invalidateQueries({ queryKey: customerDebtSlipKeyPrefix(customer.id) });
    },
    onError: (error: unknown) => {
      setSaved(false);
      const above = error instanceof ApiError ? error.outstandingCentimes : undefined;
      setOutstanding(above ?? null);
      setServerError(above === undefined ? errorKey(error) : "error_payment_above_debt");
    },
  });

  const blank: PaymentValues = { amount: null, mode: "cash", note: "" };
  const form = useForm({
    defaultValues: blank,
    onSubmit: async ({ value }) => {
      if (value.amount === null || value.amount <= 0) return;
      const written = await pay
        .mutateAsync({
          amount_centimes: value.amount,
          payment_mode: value.mode,
          note: cleared(value.note),
        })
        .then(() => true)
        .catch(() => false);
      // A refused payment keeps what was typed: the error above says what is
      // outstanding, and an emptied box means typing the figure again to find
      // out what was wrong with it.
      if (!written) return;
      form.setFieldValue("amount", null);
      form.setFieldValue("note", "");
    },
  });

  return (
    <Card>
      <CardHeader>
        <h3 className="text-md leading-none font-semibold">{t("customers_payments")}</h3>
        <CardDescription>{t("customers_pay_hint")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {/* A closed fiche keeps this: the shop stopped selling to this
            customer, not collecting from them (core, services::debt). */}
        {customer.active ? null : (
          <p className="text-sm text-muted-foreground">{t("customers_closed_still_collects")}</p>
        )}

        <div>
          <Button
            variant="outline"
            data-testid="customer-pay-button"
            onClick={() => {
              setSaved(false);
              setOpen(true);
            }}
          >
            <Icon as={Wallet} size={18} />
            {t("customers_pay")}
          </Button>
        </div>

        {saved && serverError === null ? (
          <p role="status" className="text-sm text-fg-success">
            {t("customers_paid")}
          </p>
        ) : null}

        {payments.isPending ? <Skeleton className="h-20 w-full" /> : null}
        {payments.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(payments.error))}
          </p>
        ) : null}
        {payments.isSuccess && payments.data.payments.length === 0 ? (
          <EmptyState icon={Receipt} title={t("customers_payments_empty")} />
        ) : null}
        {payments.isSuccess
          ? payments.data.payments.map((payment) => (
              <PaymentRow key={payment.ledger_id} payment={payment} />
            ))
          : null}
      </CardContent>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent data-testid="customer-pay-dialog">
          <DialogHeader>
            <DialogTitle>{t("customers_pay")}</DialogTitle>
            <DialogDescription>{t("customers_pay_confirm")}</DialogDescription>
          </DialogHeader>
          <form
            noValidate
            className="flex flex-col gap-4"
            onSubmit={(event) => {
              event.preventDefault();
              void form.handleSubmit();
            }}
          >
            <form.Field
              name="amount"
              validators={{
                onSubmit: ({ value }) => {
                  if (value === null) return "error_payment_amount_invalid";
                  return value <= 0 ? "error_payment_amount_zero" : undefined;
                },
              }}
            >
              {(field) => (
                <FormField
                  label={t("field_payment_amount")}
                  error={said(field.state.meta.errors)}
                >
                  {(parts) => (
                    <MoneyInput
                      {...parts}
                      value={field.state.value}
                      onChange={(next) => {
                        setSaved(false);
                        field.handleChange(next);
                      }}
                    />
                  )}
                </FormField>
              )}
            </form.Field>

            {/* Informational on the movement: no stamp is computed from it.
                The receipt a cash settlement is handed is the comptable's
                question and is not answered here (features.md §2, R8). */}
            <form.Field name="mode">
              {(field) => (
                <ChoiceRow
                  label={t("field_payment_mode")}
                  value={field.state.value}
                  onChange={field.handleChange}
                  options={PAYMENT_METHODS.map((mode) => ({
                    value: mode,
                    label: t(PAYMENT_METHOD_KEY[mode]),
                  }))}
                />
              )}
            </form.Field>

            <form.Field name="note">
              {(field) => (
                <FormField label={t("field_payment_note")}>
                  {(parts) => (
                    <Input
                      {...parts}
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
                {outstanding === null ? null : (
                  <Money centimes={outstanding} className="ms-1" />
                )}
              </p>
            )}

            <DialogFooter>
              <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
                {t("action_cancel")}
              </Button>
              <PayButton type="submit" disabled={pay.isPending}>
                {pay.isPending ? t("action_saving") : t("action_take_payment")}
              </PayButton>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </Card>
  );
}

/** One payment, with the documents the money landed on. The allocations are
 *  shown open rather than behind a toggle: which facture a payment settled is
 *  the question a customer asks at the counter. */
function PaymentRow({ payment }: { payment: PaymentDto }) {
  const { t } = useTranslation();
  return (
    <article
      className="flex flex-col gap-2 rounded-lg border border-border p-3"
      data-testid="customer-payment"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <span dir="ltr" className="font-numeric text-sm text-muted-foreground">
          {payment.created_at}
        </span>
        <span className="text-sm">
          {payment.payment_mode === null ? "" : t(PAYMENT_METHOD_KEY[payment.payment_mode])}
        </span>
        <Money centimes={payment.amount_centimes} className="text-md" />
      </div>
      {payment.note === null ? null : (
        <p className="text-sm text-muted-foreground">{payment.note}</p>
      )}
      <p className="text-sm">{t("customers_payment_settled")}</p>
      {payment.allocations.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t("customers_payment_settled_none")}</p>
      ) : (
        <ul className="flex flex-col gap-1 text-sm">
          {payment.allocations.map((allocation) => (
            <li key={allocation.document_id} className="flex justify-between gap-3">
              <span>
                {t("col_document")}{" "}
                <span className="font-numeric">{allocation.document_id}</span>
              </span>
              <Money centimes={allocation.amount_centimes} />
            </li>
          ))}
        </ul>
      )}
    </article>
  );
}

function StatementPanel({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  const clock = useShopToday();

  return (
    <Card>
      <CardHeader>
        <h3 className="text-md leading-none font-semibold">{t("customers_statement")}</h3>
        <CardDescription>{t("customers_statement_hint")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {/* The range defaults to the shop's own day, which the server owns,
            so the fields wait for it rather than opening on the browser's.
            A refusal is said out loud with a way to ask again: waiting is
            what a call in flight looks like, not what a failed one does. */}
        {clock.error !== null ? (
          <>
            <p role="alert" className="text-sm text-fg-danger">
              {t(errorKey(clock.error))}
            </p>
            <div>
              <Button variant="outline" onClick={clock.retry}>
                {t("action_retry")}
              </Button>
            </div>
          </>
        ) : clock.today === undefined ? (
          <Skeleton className="h-10 w-full" />
        ) : (
          <StatementRange customer={customer} today={clock.today} />
        )}
      </CardContent>
    </Card>
  );
}

/** The range and the page it asks for, once the shop's day is known. */
function StatementRange({ customer, today }: { customer: CustomerDto; today: string }) {
  const { t, lang } = useTranslation();
  const [from, setFrom] = useState(`${today.slice(0, 4)}-01-01`);
  const [to, setTo] = useState(today);
  const [asked, setAsked] = useState<{ from: string; to: string } | null>(null);
  const backwards = from > to;

  const statement = useQuery({
    queryKey: customerStatementQueryKey(customer.id, asked?.from ?? "", asked?.to ?? "", lang),
    queryFn: () => api.customerStatement(customer.id, asked?.from ?? "", asked?.to ?? "", lang),
    enabled: asked !== null,
  });

  return (
    <>
      <div className="flex flex-wrap items-end gap-3">
        <FormField label={t("field_statement_from")}>
          {(parts) => (
            <Input
              {...parts}
              type="date"
              dir="ltr"
              className="font-numeric"
              value={from}
              onChange={(event) => setFrom(event.target.value)}
            />
          )}
        </FormField>
        <FormField label={t("field_statement_to")}>
          {(parts) => (
            <Input
              {...parts}
              type="date"
              dir="ltr"
              className="font-numeric"
              value={to}
              onChange={(event) => setTo(event.target.value)}
            />
          )}
        </FormField>
        <Button
          variant="outline"
          disabled={backwards}
          onClick={() => setAsked(asked === null ? { from, to } : null)}
        >
          {asked === null ? t("action_statement") : t("action_statement_close")}
        </Button>
      </div>
      {/* Caught here as well as by the server: a range the wrong way round is
          a typing mistake, and a call that can only be refused is a call not
          worth making. */}
      {backwards ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t("error_statement_range_invalid")}
        </p>
      ) : null}
      {statement.isPending && asked !== null ? <Skeleton className="h-96 w-full" /> : null}
      {statement.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(statement.error))}
        </p>
      ) : null}
      {asked !== null && statement.isSuccess ? (
        <iframe
          title={t("customers_statement_title")}
          srcDoc={statement.data}
          // An empty sandbox: the page carries no script and needs no origin,
          // so it cannot reach this one even if a customer name ever slipped
          // past the template's escaping.
          sandbox=""
          className="h-96 w-full rounded-lg border border-border bg-card"
          data-testid="customer-statement"
        />
      ) : null}
    </>
  );
}

/**
 * One button and no fields. The slip is about what the customer owes now, so
 * there is no range to pick, and the newest ten movements are the paper's
 * length rather than a choice the screen offers.
 */
function DebtSlipPanel({ customer }: { customer: CustomerDto }) {
  const { t, lang } = useTranslation();
  const [open, setOpen] = useState(false);

  const slip = useQuery({
    queryKey: customerDebtSlipQueryKey(customer.id, lang),
    queryFn: () => api.customerDebtSlip(customer.id, lang),
    enabled: open,
  });

  return (
    <Card>
      <CardHeader>
        <h3 className="text-md leading-none font-semibold">{t("customers_debt_slip")}</h3>
        <CardDescription>{t("customers_debt_slip_hint")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div>
          <Button variant="outline" onClick={() => setOpen(!open)} data-testid="customer-debt-slip-button">
            {open ? t("action_debt_slip_close") : t("action_debt_slip")}
          </Button>
        </div>
        {slip.isPending && open ? <Skeleton className="h-96 w-full" /> : null}
        {slip.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(slip.error))}
          </p>
        ) : null}
        {open && slip.isSuccess ? (
          <iframe
            title={t("customers_debt_slip_title")}
            srcDoc={slip.data}
            // An empty sandbox, for the reason the statement's carries one:
            // the page has no script and needs no origin.
            sandbox=""
            className="h-96 w-full rounded-lg border border-border bg-card"
            data-testid="customer-debt-slip"
          />
        ) : null}
      </CardContent>
    </Card>
  );
}

/** A correction, written as a movement. Asked for first: it lands in the
 *  statement the customer is handed and nothing removes it afterwards. */
/** A signed correction. Zero is refused before it leaves the screen: a
 *  movement of nothing is a line in a statement that says nothing. */
interface AdjustValues {
  amount: number | null;
  note: string;
}

function AdjustPanel({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  const said = useFieldError();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);
  const [saved, setSaved] = useState(false);

  const adjust = useMutation({
    mutationFn: (input: { amount_centimes: number; note: string | null }) =>
      api.adjustCustomerDebt(customer.id, input),
    onSuccess: async (answer: CustomerLedgerDto) => {
      setServerError(null);
      setSaved(true);
      queryClient.setQueryData(customerLedgerQueryKey(customer.id), answer);
      // The balance on the cards above comes from the customers query, which
      // the movement has just changed, and so is the balance on any slip
      // already rendered.
      await queryClient.invalidateQueries({ queryKey: customersQueryKey });
      await queryClient.invalidateQueries({ queryKey: customerDebtSlipKeyPrefix(customer.id) });
    },
    onError: (error: unknown) => {
      setSaved(false);
      setServerError(errorKey(error));
    },
  });

  const blank: AdjustValues = { amount: null, note: "" };
  const form = useForm({
    defaultValues: blank,
    onSubmit: async ({ value }) => {
      if (value.amount === null || value.amount === 0) return;
      if (!window.confirm(t("customers_adjust_confirm"))) return;
      const written = await adjust
        .mutateAsync({ amount_centimes: value.amount, note: cleared(value.note) })
        .then(() => true)
        .catch(() => false);
      // A refused correction keeps what was typed: the error above says what
      // to change, and an empty box means typing the figure again to find out
      // what was wrong with it.
      if (!written) return;
      form.setFieldValue("amount", null);
      form.setFieldValue("note", "");
    },
  });

  return (
    <Card>
      <CardHeader>
        <h3 className="text-md leading-none font-semibold">{t("customers_adjust")}</h3>
        <CardDescription>{t("customers_adjust_hint")}</CardDescription>
      </CardHeader>
      <CardContent>
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            void form.handleSubmit();
          }}
        >
          <div className="grid gap-4 sm:grid-cols-2">
            <form.Field
              name="amount"
              validators={{
                onSubmit: ({ value }) => {
                  if (value === null) return "error_amount_invalid";
                  return value === 0 ? "error_amount_zero" : undefined;
                },
              }}
            >
              {(field) => (
                <FormField label={t("field_adjust_amount")} error={said(field.state.meta.errors)}>
                  {(parts) => (
                    <MoneyInput
                      {...parts}
                      value={field.state.value}
                      onChange={(next) => {
                        setSaved(false);
                        field.handleChange(next);
                      }}
                    />
                  )}
                </FormField>
              )}
            </form.Field>

            <form.Field name="note">
              {(field) => (
                <FormField label={t("field_adjust_note")}>
                  {(parts) => (
                    <Input
                      {...parts}
                      value={field.state.value}
                      onChange={(event) => field.handleChange(event.target.value)}
                    />
                  )}
                </FormField>
              )}
            </form.Field>
          </div>

          {serverError === null ? null : (
            <p role="alert" className="text-sm text-fg-danger">
              {t(serverError)}
            </p>
          )}
          {saved && serverError === null ? (
            <p role="status" className="text-sm text-fg-success">
              {t("customers_adjusted")}
            </p>
          ) : null}

          <div>
            <Button type="submit" disabled={adjust.isPending}>
              {adjust.isPending ? t("action_saving") : t("action_adjust")}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}
