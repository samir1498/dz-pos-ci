// The suppliers screen: who the shop buys from, what it owes each of them,
// and the movements behind that figure. Everything it shows comes from the
// API over HTTP and the balance is the core's, never added up here.
//
// Two pages, and the same fiche on both. `/suppliers` is the list: a search,
// a table of names and balances, and a panel that slides in with one
// supplier's fiche on it. `/suppliers/{id}` is the statement: the balance,
// the movements behind it, and the two dialogs that write one. A row's name
// is the way from the first to the second, and the panel opens from either,
// so the fiche cannot read one way on the list and another on the statement.
//
// Nothing here deletes a supplier: the ledger and the orders hold the fiche,
// so a shop that has stopped buying from somebody closes it, which is the
// close block at the foot of the panel and not a checkbox on the form.
// Closing over an account that is still open asks for a reason, which the
// server records beside the balance.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { ArrowLeft, FilePlus2, Scale, SquarePen, Truck, Wallet } from "lucide-react";
import { useState } from "react";
import { ApiError } from "@dzpos/shared";
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
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
import { PageHeader } from "@/components/PageHeader";
import { StatusPill } from "@/components/StatusPill";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useTranslation, type Key } from "@/i18n";
import { cleared, errorKey } from "@/lib/fields";

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

/** An amount is drawn positive whatever its sign; the word beside it carries
 *  the direction, because "Dette -1 000,00" is not a sentence anyone says at
 *  a counter. */
function positive(centimes: number): number {
  return Math.abs(centimes);
}

/** What a balance is, in the kit's words. Nil is settled; anything else is
 *  an account still running, in either direction. */
function balanceStatus(centimes: number): "paid" | "open" {
  return centimes === 0 ? "paid" : "open";
}

export function SuppliersScreen() {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  // One panel, two jobs: "new" is the blank fiche, a supplier is that
  // supplier's own.
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

  const columns: readonly Column<SupplierDto>[] = [
    {
      id: "name",
      header: t("col_name"),
      cell: (s) => (
        <Link
          to="/suppliers/$id"
          params={{ id: String(s.id) }}
          className="font-medium underline-offset-4 hover:underline"
        >
          {s.name}
        </Link>
      ),
    },
    {
      id: "phone",
      header: t("col_phone"),
      // dir="ltr" on the number itself, not on the cell: a phone is read left
      // to right with Western digits whatever the screen's language, and
      // without it the bidi algorithm is free to reorder the groups.
      cell: (s) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {s.phone ?? ""}
        </span>
      ),
    },
    {
      id: "owed",
      header: t("col_owed"),
      money: true,
      cell: (s) => <Money centimes={positive(s.balance_centimes)} />,
    },
    {
      id: "state",
      header: t("col_status"),
      cell: (s) => (
        <div className="flex flex-wrap items-center gap-1.5">
          <StatusPill status={balanceStatus(s.balance_centimes)} />
          {s.balance_centimes < 0 ? (
            <Badge variant="secondary">{t("suppliers_advance")}</Badge>
          ) : null}
          {s.active ? null : <Badge variant="outline">{t("suppliers_inactive")}</Badge>}
        </div>
      ),
    },
  ];

  const addButton = (
    <Button onClick={() => setOpen("new")}>
      <Icon as={FilePlus2} size={18} />
      {t("suppliers_add")}
    </Button>
  );

  return (
    <div className="flex flex-col gap-4">
      <PageHeader title={t("suppliers_title")} actions={addButton} />

      <FormField label={t("suppliers_search")} className="max-w-sm">
        {(parts) => (
          <Input
            {...parts}
            type="search"
            data-testid="suppliers-search"
            placeholder={t("suppliers_search_hint")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        )}
      </FormField>

      {suppliers.isPending ? (
        <p className="text-sm text-muted-foreground">{t("suppliers_loading")}</p>
      ) : null}
      {suppliers.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(suppliers.error))}
        </p>
      ) : null}
      {suppliers.isSuccess ? (
        <DataTable
          data-testid="suppliers-table"
          caption={t("suppliers_title")}
          columns={columns}
          rows={suppliers.data}
          rowKey={(s) => s.id}
          empty={
            <EmptyState
              icon={Truck}
              title={t("suppliers_empty")}
              description={t("suppliers_empty_hint")}
              action={addButton}
            />
          }
          actions={(s) => (
            <Button
              variant="ghost"
              size="icon"
              aria-label={`${t("suppliers_edit")} ${s.name}`}
              onClick={() => setOpen(s)}
            >
              <Icon as={SquarePen} size={18} />
            </Button>
          )}
        />
      ) : null}

      <SupplierSheet
        supplier={opened === "new" ? null : opened}
        open={opened !== null}
        onOpenChange={(next) => {
          if (!next) setOpen(null);
        }}
      />
    </div>
  );
}

