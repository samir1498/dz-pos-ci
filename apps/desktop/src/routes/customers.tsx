// The customers screen: who the shop sells to, what each one owes, and the
// movements behind that figure. Everything it shows comes from the API over
// HTTP and the balance is the core's, never added up here.
//
// Nothing on this screen deletes a customer: the ledger holds the fiche, so a
// shop that has stopped dealing with somebody clears the active box instead,
// and the till's picker then leaves them out.

import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { useState } from "react";
import { ApiError, formatCentimes, parseAmountToCentimes } from "@dzpos/shared";
import type {
  CustomerDto,
  CustomerLedgerDto,
  CustomerPaymentsDto,
  CustomerWriteDto,
  DebtKindDto,
  NewCustomerDto,
  PartyKindDto,
  PaymentDto,
  PaymentMethodDto,
} from "@dzpos/shared";
import {
  api,
  customerDebtSlipKeyPrefix,
  customerDebtSlipQueryKey,
  customerLedgerQueryKey,
  customerQueryKey,
  customerPaymentsQueryKey,
  customerStatementQueryKey,
  customersQueryKey,
} from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { useShopToday } from "@/lib/clock";
import {
  AmountField,
  FieldError,
  amount,
  cleared,
  errorKey,
  readable,
  shownPositive,
} from "@/lib/fields";

export const Route = createFileRoute("/customers")({ component: CustomersScreen });

const PARTY_KINDS: readonly PartyKindDto[] = ["company", "consumer"];

const PARTY_KEY: Record<PartyKindDto, Key> = {
  company: "party_company",
  consumer: "party_consumer",
};

const PAYMENT_METHODS: readonly PaymentMethodDto[] = ["cash", "card"];

const PAYMENT_METHOD_KEY: Record<PaymentMethodDto, Key> = {
  cash: "payment_cash",
  card: "payment_card",
};

const DEBT_KIND_KEY: Record<DebtKindDto, Key> = {
  opening: "debt_opening",
  sale: "debt_sale",
  payment: "debt_payment",
  avoir: "debt_avoir",
  adjustment: "debt_adjustment",
};

/**
 * The tag the list shows for one customer, in the order the shop reads them:
 * a balance past the limit is the fact that stops a sale, and it outranks the
 * others even when the limit is zero. Then no credit at all (a zero limit,
 * which is not the same answer as no limit), then the warning threshold.
 *
 * features.md §2 names the thresholds; which one wins when two apply is a
 * decision this screen takes.
 */
/** What the fiche and the list call the stored balance and how big it reads.
 *
 * A customer's balance is one signed number: positive is what they owe, and
 * an avoir past what was outstanding drives it below zero, at which point the
 * shop is holding their money. "Créance -1 000,00" is not a sentence anyone
 * says at a counter, so the sign is spent on the word instead of on the
 * figure and the amount is always shown positive.
 *
 * features.md §3 (avoir) is where a negative balance comes from.
 */
export function balanceLabel(balance_centimes: number): Key {
  return balance_centimes < 0 ? "customers_credit" : "customers_balance";
}

export function balanceShown(balance_centimes: number): string {
  return shownPositive(balance_centimes);
}

export function statusKey(customer: CustomerDto): Key {
  const { balance_centimes, credit_limit_centimes, warn_threshold_centimes } = customer;
  if (credit_limit_centimes !== null && balance_centimes > credit_limit_centimes) {
    return "status_over_limit";
  }
  if (credit_limit_centimes === 0) return "status_no_credit";
  if (warn_threshold_centimes !== null && balance_centimes >= warn_threshold_centimes) {
    return "status_near_limit";
  }
  return "status_ok";
}

