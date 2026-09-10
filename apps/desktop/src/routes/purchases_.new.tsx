// Writing an order. The supplier, the lines, what the goods cost to get
// here, what was handed over now and whether the goods came with the paper.
//
// Nothing here works out a landed cost. The extra costs are spread over the
// lines by value in the core and fixed once (features.md §1), so the screen
// sends what was agreed and reads the answer back; a second arithmetic here
// would be a second answer the day the rule changes.
//
// The day the order is dated is the shop's, asked of the server: a browser
// reads the machine's zone, which on a laptop set wrong is another day
// (lib/clock.ts).
//
// The lines are a `DataTable` of controls rather than a stack of labelled
// fields. A line has three columns and an order has as many lines as the
// delivery had, so labelling every cell of every row would say "Produit,
// Quantité, Prix d'achat" once per row; the column headings say it once and
// each control keeps the same name for a screen reader through `aria-label`.

import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, Plus, Trash2 } from "lucide-react";
import { useId, useState } from "react";
import { parseQtyToMilli } from "@dzpos/shared";
import type { NewPurchaseDto, PaymentMethodDto } from "@dzpos/shared";

import { DataTable, type Column } from "@/components/DataTable";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { MoneyInput } from "@/components/MoneyInput";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { api, productsQueryKey, purchasesQueryKey, suppliersQueryKey } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { useShopToday } from "@/lib/clock";
import { cleared, errorKey } from "@/lib/fields";

export const Route = createFileRoute("/purchases_/new")({ component: NewPurchaseScreen });

/** Exported for the screen's own tests, which mount it under a memory router
 *  rather than through the file route. */
export { NewPurchaseScreen };

const PAYMENT_METHODS: readonly PaymentMethodDto[] = ["cash", "card"];

const PAYMENT_METHOD_KEY: Record<PaymentMethodDto, Key> = {
  cash: "payment_cash",
  card: "payment_card",
};

/** One row of the line editor, as it is typed. The quantity is text until
 *  the request is built, because a half-typed quantity is not a number yet;
 *  the cost is already centimes, because `MoneyInput` owns that reading and
 *  no float is ever made from it. */
interface DraftLine {
  /** Its own identity, so a row removed in the middle takes its own values
   *  with it rather than the ones the index used to point at. */
  readonly key: number;
  productId: string;
  qty: string;
  unitCost: number | null;
}

function blankLine(key: number): DraftLine {
  return { key, productId: "", qty: "", unitCost: null };
}

