// The customers screen: who the shop sells to, what each one owes, and the
// movements behind that figure. Everything it shows comes from the API over
// HTTP and the balance is the core's, never added up here.
//
// Nothing on this screen deletes a customer: the ledger holds the fiche, so a
// shop that has stopped dealing with somebody clears the active box instead,
// and the till's picker (T3) then leaves them out.

import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { useState } from "react";
import { ApiError, formatCentimes, parseAmountToCentimes } from "@dzpos/shared";
import type {
  CustomerDto,
  CustomerLedgerDto,
  CustomerWriteDto,
  DebtKindDto,
  NewCustomerDto,
  PartyKindDto,
} from "@dzpos/shared";
import { api, customerLedgerQueryKey, customersQueryKey } from "@/api";
import { isKey, useTranslation, type Key } from "@/i18n";

export const Route = createFileRoute("/customers")({ component: CustomersScreen });

const PARTY_KINDS: readonly PartyKindDto[] = ["company", "consumer"];

const PARTY_KEY: Record<PartyKindDto, Key> = {
  company: "party_company",
  consumer: "party_consumer",
};

const DEBT_KIND_KEY: Record<DebtKindDto, Key> = {
  opening: "debt_opening",
  sale: "debt_sale",
  payment: "debt_payment",
  avoir: "debt_avoir",
  adjustment: "debt_adjustment",
};

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  not_found: "error_not_found",
  money: "error_money",
  storage: "error_storage",
  restart_needed: "error_restart_needed",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unauthorized: "error_unauthorized",
  unreachable: "error_unreachable",
};

/** The server sends a code, never a sentence; the UI owns the wording. */
function errorKey(error: unknown): Key {
  if (error instanceof ApiError) {
    return ERROR_KEY[error.code] ?? "error_unknown";
  }
  return "error_unknown";
}

/**
 * The tag the list shows for one customer, in the order the shop reads them:
 * a balance past the limit is the fact that stops a sale, and it outranks the
 * others even when the limit is zero. Then no credit at all (a zero limit,
 * which is not the same answer as no limit), then the warning threshold.
 *
 * features.md §2 names the thresholds; which one wins when two apply is a
 * decision this screen takes.
 */
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
              <span dir="ltr">{formatCentimes(c.balance_centimes)}</span>
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

  const save = useMutation({
    mutationFn: (input: NewCustomerDto) =>
      initial === null
        ? api.createCustomer(input)
        : api.updateCustomer(initial.id, whole(input)),
    onSuccess: async () => {
      setServerError(null);
      await queryClient.invalidateQueries({ queryKey: customersQueryKey });
      onDone();
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
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
          // differently (T3).
          credit_limit_centimes: amount(value.creditLimit),
          warn_threshold_centimes: amount(value.warnThreshold),
          notes: cleared(value.notes),
          active: value.active,
          opening_debt_centimes: initial === null ? amount(value.openingDebt) : null,
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
        <span>{t("customers_balance")} </span>
        <span className="font-mono" dir="ltr">
          {formatCentimes(customer.balance_centimes)}
        </span>
      </p>

      {ledger.isPending ? <p>{t("customers_loading")}</p> : null}
      {ledger.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(ledger.error))}
        </p>
      ) : null}
      {ledger.isSuccess ? <LedgerTable ledger={ledger.data} /> : null}

      <AdjustForm customer={customer} />
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
      // customers query, which the movement has just changed.
      await queryClient.invalidateQueries({ queryKey: customersQueryKey });
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

/** An amount in dinars, typed by a person. */
function AmountField({
  label,
  hint,
  value,
  onChange,
  errors,
}: {
  label: string;
  hint?: string;
  value: string;
  onChange: (value: string) => void;
  errors: unknown[];
}) {
  // The hint sits outside the label on purpose: inside it, it would be read
  // as part of the field's name, and a test or a screen reader asking for
  // "Créance de départ (DA)" would not find the box.
  return (
    <div className="flex flex-col gap-1">
      <label className="flex flex-col gap-1">
        <span>{label}</span>
        <input
          dir="ltr"
          inputMode="decimal"
          className="rounded border px-2 py-1 font-mono text-end"
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
      </label>
      {hint === undefined ? null : <span className="text-sm opacity-70">{hint}</span>}
      <FieldError messages={errors} />
    </div>
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

/** Blank is "nothing", not an empty string: the column is cleared. */
function cleared(text: string): string | null {
  const trimmed = text.trim();
  return trimmed === "" ? null : trimmed;
}

/** Blank is "no amount at all"; anything unreadable was refused by the
 *  validator before this runs. */
function amount(text: string): number | null {
  if (text.trim() === "") return null;
  return parseAmountToCentimes(text);
}

/** Whether an optional amount can be read: blank counts, since blank means
 *  the field was left empty. */
function readable(text: string): boolean {
  return text.trim() === "" || parseAmountToCentimes(text) !== null;
}

/** The update takes the fiche without the opening debt: the create type
 *  carries it and the ledger is never edited. */
function whole(input: NewCustomerDto): CustomerWriteDto {
  const { opening_debt_centimes: _opening, ...fiche } = input;
  return fiche;
}

function toPartyKind(value: string): PartyKindDto {
  const found = PARTY_KINDS.find((k) => k === value);
  return found ?? "company";
}