export function CustomersScreen() {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  // One panel, two jobs: "new" is the blank fiche, a customer is that
  // customer's own, with the ledger under it.
  const [open, setOpen] = useState<"new" | CustomerDto | null>(null);
  const customers = useQuery({
    queryKey: [...customersQueryKey, search.trim()],
    queryFn: () => api.listCustomers(search),
  });

  // The panel reads the row from the list, so it shows the balance the last
  // answer carried rather than the one it was opened with.
  const opened =
    open === null || open === "new"
      ? open
      : (customers.data?.find((c) => c.id === open.id) ?? open);

  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("customers_title")}</h1>
        <button
          type="button"
          className="rounded border px-3 py-1.5"
          onClick={() => setOpen((current) => (current === null ? "new" : null))}
        >
          {open !== null ? t("action_cancel") : t("customers_add")}
        </button>
      </header>

      <label className="flex flex-col gap-1">
        <span>{t("customers_search")}</span>
        <input
          type="search"
          className="rounded border px-2 py-1"
          placeholder={t("customers_search_hint")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </label>

      {opened === "new" ? (
        <CustomerForm key="new" initial={null} onDone={() => setOpen(null)} />
      ) : null}
      {opened !== null && opened !== "new" ? (
        <div className="flex flex-col gap-4 rounded border p-4">
          <CustomerForm key={opened.id} initial={opened} onDone={() => setOpen(null)} />
          <CustomerLedger customer={opened} />
        </div>
      ) : null}

      {customers.isPending ? <p>{t("customers_loading")}</p> : null}
      {customers.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(customers.error))}
        </p>
      ) : null}
      {customers.isSuccess ? (
        <CustomerTable rows={customers.data} onEdit={(row) => setOpen(row)} />
      ) : null}
    </section>
  );
}

/**
 * One fiche on a page of its own, which is what `/customers/$id` opens. The
 * till's credit refusal names a customer whose balance stopped a sale, and
 * the documents screen names the customer a facture was made out to; neither
 * can reach into the list screen's state to open the panel there, and a link
 * that dropped the cashier on the list with a search box to retype would be
 * the shop doing the app's work.
 *
 * The same two components the panel uses, so a fiche reads the same whichever
 * way it was opened.
 */
export function CustomerFiche({ id }: { id: number }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const customer = useQuery({
    queryKey: customerQueryKey(id),
    queryFn: () => api.getCustomer(id),
  });
  const back = () => void navigate({ to: "/customers" });

  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("customers_title")}</h1>
        <Link to="/customers" className="underline">
          {t("action_back_to_customers")}
        </Link>
      </header>

      {customer.isPending ? <p>{t("customers_loading")}</p> : null}
      {customer.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(customer.error))}
        </p>
      ) : null}
      {customer.isSuccess ? (
        <div className="flex flex-col gap-4 rounded border p-4">
          <CustomerForm key={customer.data.id} initial={customer.data} onDone={back} />
          <CustomerLedger customer={customer.data} />
        </div>
      ) : null}
    </section>
  );
}