function NewPurchaseScreen() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const today = useShopToday();
  const receiveId = useId();

  const suppliers = useQuery({
    queryKey: [...suppliersQueryKey, ""],
    queryFn: () => api.listSuppliers(),
  });
  const products = useQuery({ queryKey: productsQueryKey, queryFn: () => api.listProducts() });

  const [supplierId, setSupplierId] = useState("");
  const [documentNumber, setDocumentNumber] = useState("");
  const [dueDate, setDueDate] = useState("");
  const [transport, setTransport] = useState<number | null>(null);
  const [extra, setExtra] = useState<number | null>(null);
  const [note, setNote] = useState("");
  const [lines, setLines] = useState<DraftLine[]>([blankLine(0)]);
  const [nextKey, setNextKey] = useState(1);
  const [paidNow, setPaidNow] = useState<number | null>(null);
  const [paidMode, setPaidMode] = useState<PaymentMethodDto>("cash");
  const [receiveNow, setReceiveNow] = useState(true);
  const [problem, setProblem] = useState<Key | null>(null);

  const save = useMutation({
    mutationFn: (order: NewPurchaseDto) => api.createPurchase(order),
    onSuccess: async (made) => {
      setProblem(null);
      await queryClient.invalidateQueries({ queryKey: purchasesQueryKey });
      // The delivery moved stock and what the shop owes, so the supplier
      // list and every product list are both stale.
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
      await queryClient.invalidateQueries({ queryKey: productsQueryKey });
      await navigate({ to: "/purchases/$id", params: { id: String(made.purchase.id) } });
    },
    onError: (error: unknown) => setProblem(errorKey(error)),
  });

  const setLine = (key: number, patch: Partial<DraftLine>) => {
    setLines((current) => current.map((line) => (line.key === key ? { ...line, ...patch } : line)));
  };

  const addLine = () => {
    setLines((current) => [...current, blankLine(nextKey)]);
    setNextKey((key) => key + 1);
  };

  const removeLine = (key: number) => {
    setLines((current) => (current.length === 1 ? current : current.filter((l) => l.key !== key)));
  };

  const submit = async () => {
    const day = today.today;
    if (day === undefined) {
      setProblem("purchases_no_day");
      return;
    }
    const supplier = Number(supplierId);
    if (!Number.isInteger(supplier) || supplier <= 0) {
      setProblem("purchases_pick_supplier");
      return;
    }
    const built: NewPurchaseDto["lines"] = [];
    for (const line of lines) {
      // A blank row is a row nobody filled in, not a line ordering nothing.
      if (line.productId === "" && line.qty.trim() === "" && line.unitCost === null) continue;
      const productId = Number(line.productId);
      const qty = parseQtyToMilli(line.qty);
      if (!Number.isInteger(productId) || productId <= 0 || qty === null || qty <= 0) {
        setProblem("purchases_line_incomplete");
        return;
      }
      if (line.unitCost === null || line.unitCost < 0) {
        setProblem("purchases_line_incomplete");
        return;
      }
      built.push({
        product_id: productId,
        qty_ordered_milli: qty,
        unit_cost_centimes: line.unitCost,
      });
    }
    if (built.length === 0) {
      setProblem("purchases_no_line");
      return;
    }
    let paid: NewPurchaseDto["paid_now"] = null;
    if (paidNow !== null) {
      if (paidNow <= 0) {
        setProblem("error_amount_unreadable");
        return;
      }
      paid = { amount_centimes: paidNow, payment_mode: paidMode };
    }
    // The rejection is swallowed on purpose: onError has already turned the
    // server's code into a translated message on the form.
    await save
      .mutateAsync({
        supplier_id: supplier,
        supplier_document_number: cleared(documentNumber),
        purchase_date: day,
        due_date: cleared(dueDate),
        // Blank is nothing at all, which the core reads as no cost.
        transport_centimes: transport ?? 0,
        extra_costs_centimes: extra ?? 0,
        note: cleared(note),
        lines: built,
        paid_now: paid,
        receive_now: receiveNow,
      })
      .catch(() => undefined);
  };

  const columns: readonly Column<DraftLine>[] = [
    {
      id: "product",
      header: t("col_product"),
      cell: (line) => (
        <Select
          value={line.productId}
          onValueChange={(next) => setLine(line.key, { productId: next })}
        >
          <SelectTrigger aria-label={t("col_product")} className="w-full">
            <SelectValue placeholder={t("purchases_pick_product")} />
          </SelectTrigger>
          <SelectContent>
            {(products.data ?? []).map((p) => (
              <SelectItem key={p.id} value={String(p.id)}>
                {p.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      ),
    },
    {
      id: "qty",
      header: t("col_qty"),
      numeric: true,
      cell: (line) => (
        <Input
          dir="ltr"
          inputMode="decimal"
          autoComplete="off"
          aria-label={t("col_qty")}
          className="w-24 text-end font-numeric tabular-nums"
          value={line.qty}
          onChange={(event) => setLine(line.key, { qty: event.target.value })}
        />
      ),
    },
    {
      id: "cost",
      header: t("col_unit_cost"),
      money: true,
      cell: (line) => (
        <MoneyInput
          aria-label={t("col_unit_cost")}
          className="w-32"
          value={line.unitCost}
          onChange={(centimes) => setLine(line.key, { unitCost: centimes })}
        />
      ),
    },
  ];

  return (
    <section className="flex flex-col gap-4">
      <PageHeader
        title={t("purchases_new")}
        actions={
          <Button asChild variant="outline">
            <Link to="/purchases">
              <Icon as={ArrowLeft} size={18} flip />
              {t("action_back_to_purchases")}
            </Link>
          </Button>
        }
      />

      <form
        className="flex flex-col gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <Card>
          <CardHeader>
            <h3 className="text-md font-semibold">{t("purchases_block_supplier")}</h3>
          </CardHeader>
          <CardContent className="grid gap-4 sm:grid-cols-3">
            <FormField label={t("col_supplier")} required>
              {(parts) => (
                <Select value={supplierId} onValueChange={setSupplierId}>
                  <SelectTrigger id={parts.id} aria-label={t("col_supplier")} className="w-full">
                    <SelectValue placeholder={t("purchases_pick_supplier")} />
                  </SelectTrigger>
                  <SelectContent>
                    {(suppliers.data ?? [])
                      // A closed fiche refuses an order, so it is not offered.
                      .filter((s) => s.active)
                      .map((s) => (
                        <SelectItem key={s.id} value={String(s.id)}>
                          {s.name}
                        </SelectItem>
                      ))}
                  </SelectContent>
                </Select>
              )}
            </FormField>
            <FormField label={t("col_supplier_document")}>
              {(parts) => (
                <Input
                  {...parts}
                  dir="ltr"
                  autoComplete="off"
                  value={documentNumber}
                  onChange={(e) => setDocumentNumber(e.target.value)}
                />
              )}
            </FormField>
            <FormField label={t("field_due_date")}>
              {(parts) => (
                <Input
                  {...parts}
                  type="date"
                  dir="ltr"
                  value={dueDate}
                  onChange={(e) => setDueDate(e.target.value)}
                />
              )}
            </FormField>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <h3 className="text-md font-semibold">{t("purchases_lines")}</h3>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <DataTable
              columns={columns}
              rows={lines}
              rowKey={(line) => line.key}
              caption={t("purchases_lines")}
              actions={(line) => (
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  aria-label={`${t("purchases_remove_line")} ${String(
                    lines.findIndex((l) => l.key === line.key) + 1,
                  )}`}
                  onClick={() => removeLine(line.key)}
                >
                  <Icon as={Trash2} size={18} />
                </Button>
              )}
            />
            <div>
              <Button type="button" variant="outline" onClick={addLine}>
                <Icon as={Plus} size={18} />
                {t("purchases_add_line")}
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <h3 className="text-md font-semibold">{t("purchases_block_costs")}</h3>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <FormField label={t("field_transport")}>
                {(parts) => <MoneyInput {...parts} value={transport} onChange={setTransport} />}
              </FormField>
              <FormField label={t("field_extra_costs")}>
                {(parts) => <MoneyInput {...parts} value={extra} onChange={setExtra} />}
              </FormField>
            </div>
            <p className="text-sm text-muted-foreground">{t("purchases_landed_hint")}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <h3 className="text-md font-semibold">{t("purchases_block_payment")}</h3>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <FormField label={t("field_paid_now")}>
                {(parts) => <MoneyInput {...parts} value={paidNow} onChange={setPaidNow} />}
              </FormField>
              <FormField label={t("field_payment_mode")}>
                {(parts) => (
                  <Select
                    value={paidMode}
                    onValueChange={(next) => {
                      const chosen = PAYMENT_METHODS.find((m) => m === next);
                      if (chosen !== undefined) setPaidMode(chosen);
                    }}
                  >
                    <SelectTrigger
                      id={parts.id}
                      aria-label={t("field_payment_mode")}
                      className="w-full"
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {PAYMENT_METHODS.map((mode) => (
                        <SelectItem key={mode} value={mode}>
                          {t(PAYMENT_METHOD_KEY[mode])}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </FormField>
            </div>
            <div className="flex items-center gap-2">
              {/* The label points at the box by id: a Radix checkbox is a
                  button, and a button is labelled by `for` and not by being
                  wrapped. */}
              <Checkbox
                id={receiveId}
                checked={receiveNow}
                onCheckedChange={(next) => setReceiveNow(next === true)}
              />
              <Label htmlFor={receiveId}>{t("field_receive_now")}</Label>
            </div>
            <p className="text-sm text-muted-foreground">{t("purchases_receive_now_hint")}</p>
          </CardContent>
        </Card>

        <FormField label={t("col_note")}>
          {(parts) => (
            <Input {...parts} value={note} onChange={(e) => setNote(e.target.value)} />
          )}
        </FormField>

        {problem === null ? null : (
          <p role="alert" className="text-sm text-fg-danger">
            {t(problem)}
          </p>
        )}
        {today.error === null ? null : (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(today.error))}
          </p>
        )}

        <div>
          <Button type="submit" disabled={save.isPending}>
            {t("purchases_save")}
          </Button>
        </div>
      </form>
    </section>
  );
}
