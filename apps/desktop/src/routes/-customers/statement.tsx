// The movements behind a customer's balance, the money taken against it, and
// the one correction the counter can write onto it. `customers_.$id.tsx`
// composes these three cards under the balance figures; nothing here is
// worked out on this screen; the balance and the running column of the
// ledger are the core's (`services::debt`).

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
import { Receipt, ScrollText, Wallet } from "lucide-react";
import { useState } from "react";

import { ChoiceRow } from "@/components/ChoiceRow";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
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
  customerLedgerQueryKey,
  customerPaymentsQueryKey,
  customersQueryKey,
} from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { cleared, errorKey } from "@/lib/fields";
import { PAYMENT_METHODS, PAYMENT_METHOD_KEY } from "@/lib/payment";

import { DEBT_KIND_KEY, useFieldError } from "./parts";

/** The movements, newest first, with the balance each one left behind. */
export function Ledger({ customer }: { customer: CustomerDto }) {
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

export function PaymentsPanel({ customer }: { customer: CustomerDto }) {
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

/** A correction, written as a movement. Asked for first: it lands in the
 *  statement the customer is handed and nothing removes it afterwards. */
/** A signed correction. Zero is refused before it leaves the screen: a
 *  movement of nothing is a line in a statement that says nothing. */
interface AdjustValues {
  amount: number | null;
  note: string;
}

export function AdjustPanel({ customer }: { customer: CustomerDto }) {
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

  /** The correction the shop has typed and not yet agreed to. The question
   * is asked in a dialog of ours rather than a browser `confirm()`, which is
   * a box Windows draws in the language Windows is in and which does not
   * mirror on an Arabic screen. Submitting only opens it; the write happens
   * when the shop says yes, on the values the form was holding then. */
  const [asking, setAsking] = useState<AdjustValues | null>(null);

  const blank: AdjustValues = { amount: null, note: "" };
  const form = useForm({
    defaultValues: blank,
    onSubmit: ({ value }) => {
      if (value.amount === null || value.amount === 0) return;
      setAsking(value);
    },
  });

  const write = async () => {
    if (asking === null || asking.amount === null) return;
    // The question stays up until the correction has landed or been refused,
    // so a second press cannot write a second correction to what a customer
    // owes and closing the box cannot be read as having cancelled it.
    const written = await adjust
      .mutateAsync({ amount_centimes: asking.amount, note: cleared(asking.note) })
      .then(() => true)
      .catch(() => false);
    setAsking(null);
    // A refused correction keeps what was typed: the error above says what
    // to change, and an empty box means typing the figure again to find out
    // what was wrong with it.
    if (!written) return;
    form.setFieldValue("amount", null);
    form.setFieldValue("note", "");
  };

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

        <ConfirmDialog
          data-testid="customer-adjust-dialog"
          open={asking !== null}
          onCancel={() => setAsking(null)}
          onConfirm={() => void write()}
          title="customers_adjust"
          question="customers_adjust_confirm"
          confirm="action_adjust"
          pending={adjust.isPending}
        >
          {/* The figure itself, because a correction to what a customer owes
              is the one number the shop is agreeing to. */}
          {asking?.amount === null || asking === null ? null : (
            <p className="flex items-center justify-between gap-2 text-sm">
              <span className="text-muted-foreground">{t("field_adjust_amount")}</span>
              <Money centimes={asking.amount} data-testid="customer-adjust-asked" />
            </p>
          )}
        </ConfirmDialog>
      </CardContent>
    </Card>
  );
}