function CustomerTable({
  rows,
  onEdit,
}: {
  rows: CustomerDto[];
  onEdit: (row: CustomerDto) => void;
}) {
  const { t } = useTranslation();
  if (rows.length === 0) return <p>{t("customers_empty")}</p>;
  return (
    <table className="w-full text-start">
      <caption className="sr-only">{t("customers_title")}</caption>
      <thead>
        <tr>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_name")}</th>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_phone")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_debt")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_limit")}</th>
          <th scope="col" className="text-start pb-2 ps-3 pe-3">{t("col_status")}</th>
          <th scope="col" className="pb-2">
            <span className="sr-only">{t("customers_edit")}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {rows.map((c) => (
          <tr key={c.id} className={c.active ? "border-t" : "border-t opacity-60"}>
            <td className="py-1.5 pe-3">
              {c.name}
              {c.active ? null : (
                <span className="ms-2 rounded border px-1 text-xs uppercase">
                  {t("customers_inactive")}
                </span>
              )}
            </td>
            {/* dir="ltr" on the number itself, not on the cell: a phone and
                an amount are read left to right with Western digits whatever
                the screen's language, and without it the bidi algorithm is
                free to reorder the sign and the groups inside an RTL row.
                On the cell it would also flip which side the cell's own
                padding lands on, and two columns would touch. */}
            <td className="py-1.5 pe-3 font-mono">
              <span dir="ltr">{c.phone ?? ""}</span>
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">{balanceShown(c.balance_centimes)}</span>
              {c.balance_centimes < 0 ? (
                <span className="ms-1 rounded border px-1 font-sans text-sm">
                  {t("customers_credit")}
                </span>
              ) : null}
            </td>
            <td className="py-1.5 ps-3 pe-3 text-end font-mono">
              <span dir="ltr">
                {c.credit_limit_centimes === null ? "" : formatCentimes(c.credit_limit_centimes)}
              </span>
            </td>
            <td className="py-1.5 pe-3">
              <span className="rounded border px-1 text-sm">{t(statusKey(c))}</span>
            </td>
            <td className="py-1.5 ps-3 text-end">
              <button
                type="button"
                className="rounded border px-2 py-0.5 text-sm"
                aria-label={`${t("customers_edit")} ${c.name}`}
                onClick={() => onEdit(c)}
              >
                {t("customers_edit")}
              </button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/**
 * The blank fiche and an existing one are the same form: the same fields and
 * the same request shape either way. The opening debt is the one difference,
 * and it is only on the blank one: it is a ledger movement, not a column, so
 * an edit that could set it would be a correction nobody could see
 * (features.md §2).
 */
function CustomerForm({
  initial,
  onDone,
}: {
  initial: CustomerDto | null;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);
  // The server refuses a close over an open account without a reason
  // (features.md §2). The screen asks for one whenever it can see the account
  // is open, and this flag is for when it cannot: a fiche whose balance is
  // nil can still have a document asking to be paid, and the refusal is what
  // says so.
  const [reasonRefused, setReasonRefused] = useState(false);

  const save = useMutation({
    mutationFn: ({
      close_reason,
      ...fiche
    }: NewCustomerDto & { close_reason: string | null }) =>
      // A blank fiche closes nothing, and `POST /customers` refuses a field
      // it does not know, so the reason only travels on the update.
      initial === null
        ? api.createCustomer(fiche)
        : api.updateCustomer(initial.id, whole({ ...fiche, close_reason })),
    onSuccess: async () => {
      setServerError(null);
      setReasonRefused(false);
      await queryClient.invalidateQueries({ queryKey: customersQueryKey });
      onDone();
    },
    onError: (error: unknown) => {
      setServerError(errorKey(error));
      setReasonRefused(error instanceof ApiError && error.field === "reason");
    },
  });

  const form = useForm({
    defaultValues:
      initial === null
        ? {
            name: "",
            partyKind: "company",
            phone: "",
            address: "",
            rc: "",
            nif: "",
            nis: "",
            ai: "",
            creditLimit: "",
            warnThreshold: "",
            openingDebt: "",
            notes: "",
            active: true,
            closeReason: "",
          }
        : {
            name: initial.name,
            partyKind: initial.party_kind,
            phone: initial.phone ?? "",
            address: initial.address ?? "",
            rc: initial.rc ?? "",
            nif: initial.nif ?? "",
            nis: initial.nis ?? "",
            ai: initial.ai ?? "",
            creditLimit:
              initial.credit_limit_centimes === null
                ? ""
                : formatCentimes(initial.credit_limit_centimes),
            warnThreshold:
              initial.warn_threshold_centimes === null
                ? ""
                : formatCentimes(initial.warn_threshold_centimes),
            openingDebt: "",
            notes: initial.notes ?? "",
            active: initial.active,
            closeReason: "",
          },
    onSubmit: async ({ value }) => {
      // The rejection is swallowed on purpose: onError has already turned the
      // server's code into a translated message on the form.
      await save
        .mutateAsync({
          name: value.name,
          party_kind: toPartyKind(value.partyKind),
          phone: cleared(value.phone),
          address: cleared(value.address),
          rc: cleared(value.rc),
          nif: cleared(value.nif),
          nis: cleared(value.nis),
          ai: cleared(value.ai),
          // A blank limit is "no limit at all", which is not a limit of
          // nothing: the two are different answers and the till acts on them
          // differently.
          credit_limit_centimes: amount(value.creditLimit),
          warn_threshold_centimes: amount(value.warnThreshold),
          notes: cleared(value.notes),
          active: value.active,
          opening_debt_centimes: initial === null ? amount(value.openingDebt) : null,
          close_reason: cleared(value.closeReason),
        })
        .catch(() => undefined);
    },
  });

  /** One optional text field of the fiche: a phone, an identifier, a note.
   *  Blank clears the column (`cleared` below). */
  const optionalText = (
    name: "phone" | "address" | "rc" | "nif" | "nis" | "ai" | "notes",
    label: string,
    mono = false,
  ) => (
    <form.Field name={name}>
      {(field) => (
        <label className="flex flex-col gap-1">
          <span>{label}</span>
          <input
            dir={mono ? "ltr" : undefined}
            className={mono ? "rounded border px-2 py-1 font-mono" : "rounded border px-2 py-1"}
            value={field.state.value}
            onChange={(e) => field.handleChange(e.target.value)}
          />
        </label>
      )}
    </form.Field>
  );

  return (
    <form
      noValidate
      className="flex flex-col gap-3 rounded border p-4"
      onSubmit={(e) => {
        e.preventDefault();
        void form.handleSubmit();
      }}
    >
      <h2 className="font-semibold">{initial === null ? t("customers_new") : initial.name}</h2>

      <form.Field
        name="name"
        validators={{
          onSubmit: ({ value }) => (value.trim() === "" ? "error_name_required" : undefined),
        }}
      >
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_name")}</span>
            <input
              className="rounded border px-2 py-1"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
              onBlur={field.handleBlur}
            />
            <FieldError messages={field.state.meta.errors} />
          </label>
        )}
      </form.Field>

      <form.Field name="partyKind">
        {(field) => (
          <fieldset className="flex flex-col gap-1">
            {/* Asked for, never inferred from whether an RC was typed in:
                loi 04-02 art. 10 decides ticket against facture by who the
                buyer is (features.md §2). */}
            <legend>{t("field_party_kind")}</legend>
            <div className="flex gap-4">
              {PARTY_KINDS.map((kind) => (
                <label key={kind} className="flex items-center gap-2">
                  <input
                    type="radio"
                    name="party_kind"
                    value={kind}
                    checked={field.state.value === kind}
                    onChange={() => field.handleChange(kind)}
                  />
                  <span>{t(PARTY_KEY[kind])}</span>
                </label>
              ))}
            </div>
          </fieldset>
        )}
      </form.Field>

      {optionalText("phone", t("field_phone"), true)}
      {optionalText("address", t("field_address"))}
      {optionalText("rc", t("field_rc"), true)}
      {optionalText("nif", t("field_nif"), true)}
      {optionalText("nis", t("field_nis"), true)}
      {optionalText("ai", t("field_ai"), true)}

      <form.Field
        name="creditLimit"
        validators={{
          onSubmit: ({ value }) =>
            readable(value) ? undefined : "error_credit_limit_invalid",
        }}
      >
        {(field) => (
          <AmountField
            label={t("field_credit_limit")}
            value={field.state.value}
            onChange={field.handleChange}
            errors={field.state.meta.errors}
          />
        )}
      </form.Field>

      <form.Field
        name="warnThreshold"
        validators={{
          onSubmit: ({ value }) =>
            readable(value) ? undefined : "error_warn_threshold_invalid",
        }}
      >
        {(field) => (
          <AmountField
            label={t("field_warn_threshold")}
            value={field.state.value}
            onChange={field.handleChange}
            errors={field.state.meta.errors}
          />
        )}
      </form.Field>

      {initial === null ? (
        <form.Field
          name="openingDebt"
          validators={{
            onSubmit: ({ value }) =>
              readable(value) ? undefined : "error_opening_debt_invalid",
          }}
        >
          {(field) => (
            <AmountField
              label={t("field_opening_debt")}
              hint={t("field_opening_debt_hint")}
              value={field.state.value}
              onChange={field.handleChange}
              errors={field.state.meta.errors}
            />
          )}
        </form.Field>
      ) : null}

      {optionalText("notes", t("field_notes"))}

      <form.Field name="active">
        {(field) => (
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={field.state.value}
              onChange={(e) => field.handleChange(e.target.checked)}
            />
            <span>{t("field_customer_active")}</span>
          </label>
        )}
      </form.Field>

      {/* Closing a fiche says the shop has stopped trading with that customer,
          and doing it over an account that is still open is a decision rather
          than a tidy-up: the reason goes in the audit log beside the balance
          (features.md §2). The block appears when the balance says the account
          is open, and when the server says so about a document the screen
          cannot see. */}
      <form.Subscribe selector={(state) => state.values.active}>
        {(active) =>
          initial !== null &&
          initial.active &&
          !active &&
          (initial.balance_centimes !== 0 || reasonRefused) ? (
            <form.Field name="closeReason">
              {(field) => (
                <label className="flex flex-col gap-1 rounded border border-amber-600 p-2">
                  <span>{t("customers_close_reason")}</span>
                  <span className="text-sm">
                    {t("customers_close_reason_hint")}{" "}
                    {t(balanceLabel(initial.balance_centimes))}{" "}
                    {balanceShown(initial.balance_centimes)}
                  </span>
                  <input
                    data-testid="customer-close-reason"
                    className="rounded border px-2 py-1"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                  />
                </label>
              )}
            </form.Field>
          ) : null
        }
      </form.Subscribe>

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}

      <div className="flex gap-2">
        <button type="submit" className="rounded border px-3 py-1.5" disabled={save.isPending}>
          {save.isPending ? t("action_saving") : t("action_save")}
        </button>
        <button type="button" className="rounded border px-3 py-1.5" onClick={onDone}>
          {t("action_cancel")}
        </button>
      </div>
    </form>
  );
}

