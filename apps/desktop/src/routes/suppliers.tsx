// The suppliers screen: who the shop buys from, what it owes each of them,
// and the movements behind that figure. Everything it shows comes from the
// API over HTTP and the balance is the core's, never added up here.
//
// Nothing on this screen deletes a supplier: the ledger and the orders hold
// the fiche, so a shop that has stopped buying from somebody closes it, which
// is the close block below and not a checkbox on the form. Closing over an
// account that is still open asks for a reason, which the server records
// beside the balance.

import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { useState } from "react";
import { ApiError, formatCentimes, parseAmountToCentimes } from "@dzpos/shared";
import type {
  NewSupplierDto,
  PaymentMethodDto,
  SupplierDebtKindDto,
  SupplierDto,
  SupplierEntryDto,
  SupplierLedgerDto,
  SupplierWriteDto,
} from "@dzpos/shared";
import {
  api,
  supplierLedgerQueryKey,
  supplierQueryKey,
  suppliersQueryKey,
} from "@/api";
import { useTranslation, type Key } from "@/i18n";
import {
  AmountField,
  FieldError,
  amount,
  cleared,
  errorKey,
  readable,
  shownPositive,
} from "@/lib/fields";

export const Route = createFileRoute("/suppliers")({ component: SuppliersScreen });

const PAYMENT_METHODS: readonly PaymentMethodDto[] = ["cash", "card"];

const PAYMENT_METHOD_KEY: Record<PaymentMethodDto, Key> = {
  cash: "payment_cash",
  card: "payment_card",
};

const SUPPLIER_KIND_KEY: Record<SupplierDebtKindDto, Key> = {
  opening: "debt_opening",
  purchase: "supplier_debt_purchase",
  payment: "debt_payment",
  return: "supplier_debt_return",
  adjustment: "debt_adjustment",
};

/**
 * What the fiche and the list call the stored balance. A supplier's balance
 * is one signed number: positive is what the shop owes, and a return or an
 * advance past what was due drives it below zero, at which point the supplier
 * owes the shop. That is an advance and not a credit: the shop is not holding
 * anybody's money, it has handed money over.
 */
export function supplierBalanceLabel(balance_centimes: number): Key {
  return balance_centimes < 0 ? "suppliers_advance" : "suppliers_balance";
}

export function SuppliersScreen() {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  // One panel, two jobs: "new" is the blank fiche, a supplier is that
  // supplier's own, with the ledger under it.
  const [open, setOpen] = useState<"new" | SupplierDto | null>(null);
  const suppliers = useQuery({
    queryKey: [...suppliersQueryKey, search.trim()],
    queryFn: () => api.listSuppliers(search),
  });

  // The panel reads the row from the list, so it shows the balance the last
  // answer carried rather than the one it was opened with.
  const opened =
    open === null || open === "new"
      ? open
      : (suppliers.data?.find((s) => s.id === open.id) ?? open);

  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("suppliers_title")}</h1>
        <button
          type="button"
          className="rounded border px-3 py-1.5"
          onClick={() => setOpen((current) => (current === null ? "new" : null))}
        >
          {open !== null ? t("action_cancel") : t("suppliers_add")}
        </button>
      </header>

      <label className="flex flex-col gap-1">
        <span>{t("suppliers_search")}</span>
        <input
          type="search"
          className="rounded border px-2 py-1"
          placeholder={t("suppliers_search_hint")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </label>

      {opened === "new" ? (
        <SupplierForm key="new" initial={null} onDone={() => setOpen(null)} />
      ) : null}
      {opened !== null && opened !== "new" ? (
        <div className="flex flex-col gap-4 rounded border p-4">
          <SupplierForm key={opened.id} initial={opened} onDone={() => setOpen(null)} />
          <SupplierLedgerPanel supplier={opened} />
        </div>
      ) : null}

      {suppliers.isPending ? <p>{t("suppliers_loading")}</p> : null}
      {suppliers.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(suppliers.error))}
        </p>
      ) : null}
      {suppliers.isSuccess ? (
        <SupplierTable rows={suppliers.data} onEdit={(row) => setOpen(row)} />
      ) : null}
    </section>
  );
}

