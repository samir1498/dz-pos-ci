// The supplier's statement: the balance, the ledger behind it, and the two
// dialogs that write a movement onto it. `SupplierStatement` is the one
// export the fiche page reads; the ledger table, the payment dialog and the
// adjustment dialog exist only to build the card it renders and are not used
// anywhere else.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Scale, Wallet } from "lucide-react";
import { useState } from "react";
import { ApiError } from "@dzpos/shared";
import type {
  PaymentMethodDto,
  SupplierDto,
  SupplierEntryDto,
  SupplierLedgerDto,
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
import { useTranslation, type Key } from "@/i18n";
import { cleared, errorKey, positive } from "@/lib/fields";
import { PAYMENT_METHODS, PAYMENT_METHOD_KEY } from "@/lib/payment";

import { balanceStatus, supplierBalanceLabel, SUPPLIER_KIND_KEY } from "./parts";

/** The balance, the movements behind it, and the two dialogs that write one. */
export function SupplierStatement({ supplier }: { supplier: SupplierDto }) {
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