/** The movements, and the form that writes one. */
function CustomerLedger({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  const ledger = useQuery({
    queryKey: customerLedgerQueryKey(customer.id),
    queryFn: () => api.customerLedger(customer.id),
  });

  return (
    <section className="flex flex-col gap-3">
      <h2 className="font-semibold">{t("customers_ledger")}</h2>
      <p>
        <span>{t(balanceLabel(customer.balance_centimes))} </span>
        <span className="font-mono" dir="ltr">
          {balanceShown(customer.balance_centimes)}
        </span>
      </p>

      {ledger.isPending ? <p>{t("customers_loading")}</p> : null}
      {ledger.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(ledger.error))}
        </p>
      ) : null}
      {ledger.isSuccess ? <LedgerTable ledger={ledger.data} /> : null}

      <PaymentForm customer={customer} />
      <PaymentsList customer={customer} />
      <StatementPanel customer={customer} />
      <DebtSlipPanel customer={customer} />
      <AdjustForm customer={customer} />
    </section>
  );
}

/**
 * Money against the debt. The server settles the oldest documents first and
 * refuses a payment above what the customer owes, so nothing here caps the
 * figure or picks the documents: a screen that decided either would be a
 * second answer to what the customer owes.
 */
function PaymentForm({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);
  /** What the customer actually owes, as the refused payment reported it.
   *  "Too much" is useless without the amount that would not have been. */
  const [outstanding, setOutstanding] = useState<number | null>(null);
  const [saved, setSaved] = useState(false);

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
      queryClient.setQueryData(customerPaymentsQueryKey(customer.id), answer);
      // The movement is on the ledger too, and the balance on the list above
      // came from the customers query. The slip is a rendered page carrying
      // the old balance, in whichever languages it has been asked for.
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

  const form = useForm({
    defaultValues: { amount: "", mode: "cash", note: "" },
    onSubmit: async ({ value }) => {
      const centimes = parseAmountToCentimes(value.amount);
      if (centimes === null || centimes <= 0) return;
      if (!window.confirm(t("customers_pay_confirm"))) return;
      const written = await pay
        .mutateAsync({
          amount_centimes: centimes,
          payment_mode: toPaymentMethod(value.mode),
          note: cleared(value.note),
        })
        .then(() => true)
        .catch(() => false);
      // A refused payment keeps what was typed: the error above says what is
      // outstanding, and an emptied box means typing the figure again to find
      // out what was wrong with it.
      if (!written) return;
      form.setFieldValue("amount", "");
      form.setFieldValue("note", "");
    },
  });

  return (
    <form
      noValidate
      className="flex flex-col gap-3 rounded border p-3"
      onSubmit={(e) => {
        e.preventDefault();
        void form.handleSubmit();
      }}
    >
      <h3 className="font-semibold">{t("customers_pay")}</h3>
      <p className="text-sm opacity-70">{t("customers_pay_hint")}</p>
      {/* A closed fiche keeps this form: the shop stopped selling to this
          customer, not collecting from them (core, services::debt). */}
      {customer.active ? null : (
        <p className="text-sm opacity-70">{t("customers_closed_still_collects")}</p>
      )}

      <form.Field
        name="amount"
        validators={{
          onSubmit: ({ value }) => {
            const centimes = parseAmountToCentimes(value);
            if (centimes === null) return "error_payment_amount_invalid";
            return centimes <= 0 ? "error_payment_amount_zero" : undefined;
          },
        }}
      >
        {(field) => (
          <AmountField
            label={t("field_payment_amount")}
            value={field.state.value}
            onChange={(next) => {
              setSaved(false);
              field.handleChange(next);
            }}
            errors={field.state.meta.errors}
          />
        )}
      </form.Field>

      <form.Field name="mode">
        {(field) => (
          <fieldset className="flex flex-col gap-1">
            {/* Informational on the movement: no stamp is computed from it.
                The receipt a cash settlement is handed is the comptable's
                question and is not answered here (features.md §2, R8). */}
            <legend>{t("field_payment_mode")}</legend>
            <div className="flex gap-4">
              {PAYMENT_METHODS.map((mode) => (
                <label key={mode} className="flex items-center gap-2">
                  <input
                    type="radio"
                    name="payment_mode"
                    value={mode}
                    checked={field.state.value === mode}
                    onChange={() => field.handleChange(mode)}
                  />
                  <span>{t(PAYMENT_METHOD_KEY[mode])}</span>
                </label>
              ))}
            </div>
          </fieldset>
        )}
      </form.Field>

      <form.Field name="note">
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_payment_note")}</span>
            <input
              className="rounded border px-2 py-1"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
          </label>
        )}
      </form.Field>

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
          {outstanding === null ? null : (
            <span className="ms-1 font-mono" dir="ltr">
              {formatCentimes(outstanding)}
            </span>
          )}
        </p>
      ) : null}
      {saved && serverError === null ? <p role="status">{t("customers_paid")}</p> : null}

      <div>
        <button type="submit" className="rounded border px-3 py-1.5" disabled={pay.isPending}>
          {pay.isPending ? t("action_saving") : t("action_take_payment")}
        </button>
      </div>
    </form>
  );
}