/**
 * One fiche on a page of its own, which is what `/suppliers/$id` opens. A
 * purchase screen (T3) has a supplier id and no fiche, and a link that
 * dropped the operator on the list with a search box to retype would be the
 * shop doing the app's work.
 *
 * The same two components the panel uses, so a fiche reads the same whichever
 * way it was opened.
 */
export function SupplierFiche({ id }: { id: number }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const supplier = useQuery({
    queryKey: supplierQueryKey(id),
    queryFn: () => api.getSupplier(id),
  });
  const back = () => void navigate({ to: "/suppliers" });

  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("suppliers_title")}</h1>
        <Link to="/suppliers" className="underline">
          {t("action_back_to_suppliers")}
        </Link>
      </header>

      {supplier.isPending ? <p>{t("suppliers_loading")}</p> : null}
      {supplier.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(supplier.error))}
        </p>
      ) : null}
      {supplier.isSuccess ? (
        <div className="flex flex-col gap-4 rounded border p-4">
          <SupplierForm key={supplier.data.id} initial={supplier.data} onDone={back} />
          <SupplierLedgerPanel supplier={supplier.data} />
        </div>
      ) : null}
    </section>
  );
}

function SupplierTable({
  rows,
  onEdit,
}: {
  rows: SupplierDto[];
  onEdit: (row: SupplierDto) => void;
}) {
  const { t } = useTranslation();
  if (rows.length === 0) return <p>{t("suppliers_empty")}</p>;
  return (
    <table className="w-full text-start">
      <caption className="sr-only">{t("suppliers_title")}</caption>
      <thead>
        <tr>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_name")}</th>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_phone")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_owed")}</th>
          <th scope="col" className="pb-2">
            <span className="sr-only">{t("suppliers_edit")}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {rows.map((s) => (
          <tr key={s.id} className={s.active ? "border-t" : "border-t opacity-60"}>
            <td className="py-1.5 pe-3">
              {s.name}
              {s.active ? null : (
                <span className="ms-2 rounded border px-1 text-xs uppercase">
                  {t("suppliers_inactive")}
                </span>
              )}
            </td>
            {/* dir="ltr" on the number itself, not on the cell: a phone and
                an amount are read left to right with Western digits whatever
                the screen's language, and without it the bidi algorithm is
                free to reorder the sign and the groups inside an RTL row. */}
            <td className="py-1.5 pe-3 font-mono">
              <span dir="ltr">{s.phone ?? ""}</span>
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">{shownPositive(s.balance_centimes)}</span>
              {s.balance_centimes < 0 ? (
                <span className="ms-1 rounded border px-1 font-sans text-sm">
                  {t("suppliers_advance")}
                </span>
              ) : null}
            </td>
            <td className="py-1.5 ps-3 text-end">
              <button
                type="button"
                className="rounded border px-2 py-0.5 text-sm"
                aria-label={`${t("suppliers_edit")} ${s.name}`}
                onClick={() => onEdit(s)}
              >
                {t("suppliers_edit")}
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
 * an edit that could set it would be a correction nobody could see.
 *
 * There is no active checkbox. Closing a fiche is a decision the server asks
 * a reason for, so it is the block under the form and not a box beside a
 * phone number; reopening one is the button that stands in its place.
 */
function SupplierForm({
  initial,
  onDone,
}: {
  initial: SupplierDto | null;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const save = useMutation({
    mutationFn: (fiche: NewSupplierDto) =>
      // A blank fiche closes nothing, and `POST /suppliers` refuses a field
      // it does not know, so the create and the update send two shapes.
      initial === null ? api.createSupplier(fiche) : api.updateSupplier(initial.id, whole(fiche)),
    onSuccess: async () => {
      setServerError(null);
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
      onDone();
    },
    onError: (error: unknown) => {
      // On the code and not on the field: a name too long is a `validation`
      // on `name` as well, and it is the one thing this message would be
      // wrong about.
      setServerError(
        error instanceof ApiError && error.code === "conflict"
          ? "error_supplier_name_taken"
          : errorKey(error),
      );
    },
  });

  const form = useForm({
    defaultValues:
      initial === null
        ? {
            name: "",
            phone: "",
            address: "",
            rc: "",
            nif: "",
            nis: "",
            ai: "",
            openingDebt: "",
            notes: "",
          }
        : {
            name: initial.name,
            phone: initial.phone ?? "",
            address: initial.address ?? "",
            rc: initial.rc ?? "",
            nif: initial.nif ?? "",
            nis: initial.nis ?? "",
            ai: initial.ai ?? "",
            openingDebt: "",
            notes: initial.notes ?? "",
          },
    onSubmit: async ({ value }) => {
      // The rejection is swallowed on purpose: onError has already turned the
      // server's code into a translated message on the form.
      await save
        .mutateAsync({
          name: value.name,
          phone: cleared(value.phone),
          address: cleared(value.address),
          rc: cleared(value.rc),
          nif: cleared(value.nif),
          nis: cleared(value.nis),
          ai: cleared(value.ai),
          notes: cleared(value.notes),
          // A fiche keeps the state it is in: the close block below is what
          // changes it.
          active: initial === null ? true : initial.active,
          opening_debt_centimes: initial === null ? amount(value.openingDebt) : null,
        })
        .catch(() => undefined);
    },
  });

  /** One optional text field of the fiche: a phone, an identifier, a note. */
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
    <div className="flex flex-col gap-3">
      <form
        noValidate
        className="flex flex-col gap-3 rounded border p-4"
        onSubmit={(e) => {
          e.preventDefault();
          void form.handleSubmit();
        }}
      >
        <h2 className="font-semibold">{initial === null ? t("suppliers_new") : initial.name}</h2>

        <form.Field
          name="name"
          validators={{
            onSubmit: ({ value }) => {
              const name = value.trim();
              if (name === "") return "error_name_required";
              // The bound the core stores under (`MAX_FIELD_CHARS`), held
              // here so a paste gone wrong is said in the field's own words
              // rather than making the round trip.
              return name.length > 200 ? "error_name_too_long" : undefined;
            },
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

        {optionalText("phone", t("field_phone"), true)}
        {optionalText("address", t("field_address"))}
        {optionalText("rc", t("field_rc"), true)}
        {optionalText("nif", t("field_nif"), true)}
        {optionalText("nis", t("field_nis"), true)}
        {optionalText("ai", t("field_ai"), true)}

        {initial === null ? (
          <form.Field
            name="openingDebt"
            validators={{
              onSubmit: ({ value }) => (readable(value) ? undefined : "error_opening_debt_invalid"),
            }}
          >
            {(field) => (
              <AmountField
                label={t("field_supplier_opening_debt")}
                hint={t("field_supplier_opening_debt_hint")}
                value={field.state.value}
                onChange={field.handleChange}
                errors={field.state.meta.errors}
              />
            )}
          </form.Field>
        ) : null}

        {optionalText("notes", t("field_notes"))}

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

      {initial === null ? null : <CloseBlock supplier={initial} />}
    </div>
  );
}

/**
 * Stopping, or resuming, the buying. The reason is asked for whenever the
 * screen can see the account is open, and the server asks for it again when
 * only it can see that an order is still unpaid at a nil balance; the refusal
 * names the `reason` field, so the box stays and the message goes with it.
 */
function CloseBlock({ supplier }: { supplier: SupplierDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [reason, setReason] = useState("");
  const [serverError, setServerError] = useState<Key | null>(null);
  const [reasonRefused, setReasonRefused] = useState(false);

  const change = useMutation({
    mutationFn: (input: { close: boolean }) =>
      input.close
        ? api.closeSupplier(supplier.id, { reason: cleared(reason) })
        : api.updateSupplier(supplier.id, {
            name: supplier.name,
            phone: supplier.phone,
            address: supplier.address,
            rc: supplier.rc,
            nif: supplier.nif,
            nis: supplier.nis,
            ai: supplier.ai,
            notes: supplier.notes,
            active: true,
            close_reason: null,
          }),
    onSuccess: async () => {
      setServerError(null);
      setReasonRefused(false);
      setReason("");
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
      await queryClient.invalidateQueries({ queryKey: supplierQueryKey(supplier.id) });
    },
    onError: (error: unknown) => {
      setReasonRefused(error instanceof ApiError && error.field === "reason");
      setServerError(errorKey(error));
    },
  });

  if (!supplier.active) {
    return (
      <section className="flex flex-col gap-2 rounded border p-3">
        <h3 className="font-semibold">{t("suppliers_closed")}</h3>
        <p className="text-sm opacity-70">{t("suppliers_closed_still_pays")}</p>
        {serverError !== null ? (
          <p role="alert" className="text-red-700">
            {t(serverError)}
          </p>
        ) : null}
        <div>
          <button
            type="button"
            className="rounded border px-3 py-1.5"
            disabled={change.isPending}
            onClick={() => void change.mutateAsync({ close: false }).catch(() => undefined)}
          >
            {t("action_reopen_supplier")}
          </button>
        </div>
      </section>
    );
  }

  const asksForReason = supplier.balance_centimes !== 0 || reasonRefused;

  return (
    <section className="flex flex-col gap-2 rounded border border-amber-600 p-3">
      <h3 className="font-semibold">{t("suppliers_close")}</h3>
      <p className="text-sm opacity-70">{t("suppliers_close_hint")}</p>
      {asksForReason ? (
        <label className="flex flex-col gap-1">
          <span>{t("suppliers_close_reason")}</span>
          <span className="text-sm">
            {t(supplierBalanceLabel(supplier.balance_centimes))}{" "}
            <span className="font-mono" dir="ltr">
              {shownPositive(supplier.balance_centimes)}
            </span>
          </span>
          <input
            data-testid="supplier-close-reason"
            className="rounded border px-2 py-1"
            value={reason}
            onChange={(e) => setReason(e.target.value)}
          />
        </label>
      ) : null}
      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}
      <div>
        <button
          type="button"
          className="rounded border px-3 py-1.5"
          disabled={change.isPending}
          onClick={() => void change.mutateAsync({ close: true }).catch(() => undefined)}
        >
          {t("action_close_supplier")}
        </button>
      </div>
    </section>
  );
}

/** The movements, and the two forms that write one. */
function SupplierLedgerPanel({ supplier }: { supplier: SupplierDto }) {
  const { t } = useTranslation();
  const ledger = useQuery({
    queryKey: supplierLedgerQueryKey(supplier.id),
    queryFn: () => api.supplierLedger(supplier.id),
  });

  return (
    <section className="flex flex-col gap-3">
      <h2 className="font-semibold">{t("suppliers_ledger")}</h2>
      <p>
        <span>{t(supplierBalanceLabel(supplier.balance_centimes))} </span>
        <span className="font-mono" dir="ltr">
          {shownPositive(supplier.balance_centimes)}
        </span>
      </p>

      {ledger.isPending ? <p>{t("suppliers_loading")}</p> : null}
      {ledger.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(ledger.error))}
        </p>
      ) : null}
      {ledger.isSuccess ? <LedgerTable ledger={ledger.data} /> : null}

      <PaymentForm supplier={supplier} />
      <AdjustForm supplier={supplier} />
    </section>
  );
}

/**
 * Money to the supplier. The server settles the oldest orders first and
 * refuses a payment above what the shop owes, so nothing here caps the figure
 * or picks the orders: a screen that decided either would be a second answer
 * to what the shop owes.
 */
function PaymentForm({ supplier }: { supplier: SupplierDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);
  /** What the shop actually owes, as the refused payment reported it. "Too
   *  much" is useless without the amount that would not have been. */
  const [outstanding, setOutstanding] = useState<number | null>(null);
  const [saved, setSaved] = useState(false);

  const pay = useMutation({
    mutationFn: (input: {
      amount_centimes: number;
      payment_mode: PaymentMethodDto;
      note: string | null;
    }) => api.paySupplier(supplier.id, input),
    onSuccess: async (answer: SupplierLedgerDto) => {
      setServerError(null);
      setOutstanding(null);
      setSaved(true);
      queryClient.setQueryData(supplierLedgerQueryKey(supplier.id), answer);
      // The balance on the list above and on the fiche came from the
      // suppliers query, which the movement has just changed.
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
    },
    onError: (error: unknown) => {
      setSaved(false);
      const above = error instanceof ApiError ? error.outstandingCentimes : undefined;
      setOutstanding(above ?? null);
      setServerError(above === undefined ? errorKey(error) : "error_supplier_payment_above_debt");
    },
  });

  const form = useForm({
    defaultValues: { amount: "", mode: "cash", note: "" },
    onSubmit: async ({ value }) => {
      const centimes = parseAmountToCentimes(value.amount);
      if (centimes === null || centimes <= 0) return;
      if (!window.confirm(t("suppliers_pay_confirm"))) return;
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
      <h3 className="font-semibold">{t("suppliers_pay")}</h3>
      <p className="text-sm opacity-70">{t("suppliers_pay_hint")}</p>

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
            <legend>{t("field_payment_mode")}</legend>
            <div className="flex gap-4">
              {PAYMENT_METHODS.map((mode) => (
                <label key={mode} className="flex items-center gap-2">
                  <input
                    type="radio"
                    name="supplier_payment_mode"
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
      {saved && serverError === null ? <p role="status">{t("suppliers_paid")}</p> : null}

      <div>
        <button type="submit" className="rounded border px-3 py-1.5" disabled={pay.isPending}>
          {pay.isPending ? t("action_saving") : t("action_pay_supplier")}
        </button>
      </div>
    </form>
  );
}

function LedgerTable({ ledger }: { ledger: SupplierLedgerDto }) {
  const { t } = useTranslation();
  if (ledger.entries.length === 0) return <p>{t("suppliers_ledger_empty")}</p>;
  return (
    <table className="w-full text-start">
      <caption className="sr-only">{t("suppliers_ledger")}</caption>
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
            <td className="py-1.5 pe-3">
              {t(SUPPLIER_KIND_KEY[entry.kind])}
              <Settled entry={entry} />
            </td>
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
            {/* The running balance is the core's (services::supplier_debt): a
                column added up here would be a second answer to what the shop
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

/** Which orders a payment went to, under the kind it went as. Shown open
 *  rather than behind a toggle: which order the money settled is the question
 *  asked of a payment, and the server decided it. */
function Settled({ entry }: { entry: SupplierEntryDto }) {
  const { t } = useTranslation();
  if (entry.allocations.length === 0) return null;
  return (
    <ul className="text-sm opacity-70" data-testid="supplier-allocations">
      {entry.allocations.map((allocation) => (
        <li key={allocation.purchase_id}>
          {t("col_purchase")} <span className="font-mono">{allocation.purchase_id}</span>{" "}
          <span className="font-mono" dir="ltr">
            {formatCentimes(allocation.amount_centimes)}
          </span>
        </li>
      ))}
    </ul>
  );
}

/** A correction, written as a movement. It lands in the account the shop
 *  reads back to the supplier and nothing removes it afterwards. */
function AdjustForm({ supplier }: { supplier: SupplierDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);
  const [saved, setSaved] = useState(false);

  const adjust = useMutation({
    mutationFn: (input: { amount_centimes: number; note: string | null }) =>
      api.adjustSupplierDebt(supplier.id, input),
    onSuccess: async (answer: SupplierLedgerDto) => {
      setServerError(null);
      setSaved(true);
      queryClient.setQueryData(supplierLedgerQueryKey(supplier.id), answer);
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
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
      if (!window.confirm(t("suppliers_adjust_confirm"))) return;
      const written = await adjust
        .mutateAsync({ amount_centimes: centimes, note: cleared(value.note) })
        .then(() => true)
        .catch(() => false);
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
      <h3 className="font-semibold">{t("suppliers_adjust")}</h3>
      <p className="text-sm opacity-70">{t("suppliers_adjust_hint")}</p>

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
      {saved && serverError === null ? <p role="status">{t("suppliers_adjusted")}</p> : null}

      <div>
        <button type="submit" className="rounded border px-3 py-1.5" disabled={adjust.isPending}>
          {adjust.isPending ? t("action_saving") : t("action_adjust")}
        </button>
      </div>
    </form>
  );
}

/** The update body: the whole fiche without the opening debt, which is a
 *  create-only field, and with no close reason, which the close route
 *  carries. */
function whole(input: NewSupplierDto): SupplierWriteDto {
  const { opening_debt_centimes: _opening, ...fiche } = input;
  return { ...fiche, close_reason: null };
}

/** Cash unless the form says otherwise: the radio group has no third option,
 *  and a payment mode is never guessed from an unknown string. */
function toPaymentMethod(value: string): PaymentMethodDto {
  const found = PAYMENT_METHODS.find((m) => m === value);
  return found ?? "cash";
}
