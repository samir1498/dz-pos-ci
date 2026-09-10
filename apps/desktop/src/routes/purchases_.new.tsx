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

import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { parseAmountToCentimes, parseQtyToMilli } from "@dzpos/shared";
import type { NewPurchaseDto, PaymentMethodDto } from "@dzpos/shared";

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

/** One row of the line editor, as it is typed. Everything is text until the
 *  request is built: a half-typed quantity is not a number yet. */
interface DraftLine {
  productId: string;
  qty: string;
  unitCost: string;
}

const BLANK_LINE: DraftLine = { productId: "", qty: "", unitCost: "" };

/** Blank is nothing at all, which the core reads as no cost; anything
 *  unreadable stops the form before the request is built. */
function centimesOrZero(text: string): number | null {
  if (text.trim() === "") return 0;
  return parseAmountToCentimes(text);
}

function NewPurchaseScreen() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const today = useShopToday();

  const suppliers = useQuery({
    queryKey: [...suppliersQueryKey, ""],
    queryFn: () => api.listSuppliers(),
  });
  const products = useQuery({ queryKey: productsQueryKey, queryFn: () => api.listProducts() });

  const [supplierId, setSupplierId] = useState("");
  const [documentNumber, setDocumentNumber] = useState("");
  const [dueDate, setDueDate] = useState("");
  const [transport, setTransport] = useState("");
  const [extra, setExtra] = useState("");
  const [note, setNote] = useState("");
  const [lines, setLines] = useState<DraftLine[]>([BLANK_LINE]);
  const [paidNow, setPaidNow] = useState("");
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

  const setLine = (index: number, field: keyof DraftLine, value: string) => {
    setLines((current) =>
      current.map((line, n) => (n === index ? { ...line, [field]: value } : line)),
    );
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
      if (line.productId === "" && line.qty.trim() === "" && line.unitCost.trim() === "") continue;
      const productId = Number(line.productId);
      const qty = parseQtyToMilli(line.qty);
      const unitCost = parseAmountToCentimes(line.unitCost);
      if (!Number.isInteger(productId) || productId <= 0 || qty === null || qty <= 0) {
        setProblem("purchases_line_incomplete");
        return;
      }
      if (unitCost === null || unitCost < 0) {
        setProblem("purchases_line_incomplete");
        return;
      }
      built.push({
        product_id: productId,
        qty_ordered_milli: qty,
        unit_cost_centimes: unitCost,
      });
    }
    if (built.length === 0) {
      setProblem("purchases_no_line");
      return;
    }
    const transportCentimes = centimesOrZero(transport);
    const extraCentimes = centimesOrZero(extra);
    if (transportCentimes === null || extraCentimes === null) {
      setProblem("error_amount_unreadable");
      return;
    }
    let paid: NewPurchaseDto["paid_now"] = null;
    if (paidNow.trim() !== "") {
      const amount = parseAmountToCentimes(paidNow);
      if (amount === null || amount <= 0) {
        setProblem("error_amount_unreadable");
        return;
      }
      paid = { amount_centimes: amount, payment_mode: paidMode };
    }
    // The rejection is swallowed on purpose: onError has already turned the
    // server's code into a translated message on the form.
    await save
      .mutateAsync({
        supplier_id: supplier,
        supplier_document_number: cleared(documentNumber),
        purchase_date: day,
        due_date: cleared(dueDate),
        transport_centimes: transportCentimes,
        extra_costs_centimes: extraCentimes,
        note: cleared(note),
        lines: built,
        paid_now: paid,
        receive_now: receiveNow,
      })
      .catch(() => undefined);
  };

  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("purchases_new")}</h1>
        <Link to="/purchases" className="underline">
          {t("action_back_to_purchases")}
        </Link>
      </header>

      <form
        className="flex flex-col gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <div className="flex flex-wrap gap-4">
          {/* The label points at the select by id rather than wrapping it:
              a wrapping label's text is its whole content, options included,
              so "Fournisseur" would only match a control whose list is
              empty. */}
          <label className="flex flex-col gap-1" htmlFor="purchase-supplier">
            <span>{t("col_supplier")}</span>
            <select
              id="purchase-supplier"
              className="rounded border px-2 py-1"
              value={supplierId}
              onChange={(e) => setSupplierId(e.target.value)}
            >
              <option value="">{t("purchases_pick_supplier")}</option>
              {(suppliers.data ?? [])
                // A closed fiche refuses an order, so it is not offered.
                .filter((s) => s.active)
                .map((s) => (
                  <option key={s.id} value={String(s.id)}>
                    {s.name}
                  </option>
                ))}
            </select>
          </label>
          <label className="flex flex-col gap-1">
            <span>{t("col_supplier_document")}</span>
            <input
              dir="ltr"
              className="rounded border px-2 py-1"
              value={documentNumber}
              onChange={(e) => setDocumentNumber(e.target.value)}
            />
          </label>
          <label className="flex flex-col gap-1">
            <span>{t("field_due_date")}</span>
            <input
              type="date"
              dir="ltr"
              className="rounded border px-2 py-1"
              value={dueDate}
              onChange={(e) => setDueDate(e.target.value)}
            />
          </label>
        </div>

        <fieldset className="flex flex-col gap-2 rounded border p-3">
          <legend className="px-1">{t("purchases_lines")}</legend>
          {lines.map((line, index) => (
            <div key={index} className="flex flex-wrap items-end gap-3">
              <label className="flex flex-col gap-1" htmlFor={`purchase-line-product-${index}`}>
                <span>{t("col_product")}</span>
                <select
                  id={`purchase-line-product-${index}`}
                  className="rounded border px-2 py-1"
                  value={line.productId}
                  onChange={(e) => setLine(index, "productId", e.target.value)}
                >
                  <option value="">{t("purchases_pick_product")}</option>
                  {(products.data ?? []).map((p) => (
                    <option key={p.id} value={String(p.id)}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </label>
              <label className="flex flex-col gap-1" htmlFor={`purchase-line-qty-${index}`}>
                <span>{t("col_qty")}</span>
                <input
                  id={`purchase-line-qty-${index}`}
                  dir="ltr"
                  inputMode="decimal"
                  className="w-24 rounded border px-2 py-1 font-mono text-end"
                  value={line.qty}
                  onChange={(e) => setLine(index, "qty", e.target.value)}
                />
              </label>
              <label className="flex flex-col gap-1" htmlFor={`purchase-line-cost-${index}`}>
                <span>{t("col_unit_cost")}</span>
                <input
                  id={`purchase-line-cost-${index}`}
                  dir="ltr"
                  inputMode="decimal"
                  className="w-32 rounded border px-2 py-1 font-mono text-end"
                  value={line.unitCost}
                  onChange={(e) => setLine(index, "unitCost", e.target.value)}
                />
              </label>
              <button
                type="button"
                className="rounded border px-2 py-1"
                aria-label={`${t("purchases_remove_line")} ${String(index + 1)}`}
                onClick={() =>
                  setLines((current) =>
                    current.length === 1 ? current : current.filter((_, n) => n !== index),
                  )
                }
              >
                {t("purchases_remove_line")}
              </button>
            </div>
          ))}
          <div>
            <button
              type="button"
              className="rounded border px-3 py-1"
              onClick={() => setLines((current) => [...current, BLANK_LINE])}
            >
              {t("purchases_add_line")}
            </button>
          </div>
        </fieldset>

        <div className="flex flex-wrap gap-4">
          <label className="flex flex-col gap-1">
            <span>{t("field_transport")}</span>
            <input
              dir="ltr"
              inputMode="decimal"
              className="rounded border px-2 py-1 font-mono text-end"
              value={transport}
              onChange={(e) => setTransport(e.target.value)}
            />
          </label>
          <label className="flex flex-col gap-1">
            <span>{t("field_extra_costs")}</span>
            <input
              dir="ltr"
              inputMode="decimal"
              className="rounded border px-2 py-1 font-mono text-end"
              value={extra}
              onChange={(e) => setExtra(e.target.value)}
            />
          </label>
        </div>
        <p className="text-sm opacity-70">{t("purchases_landed_hint")}</p>

        <div className="flex flex-wrap gap-4">
          <label className="flex flex-col gap-1">
            <span>{t("field_paid_now")}</span>
            <input
              dir="ltr"
              inputMode="decimal"
              className="rounded border px-2 py-1 font-mono text-end"
              value={paidNow}
              onChange={(e) => setPaidNow(e.target.value)}
            />
          </label>
          <label className="flex flex-col gap-1" htmlFor="purchase-paid-mode">
            <span>{t("field_payment_mode")}</span>
            <select
              id="purchase-paid-mode"
              className="rounded border px-2 py-1"
              value={paidMode}
              onChange={(e) => {
                const chosen = PAYMENT_METHODS.find((m) => m === e.target.value);
                if (chosen !== undefined) setPaidMode(chosen);
              }}
            >
              {PAYMENT_METHODS.map((mode) => (
                <option key={mode} value={mode}>
                  {t(PAYMENT_METHOD_KEY[mode])}
                </option>
              ))}
            </select>
          </label>
        </div>

        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={receiveNow}
            onChange={(e) => setReceiveNow(e.target.checked)}
          />
          <span>{t("field_receive_now")}</span>
        </label>
        <p className="text-sm opacity-70">{t("purchases_receive_now_hint")}</p>

        <label className="flex flex-col gap-1">
          <span>{t("col_note")}</span>
          <input
            className="rounded border px-2 py-1"
            value={note}
            onChange={(e) => setNote(e.target.value)}
          />
        </label>

        {problem === null ? null : (
          <p role="alert" className="text-red-700">
            {t(problem)}
          </p>
        )}
        {today.error === null ? null : (
          <p role="alert" className="text-red-700">
            {t(errorKey(today.error))}
          </p>
        )}

        <div>
          <button
            type="submit"
            className="rounded border px-3 py-1.5"
            disabled={save.isPending}
          >
            {t("purchases_save")}
          </button>
        </div>
      </form>
    </section>
  );
}