/**
 * One supplier's statement, which is what `/suppliers/{id}` opens. A purchase
 * screen has a supplier id and no fiche, and a link that dropped the operator
 * on the list with a search box to retype would be the shop doing the app's
 * work.
 *
 * The balance, the movements behind it, and the two dialogs that write one.
 * The fiche itself is the same panel the list opens, so the two ways in
 * cannot drift apart.
 */
export function SupplierFiche({ id }: { id: number }) {
  const { t } = useTranslation();
  const [sheetOpen, setSheetOpen] = useState(false);
  const supplier = useQuery({
    queryKey: supplierQueryKey(id),
    queryFn: () => api.getSupplier(id),
  });

  return (
    <div className="flex flex-col gap-4">
      {supplier.isPending ? (
        <p className="text-sm text-muted-foreground">{t("suppliers_loading")}</p>
      ) : null}
      {supplier.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(supplier.error))}
        </p>
      ) : null}
      {supplier.isSuccess ? (
        <>
          <PageHeader
            title={supplier.data.name}
            actions={
              <>
                <Button variant="ghost" asChild>
                  <Link to="/suppliers">
                    <Icon as={ArrowLeft} size={18} flip />
                    {t("action_back_to_suppliers")}
                  </Link>
                </Button>
                <Button variant="outline" onClick={() => setSheetOpen(true)}>
                  <Icon as={SquarePen} size={18} />
                  {t("suppliers_edit")}
                </Button>
              </>
            }
          />
          <SupplierStatement supplier={supplier.data} />
          <SupplierSheet
            supplier={supplier.data}
            open={sheetOpen}
            onOpenChange={setSheetOpen}
          />
        </>
      ) : null}
    </div>
  );
}

/** The fiche in a panel: the form, and under a rule the block that stops or
 *  resumes the buying. It slides in from the side the page is not read from,
 *  which `AppShell` computes the same way for the sidebar. */