/** The payments, each with the documents the money landed on. The allocations
 *  are shown open rather than behind a toggle: which facture a payment settled
 *  is the question a customer asks at the counter. */
function PaymentsList({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  const payments = useQuery({
    queryKey: customerPaymentsQueryKey(customer.id),
    queryFn: () => api.customerPayments(customer.id),
  });

  return (
    <section className="flex flex-col gap-2">
      <h3 className="font-semibold">{t("customers_payments")}</h3>
      {payments.isPending ? <p>{t("customers_loading")}</p> : null}
      {payments.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(payments.error))}
        </p>
      ) : null}
      {payments.isSuccess && payments.data.payments.length === 0 ? (
        <p>{t("customers_payments_empty")}</p>
      ) : null}
      {payments.isSuccess
        ? payments.data.payments.map((payment) => (
            <PaymentRow key={payment.ledger_id} payment={payment} />
          ))
        : null}
    </section>
  );
}

function PaymentRow({ payment }: { payment: PaymentDto }) {
  const { t } = useTranslation();
  return (
    <article className="rounded border p-2" data-testid="customer-payment">
      <div className="flex justify-between gap-3">
        <span className="font-mono" dir="ltr">
          {payment.created_at}
        </span>
        <span>
          {payment.payment_mode === null ? "" : t(PAYMENT_METHOD_KEY[payment.payment_mode])}
        </span>
        <span className="font-mono" dir="ltr">
          {formatCentimes(payment.amount_centimes)}
        </span>
      </div>
      {payment.note === null ? null : <p className="text-sm opacity-70">{payment.note}</p>}
      <p className="text-sm">{t("customers_payment_settled")}</p>
      {payment.allocations.length === 0 ? (
        <p className="text-sm opacity-70">{t("customers_payment_settled_none")}</p>
      ) : (
        <ul className="text-sm">
          {payment.allocations.map((allocation) => (
            <li key={allocation.document_id} className="flex justify-between gap-3">
              <span>
                {t("col_document")} <span className="font-mono">{allocation.document_id}</span>
              </span>
              <span className="font-mono" dir="ltr">
                {formatCentimes(allocation.amount_centimes)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </article>
  );
}

/**
 * The statement itself, not a screen that resembles it. The core renders the
 * page from the ledger and it goes into an iframe as it came, the way the
 * till shows a ticket: the shop is looking at what the printer will put on
 * paper rather than at a second rendering of the same balances.
 */
function StatementPanel({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  const clock = useShopToday();

  return (
    <section className="flex flex-col gap-2 rounded border p-3">
      <h3 className="font-semibold">{t("customers_statement")}</h3>
      <p className="text-sm opacity-70">{t("customers_statement_hint")}</p>
      {/* The range defaults to the shop's own day, which the server owns,
          so the fields wait for it rather than opening on the browser's.
          A refusal is said out loud with a way to ask again: waiting is
          what a call in flight looks like, not what a failed one does. */}
      {clock.error !== null ? (
        <>
          <p role="alert" className="text-red-700">
            {t(errorKey(clock.error))}
          </p>
          <button
            type="button"
            className="self-start rounded border px-3 py-1.5"
            onClick={clock.retry}
          >
            {t("action_retry")}
          </button>
        </>
      ) : clock.today === undefined ? (
        <p>{t("customers_loading")}</p>
      ) : (
        <StatementRange customer={customer} today={clock.today} />
      )}
    </section>
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
    queryFn: () =>
      api.customerStatement(customer.id, asked?.from ?? "", asked?.to ?? "", lang),
    enabled: asked !== null,
  });

  return (
    <>
      <div className="flex flex-wrap items-end gap-3">
        <label className="flex flex-col gap-1">
          <span>{t("field_statement_from")}</span>
          <input
            type="date"
            dir="ltr"
            className="rounded border px-2 py-1 font-mono"
            value={from}
            onChange={(e) => setFrom(e.target.value)}
          />
        </label>
        <label className="flex flex-col gap-1">
          <span>{t("field_statement_to")}</span>
          <input
            type="date"
            dir="ltr"
            className="rounded border px-2 py-1 font-mono"
            value={to}
            onChange={(e) => setTo(e.target.value)}
          />
        </label>
        <button
          type="button"
          className="rounded border px-3 py-1.5"
          disabled={backwards}
          onClick={() => setAsked(asked === null ? { from, to } : null)}
        >
          {asked === null ? t("action_statement") : t("action_statement_close")}
        </button>
      </div>
      {/* Caught here as well as by the server: a range the wrong way round is
          a typing mistake, and a call that can only be refused is a call not
          worth making. */}
      {backwards ? (
        <p role="alert" className="text-red-700">
          {t("error_statement_range_invalid")}
        </p>
      ) : null}
      {statement.isPending && asked !== null ? <p>{t("customers_loading")}</p> : null}
      {statement.isError ? (
        <p role="alert" className="text-red-700">
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
          className="h-96 w-full border-0"
          data-testid="customer-statement"
        />
      ) : null}
    </>
  );
}

/**
 * The debt slip itself, the way the statement panel above shows the
 * statement: the core renders the 80 mm page and it goes into an iframe as it
 * came, so the shop is looking at what the printer will put on paper.
 *
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
    <section className="flex flex-col gap-2 rounded border p-3">
      <h3 className="font-semibold">{t("customers_debt_slip")}</h3>
      <p className="text-sm opacity-70">{t("customers_debt_slip_hint")}</p>
      <div>
        <button
          type="button"
          className="rounded border px-3 py-1.5"
          onClick={() => setOpen(!open)}
          data-testid="customer-debt-slip-button"
        >
          {open ? t("action_debt_slip_close") : t("action_debt_slip")}
        </button>
      </div>
      {slip.isPending && open ? <p>{t("customers_loading")}</p> : null}
      {slip.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(slip.error))}
        </p>
      ) : null}
      {open && slip.isSuccess ? (
        <iframe
          title={t("customers_debt_slip_title")}
          srcDoc={slip.data}
          // An empty sandbox, for the reason the statement's carries one: the
          // page has no script and needs no origin.
          sandbox=""
          className="h-96 w-full border-0"
          data-testid="customer-debt-slip"
        />
      ) : null}
    </section>
  );
}

function LedgerTable({ ledger }: { ledger: CustomerLedgerDto }) {
  const { t } = useTranslation();
  if (ledger.entries.length === 0) return <p>{t("customers_ledger_empty")}</p>;
  return (
    <table className="w-full text-start">
      <caption className="sr-only">{t("customers_ledger")}</caption>
      <thead>
        <tr>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_date")}</th>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_kind")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_debit")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_credit")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_balance")}</th>
          <th scope="col" className="text-start pb-2 ps-3">{t("col_note")}</th>
        </tr>
      </thead>
      <tbody>
        {ledger.entries.map((entry) => (
          <tr key={entry.id} className="border-t">
            <td className="py-1.5 pe-3 font-mono">
              <span dir="ltr">{entry.created_at}</span>
            </td>
            <td className="py-1.5 pe-3">{t(DEBT_KIND_KEY[entry.kind])}</td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">
                {entry.debit_centimes === 0 ? "" : formatCentimes(entry.debit_centimes)}
              </span>
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">
                {entry.credit_centimes === 0 ? "" : formatCentimes(entry.credit_centimes)}
              </span>
            </td>
            {/* The running balance is the core's (services::debt): a column
                added up here would be a second answer to what a customer
                owes. */}
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">{formatCentimes(entry.balance_after_centimes)}</span>
            </td>
            <td className="py-1.5 ps-3">{entry.note ?? ""}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** A correction, written as a movement. Asked for first: it lands in the
 *  statement the customer is handed and nothing removes it afterwards. */
function AdjustForm({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
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
      // The balance on the list and on the fiche above it comes from the
      // customers query, which the movement has just changed, and so is the
      // balance on any slip already rendered.
      await queryClient.invalidateQueries({ queryKey: customersQueryKey });
      await queryClient.invalidateQueries({ queryKey: customerDebtSlipKeyPrefix(customer.id) });
    },
    onError: (error: unknown) => {
      setSaved(false);
      setServerError(errorKey(error));
    },
  });

  const form = useForm({
    defaultValues: { amount: "", note: "" },
    onSubmit: async ({ value }) => {
      const centimes = parseAmountToCentimes(value.amount);
      if (centimes === null || centimes === 0) return;
      if (!window.confirm(t("customers_adjust_confirm"))) return;
      const written = await adjust
        .mutateAsync({ amount_centimes: centimes, note: cleared(value.note) })
        .then(() => true)
        .catch(() => false);
      // A refused correction keeps what was typed: the error above says what
      // to change, and an empty box means typing the figure again to find
      // out what was wrong with it.
      if (!written) return;
      form.setFieldValue("amount", "");
      form.setFieldValue("note", "");
    },
  });

  return (
    <form
      noValidate
      className="flex flex-col gap-3 rounded border p-3"
      onSubmit={(e) => {
        e.preventDefault();
        void form.handleSubmit();
      }}
    >
      <h3 className="font-semibold">{t("customers_adjust")}</h3>
      <p className="text-sm opacity-70">{t("customers_adjust_hint")}</p>

      <form.Field
        name="amount"
        validators={{
          onSubmit: ({ value }) => {
            const centimes = parseAmountToCentimes(value);
            if (centimes === null) return "error_amount_invalid";
            return centimes === 0 ? "error_amount_zero" : undefined;
          },
        }}
      >
        {(field) => (
          <AmountField
            label={t("field_adjust_amount")}
            value={field.state.value}
            onChange={(next) => {
              setSaved(false);
              field.handleChange(next);
            }}
            errors={field.state.meta.errors}
          />
        )}
      </form.Field>

      <form.Field name="note">
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_adjust_note")}</span>
            <input
              className="rounded border px-2 py-1"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
          </label>
        )}
      </form.Field>

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}
      {saved && serverError === null ? <p role="status">{t("customers_adjusted")}</p> : null}

      <div>
        <button type="submit" className="rounded border px-3 py-1.5" disabled={adjust.isPending}>
          {adjust.isPending ? t("action_saving") : t("action_adjust")}
        </button>
      </div>
    </form>
  );
}

/** The update takes the fiche without the opening debt: the create type
 *  carries it and the ledger is never edited. */
/** The update body: the whole fiche without the opening debt, which is a
 *  create-only field, plus the reason a close over an open account needs. */
function whole(input: NewCustomerDto & { close_reason: string | null }): CustomerWriteDto {
  const { opening_debt_centimes: _opening, ...fiche } = input;
  return fiche;
}

function toPartyKind(value: string): PartyKindDto {
  const found = PARTY_KINDS.find((k) => k === value);
  return found ?? "company";
}

/** Cash unless the form says otherwise: the radio group has no third option,
 *  and a payment mode is never guessed from an unknown string. */
function toPaymentMethod(value: string): PaymentMethodDto {
  const found = PAYMENT_METHODS.find((m) => m === value);
  return found ?? "cash";
}