function SupplierSheet({
  supplier,
  open,
  onOpenChange,
}: {
  /** `null` is a blank fiche. */
  supplier: SupplierDto | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, dir } = useTranslation();
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      {/* The side is physical on purpose: the panel's edge, its border and
          the half it slides in from have to agree, so the caller picks it
          from the page direction. */}
      <SheetContent
        side={dir === "rtl" ? "left" : "right"}
        data-testid="supplier-sheet"
        className="w-full gap-0 overflow-y-auto sm:max-w-lg"
      >
        <SheetHeader>
          <SheetTitle>{supplier === null ? t("suppliers_new") : supplier.name}</SheetTitle>
          <SheetDescription>{t("suppliers_fiche_hint")}</SheetDescription>
        </SheetHeader>
        <div className="flex flex-col gap-4 px-4 pb-6">
          <SupplierForm
            key={supplier === null ? "new" : supplier.id}
            initial={supplier}
            onDone={() => onOpenChange(false)}
          />
          {supplier === null ? null : (
            <>
              <Separator />
              <CloseBlock supplier={supplier} onDone={() => onOpenChange(false)} />
            </>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

/** What the fiche form holds while it is being filled. The opening debt is
 *  centimes or nothing at all; no float is made on the way through. */
interface FicheValues {
  name: string;
  phone: string;
  address: string;
  rc: string;
  nif: string;
  nis: string;
  ai: string;
  openingDebt: number | null;
  notes: string;
}

/**
 * The blank fiche and an existing one are the same form: the same fields and
 * the same request shape either way. The opening debt is the one difference,
 * and it is only on the blank one: it is a ledger movement, not a column, so
 * an edit that could set it would be a correction nobody could see.
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
      if (initial !== null) {
        await queryClient.invalidateQueries({ queryKey: supplierQueryKey(initial.id) });
      }
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

  const blank: FicheValues = {
    name: "",
    phone: "",
    address: "",
    rc: "",
    nif: "",
    nis: "",
    ai: "",
    openingDebt: null,
    notes: "",
  };

  const form = useForm({
    defaultValues:
      initial === null
        ? blank
        : {
            ...blank,
            name: initial.name,
            phone: initial.phone ?? "",
            address: initial.address ?? "",
            rc: initial.rc ?? "",
            nif: initial.nif ?? "",
            nis: initial.nis ?? "",
            ai: initial.ai ?? "",
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
          opening_debt_centimes: initial === null ? value.openingDebt : null,
        })
        .catch(() => undefined);
    },
  });

  /** One optional text field of the fiche: a phone, an identifier, a note. */
  const optionalText = (
    name: "phone" | "address" | "rc" | "nif" | "nis" | "ai" | "notes",
    label: string,
    ltr = false,
  ) => (
    <form.Field name={name}>
      {(field) => (
        <FormField label={label}>
          {(parts) => (
            <Input
              {...parts}
              dir={ltr ? "ltr" : undefined}
              className={ltr ? "font-numeric tabular-nums" : undefined}
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
          )}
        </FormField>
      )}
    </form.Field>
  );

  return (
    <form
      noValidate
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        void form.handleSubmit();
      }}
    >
      <form.Field
        name="name"
        validators={{
          onSubmit: ({ value }) => {
            const name = value.trim();
            if (name === "") return "error_name_required";
            // The bound the core stores under (`MAX_FIELD_CHARS`), held here
            // so a paste gone wrong is said in the field's own words rather
            // than making the round trip.
            return name.length > 200 ? "error_name_too_long" : undefined;
          },
        }}
      >
        {(field) => (
          <FormField label={t("field_name")} error={messageOf(field.state.meta.errors, t)}>
            {(parts) => (
              <Input
                {...parts}
                value={field.state.value}
                onChange={(e) => field.handleChange(e.target.value)}
                onBlur={field.handleBlur}
              />
            )}
          </FormField>
        )}
      </form.Field>

      {optionalText("phone", t("field_phone"), true)}
      {optionalText("address", t("field_address"))}
      {optionalText("rc", t("field_rc"), true)}
      {optionalText("nif", t("field_nif"), true)}
      {optionalText("nis", t("field_nis"), true)}
      {optionalText("ai", t("field_ai"), true)}

      {initial === null ? (
        <form.Field name="openingDebt">
          {(field) => (
            <FormField
              label={t("field_supplier_opening_debt")}
              hint={t("field_supplier_opening_debt_hint")}
            >
              {(parts) => (
                <MoneyInput
                  {...parts}
                  value={field.state.value}
                  onChange={field.handleChange}
                />
              )}
            </FormField>
          )}
        </form.Field>
      ) : null}

      {optionalText("notes", t("field_notes"))}

      {serverError !== null ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(serverError)}
        </p>
      ) : null}

      <div className="flex flex-wrap gap-2">
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

/**
 * Stopping, or resuming, the buying. The reason is asked for whenever the
 * screen can see the account is open, and the server asks for it again when
 * only it can see that an order is still unpaid at a nil balance; the refusal
 * names the `reason` field, so the box stays and the message goes with it.
 */
function CloseBlock({ supplier, onDone }: { supplier: SupplierDto; onDone: () => void }) {
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
      // The panel steps out of the way: what changed is on the list and on
      // the statement behind it, and a panel left open over them hides the
      // answer to the thing that was just done.
      onDone();
    },
    onError: (error: unknown) => {
      setReasonRefused(error instanceof ApiError && error.field === "reason");
      setServerError(errorKey(error));
    },
  });

  if (!supplier.active) {
    return (
      <section className="flex flex-col gap-2">
        <h3 className="text-md font-semibold">{t("suppliers_closed")}</h3>
        <p className="text-sm text-muted-foreground">{t("suppliers_closed_still_pays")}</p>
        {serverError !== null ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(serverError)}
          </p>
        ) : null}
        <div>
          <Button
            type="button"
            variant="outline"
            disabled={change.isPending}
            onClick={() => void change.mutateAsync({ close: false }).catch(() => undefined)}
          >
            {t("action_reopen_supplier")}
          </Button>
        </div>
      </section>
    );
  }

  const asksForReason = supplier.balance_centimes !== 0 || reasonRefused;

  return (
    <section className="flex flex-col gap-2 rounded-lg border border-warn bg-warn-soft p-3">
      <h3 className="text-md font-semibold">{t("suppliers_close")}</h3>
      <p className="text-sm text-muted-foreground">{t("suppliers_close_hint")}</p>
      {asksForReason ? (
        <>
          <p className="flex items-center gap-2 text-sm">
            <span>{t(supplierBalanceLabel(supplier.balance_centimes))}</span>
            <Money centimes={positive(supplier.balance_centimes)} />
          </p>
          <FormField label={t("suppliers_close_reason")}>
            {(parts) => (
              <Input
                {...parts}
                data-testid="supplier-close-reason"
                value={reason}
                onChange={(e) => setReason(e.target.value)}
              />
            )}
          </FormField>
        </>
      ) : null}
      {serverError !== null ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(serverError)}
        </p>
      ) : null}
      <div>
        <Button
          type="button"
          variant="destructive"
          disabled={change.isPending}
          onClick={() => void change.mutateAsync({ close: true }).catch(() => undefined)}
        >
          {t("action_close_supplier")}
        </Button>
      </div>
    </section>
  );
}

/** The balance, the movements behind it, and the two dialogs that write one. */
function SupplierStatement({ supplier }: { supplier: SupplierDto }) {
  const { t } = useTranslation();
  const ledger = useQuery({
    queryKey: supplierLedgerQueryKey(supplier.id),
    queryFn: () => api.supplierLedger(supplier.id),
  });

  return (
    <section className="flex flex-col gap-4">
      <Card data-testid="supplier-balance">
        <CardHeader>
          <CardDescription>{t(supplierBalanceLabel(supplier.balance_centimes))}</CardDescription>
          <CardTitle>
            <Money centimes={positive(supplier.balance_centimes)} className="text-2xl" />
          </CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap items-center gap-2">
          <StatusPill status={balanceStatus(supplier.balance_centimes)} />
          {supplier.active ? null : (
            <Badge variant="outline">{t("suppliers_inactive")}</Badge>
          )}
          <div className="ms-auto flex flex-wrap gap-2">
            <PayDialog supplier={supplier} />
            <AdjustDialog supplier={supplier} />
          </div>
        </CardContent>
      </Card>

      <h3 className="text-md font-semibold">{t("suppliers_ledger")}</h3>
      {ledger.isPending ? (
        <p className="text-sm text-muted-foreground">{t("suppliers_loading")}</p>
      ) : null}
      {ledger.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(ledger.error))}
        </p>
      ) : null}
      {ledger.isSuccess ? <LedgerTable ledger={ledger.data} /> : null}
    </section>
  );
}

function LedgerTable({ ledger }: { ledger: SupplierLedgerDto }) {
  const { t } = useTranslation();
  const columns: readonly Column<SupplierEntryDto>[] = [
    {
      id: "date",
      header: t("col_date"),
      cell: (entry) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {entry.created_at}
        </span>
      ),
    },
    {
      id: "kind",
      header: t("col_kind"),
      cell: (entry) => (
        <div className="flex flex-col gap-1">
          <span>{t(SUPPLIER_KIND_KEY[entry.kind])}</span>
          <Settled entry={entry} />
        </div>
      ),
    },
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
      // The running balance is the core's (services::supplier_debt): a column
      // added up here would be a second answer to what the shop owes.
      id: "balance",
      header: t("col_balance"),
      money: true,
      cell: (entry) => <Money centimes={entry.balance_after_centimes} />,
    },
    { id: "note", header: t("col_note"), cell: (entry) => entry.note ?? "" },
  ];

  return (
    <DataTable
      data-testid="supplier-ledger"
      caption={t("suppliers_ledger")}
      columns={columns}
      rows={ledger.entries}
      rowKey={(entry) => entry.id}
      empty={<EmptyState icon={Scale} title={t("suppliers_ledger_empty")} />}
    />
  );
}

/** Which orders a payment went to, under the kind it went as. Shown open
 *  rather than behind a toggle: which order the money settled is the question
 *  asked of a payment, and the server decided it. */
function Settled({ entry }: { entry: SupplierEntryDto }) {
  const { t } = useTranslation();
  if (entry.allocations.length === 0) return null;
  return (
    <ul className="flex flex-col gap-0.5 text-sm text-muted-foreground" data-testid="supplier-allocations">
      {entry.allocations.map((allocation) => (
        <li key={allocation.purchase_id} className="flex items-center gap-1.5">
          <span>{t("col_purchase")}</span>
          <span dir="ltr" className="font-numeric tabular-nums">
            {allocation.purchase_id}
          </span>
          <Money centimes={allocation.amount_centimes} />
        </li>
      ))}
    </ul>
  );
}

/** The two modes a payment can be in. There is no third, so this is two
 *  buttons rather than a list that has to be opened to be read; the pressed
 *  one is the mode the payment goes out as. */
function ModePicker({
  value,
  onChange,
}: {
  value: PaymentMethodDto;
  onChange: (mode: PaymentMethodDto) => void;
}) {
  const { t } = useTranslation();
  return (
    <fieldset className="flex flex-col gap-1.5 text-start">
      {/* A legend rather than a `Label`: a label points at one control and
          this names a pair of them. The kit's label styling is what it wears
          so it does not read as a different kind of field. */}
      <legend className="text-sm leading-none font-medium">{t("field_payment_mode")}</legend>
      <div className="flex flex-wrap gap-2">
        {PAYMENT_METHODS.map((mode) => (
          <Button
            key={mode}
            type="button"
            variant={value === mode ? "default" : "outline"}
            aria-pressed={value === mode}
            onClick={() => onChange(mode)}
          >
            {t(PAYMENT_METHOD_KEY[mode])}
          </Button>
        ))}
      </div>
    </fieldset>
  );
}

/**
 * Money to the supplier. The server settles the oldest orders first and
 * refuses a payment above what the shop owes, so nothing here caps the figure
 * or picks the orders: a screen that decided either would be a second answer
 * to what the shop owes.
 *
 * The dialog is the confirmation. A browser `confirm()` over a panel that
 * already says what is about to happen is the same question asked twice, in a
 * box the app cannot translate or theme.
 */
function PayDialog({ supplier }: { supplier: SupplierDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [amount, setAmount] = useState<number | null>(null);
  const [mode, setMode] = useState<PaymentMethodDto>("cash");
  const [note, setNote] = useState("");
  const [fieldError, setFieldError] = useState<Key | null>(null);
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
      setAmount(null);
      setNote("");
      setOpen(false);
      queryClient.setQueryData(supplierLedgerQueryKey(supplier.id), answer);
      // The balance on the list and on the card came from the suppliers
      // query, which the movement has just changed.
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
      await queryClient.invalidateQueries({ queryKey: supplierQueryKey(supplier.id) });
    },
    onError: (error: unknown) => {
      setSaved(false);
      const above = error instanceof ApiError ? error.outstandingCentimes : undefined;
      setOutstanding(above ?? null);
      setServerError(above === undefined ? errorKey(error) : "error_supplier_payment_above_debt");
    },
  });

  const submit = () => {
    if (amount === null) {
      setFieldError("error_payment_amount_invalid");
      return;
    }
    if (amount <= 0) {
      setFieldError("error_payment_amount_zero");
      return;
    }
    setFieldError(null);
    // A refused payment keeps what was typed and the dialog stays open: the
    // message says what is outstanding, and an emptied box means typing the
    // figure again to find out what was wrong with it.
    void pay
      .mutateAsync({ amount_centimes: amount, payment_mode: mode, note: cleared(note) })
      .catch(() => undefined);
  };

  return (
    <>
      {saved ? (
        <p role="status" className="text-sm text-fg-success">
          {t("suppliers_paid")}
        </p>
      ) : null}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <Button variant="outline" data-testid="supplier-pay-open">
            <Icon as={Wallet} size={18} />
            {t("suppliers_pay")}
          </Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("suppliers_pay")}</DialogTitle>
            <DialogDescription>{t("suppliers_pay_hint")}</DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-4">
            <FormField
              label={t("field_payment_amount")}
              error={fieldError === null ? undefined : t(fieldError)}
            >
              {(parts) => (
                <MoneyInput
                  {...parts}
                  value={amount}
                  onChange={(next) => {
                    setSaved(false);
                    setAmount(next);
                  }}
                />
              )}
            </FormField>
            <ModePicker value={mode} onChange={setMode} />
            <FormField label={t("field_payment_note")}>
              {(parts) => (
                <Input {...parts} value={note} onChange={(e) => setNote(e.target.value)} />
              )}
            </FormField>
            {serverError !== null ? (
              <p role="alert" className="flex flex-wrap items-center gap-1 text-sm text-fg-danger">
                <span>{t(serverError)}</span>
                {outstanding === null ? null : <Money centimes={outstanding} />}
              </p>
            ) : null}
          </div>
          <DialogFooter>
            <Button type="button" onClick={submit} disabled={pay.isPending}>
              {pay.isPending ? t("action_saving") : t("action_pay_supplier")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

/** A correction, written as a movement. It lands in the account the shop
 *  reads back to the supplier and nothing removes it afterwards. */
function AdjustDialog({ supplier }: { supplier: SupplierDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [amount, setAmount] = useState<number | null>(null);
  const [note, setNote] = useState("");
  const [fieldError, setFieldError] = useState<Key | null>(null);
  const [serverError, setServerError] = useState<Key | null>(null);
  const [saved, setSaved] = useState(false);

  const adjust = useMutation({
    mutationFn: (input: { amount_centimes: number; note: string | null }) =>
      api.adjustSupplierDebt(supplier.id, input),
    onSuccess: async (answer: SupplierLedgerDto) => {
      setServerError(null);
      setSaved(true);
      setAmount(null);
      setNote("");
      setOpen(false);
      queryClient.setQueryData(supplierLedgerQueryKey(supplier.id), answer);
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
      await queryClient.invalidateQueries({ queryKey: supplierQueryKey(supplier.id) });
    },
    onError: (error: unknown) => {
      setSaved(false);
      setServerError(errorKey(error));
    },
  });

  const submit = () => {
    if (amount === null) {
      setFieldError("error_amount_invalid");
      return;
    }
    if (amount === 0) {
      setFieldError("error_amount_zero");
      return;
    }
    setFieldError(null);
    void adjust
      .mutateAsync({ amount_centimes: amount, note: cleared(note) })
      .catch(() => undefined);
  };

  return (
    <>
      {saved ? (
        <p role="status" className="text-sm text-fg-success">
          {t("suppliers_adjusted")}
        </p>
      ) : null}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <Button variant="outline" data-testid="supplier-adjust-open">
            <Icon as={Scale} size={18} />
            {t("suppliers_adjust")}
          </Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("suppliers_adjust")}</DialogTitle>
            <DialogDescription>{t("suppliers_adjust_hint")}</DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-4">
            <FormField
              label={t("field_adjust_amount")}
              error={fieldError === null ? undefined : t(fieldError)}
            >
              {(parts) => (
                <MoneyInput
                  {...parts}
                  value={amount}
                  onChange={(next) => {
                    setSaved(false);
                    setAmount(next);
                  }}
                />
              )}
            </FormField>
            <FormField label={t("field_adjust_note")}>
              {(parts) => (
                <Input {...parts} value={note} onChange={(e) => setNote(e.target.value)} />
              )}
            </FormField>
            {serverError !== null ? (
              <p role="alert" className="text-sm text-fg-danger">
                {t(serverError)}
              </p>
            ) : null}
          </div>
          <DialogFooter>
            <Button type="button" onClick={submit} disabled={adjust.isPending}>
              {adjust.isPending ? t("action_saving") : t("action_adjust")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

/** A field validator returns a translation key, never a sentence. */
function messageOf(messages: unknown[], t: (key: Key) => string): string | undefined {
  const key = messages.find((m): m is string => typeof m === "string");
  if (key === undefined) return undefined;
  return isSupplierErrorKey(key) ? t(key) : t("error_unknown");
}

/** The keys this screen's own validators hand back. Spelled out rather than
 *  cast, so a key the dictionaries do not carry fails the parity test rather
 *  than printing itself on a fiche. */
const OWN_ERRORS: readonly Key[] = ["error_name_required", "error_name_too_long"];

function isSupplierErrorKey(value: string): value is Key {
  return OWN_ERRORS.some((key) => key === value);
}

/** The update body: the whole fiche without the opening debt, which is a
 *  create-only field, and with no close reason, which the close route
 *  carries. */
function whole(input: NewSupplierDto): SupplierWriteDto {
  const { opening_debt_centimes: _opening, ...fiche } = input;
  return { ...fiche, close_reason: null };
}
