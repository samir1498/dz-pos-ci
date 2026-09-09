// The till: the screen a cashier lives on. Search or scan on the start
// side, the cart and the totals on the end side, one POST /sales at the end.
//
// Two rules about the amounts on this screen, both from architecture.md
// rule 2. The totals it shows while the cart is being built are a preview,
// computed by `computeTotals` in @dzpos/shared, the same module the same
// fixtures pin the Rust core with; the amounts the ticket carries are the
// ones the API answered with, never the preview. And the refusals the screen
// makes on its own (a quantity that is not a whole unit on a product sold by
// the piece, a discount above its own line, cash below the net to pay) are
// there to keep a cashier from posting a basket the core would reject; the
// core still refuses it, and its code is what the screen shows if it does.

import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  ApiError,
  MoneyError,
  computeTotals,
  formatCentimes,
  formatQty,
  lineTotal,
  parseAmountToCentimes,
  parseQtyToMilli,
} from "@dzpos/shared";
import type {
  NewSaleDto,
  PaymentModeDto,
  ProductDto,
  RegimeDto,
  SaleDto,
  Totals,
  TotalsLine,
  UnitDto,
} from "@dzpos/shared";
import {
  api,
  categoriesQueryKey,
  productsQueryKey,
  saleTicketQueryKey,
  settingsQueryKey,
} from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { rateCellLabel } from "@/lib/rate";

export const Route = createFileRoute("/till")({ component: TillScreen });

/** Whether the droit de timbre applies at all. `STAMP_ENABLED` in
 * crates/core/src/services/sales.rs is the same constant: there is no shop
 * setting for it yet, and a cash payment is still what makes it due. When it
 * becomes a setting, both read the setting. */
const STAMP_ENABLED = true;

/** One unit, in thousandths. What a tile adds and what the + button adds. */
const ONE_UNIT_MILLI = 1_000;

/** Units a shop cannot sell a fraction of. A half box is not a thing a
 * receipt can say, and the core would take the 500 without a word. */
const WHOLE_UNITS: readonly UnitDto[] = ["piece", "box"];

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  duplicate_barcode: "error_duplicate_barcode",
  not_found: "error_not_found",
  money: "error_money",
  print: "error_print",
  storage: "error_storage",
  exhausted: "error_exhausted",
  bad_request: "error_bad_request",
  unauthorized: "error_unauthorized",
  bad_response: "error_bad_response",
  unreachable: "error_unreachable",
};

/** The server sends a code, never a sentence; the UI owns the wording. */
function errorKey(error: unknown): Key {
  if (error instanceof ApiError) return ERROR_KEY[error.code] ?? "error_unknown";
  return "error_unknown";
}

/** A refusal `computeTotals` makes, as something the cashier can read. The
 * screen catches the ones a field can explain before this is reached; this
 * is the floor under them. */
const MONEY_ERROR_KEY: Record<string, Key> = {
  Overflow: "error_money",
  RateOutOfRange: "error_money",
  NegativeQuantity: "error_qty_invalid",
  NegativeUnitPrice: "error_money",
  NegativeDiscount: "error_discount_invalid",
  LineDiscountAboveLine: "error_discount_above_line",
  GlobalDiscountAboveTotal: "error_discount_above_total",
};

/** A cart line as the cashier is editing it: the quantity and the discount
 * stay the text that was typed, so "1," on the way to "1,5" is not thrown
 * away by a parse and written back as "1". */
interface CartLine {
  readonly product: ProductDto;
  readonly qtyText: string;
  readonly discountText: string;
}

/** A line read: its amounts if they are readable, the field message if not. */
interface ReadLine {
  readonly qtyMilli: number;
  readonly discount: number;
  readonly gross: number;
  readonly problem: Key | null;
}

function readLine(line: CartLine): ReadLine {
  const qtyMilli = parseQtyToMilli(line.qtyText);
  if (qtyMilli === null || qtyMilli <= 0) {
    return { qtyMilli: 0, discount: 0, gross: 0, problem: "error_qty_invalid" };
  }
  if (WHOLE_UNITS.includes(line.product.unit) && qtyMilli % ONE_UNIT_MILLI !== 0) {
    return { qtyMilli, discount: 0, gross: 0, problem: "error_qty_whole" };
  }
  const gross = lineTotal(line.product.selling_centimes, qtyMilli);
  const discount =
    line.discountText.trim() === "" ? 0 : (parseAmountToCentimes(line.discountText) ?? -1);
  if (discount < 0) {
    return { qtyMilli, discount: 0, gross, problem: "error_discount_invalid" };
  }
  if (discount > gross) {
    return { qtyMilli, discount, gross, problem: "error_discount_above_line" };
  }
  return { qtyMilli, discount, gross, problem: null };
}

export function TillScreen() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const searchRef = useRef<HTMLInputElement>(null);

  const [search, setSearch] = useState("");
  const [categoryId, setCategoryId] = useState<number | null>(null);
  const [cart, setCart] = useState<CartLine[]>([]);
  const [globalDiscountText, setGlobalDiscountText] = useState("");
  const [tenderedText, setTenderedText] = useState("");
  const [mode, setMode] = useState<PaymentModeDto>("cash");
  const [done, setDone] = useState<SaleDto | null>(null);
  const [receiptId, setReceiptId] = useState<number | null>(null);
  const [serverError, setServerError] = useState<Key | null>(null);

  const products = useQuery({ queryKey: productsQueryKey, queryFn: () => api.listProducts() });
  const categories = useQuery({ queryKey: categoriesQueryKey, queryFn: () => api.listCategories() });
  const settings = useQuery({ queryKey: settingsQueryKey, queryFn: () => api.getSettings() });

  const pay = useMutation({
    mutationFn: (input: NewSaleDto) => api.createSale(input),
    onSuccess: async (issued: SaleDto) => {
      setServerError(null);
      setDone(issued);
      setReceiptId(null);
      setCart([]);
      setGlobalDiscountText("");
      setTenderedText("");
      // The next customer's first scan goes into this box; a filter left
      // over from the last basket would take its digits on the end and
      // match nothing.
      setSearch("");
      // The sale moved stock, so the tiles owe the shop a new count.
      await queryClient.invalidateQueries({ queryKey: productsQueryKey });
      searchRef.current?.focus();
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  const rows: ProductDto[] = products.data ?? [];
  const query = search.trim().toLowerCase();
  const visible = rows.filter((p) => {
    if (categoryId !== null && p.category_id !== categoryId) return false;
    if (query === "") return true;
    return p.name.toLowerCase().includes(query) || (p.barcode ?? "").toLowerCase().includes(query);
  });

  const add = useCallback((product: ProductDto) => {
    setDone(null);
    setReceiptId(null);
    setServerError(null);
    setCart((current) => {
      const found = current.find((l) => l.product.id === product.id);
      if (found === undefined) {
        return [...current, { product, qtyText: formatQty(ONE_UNIT_MILLI), discountText: "" }];
      }
      // A quantity being typed is not a number yet; one more of a line whose
      // box reads "1," starts again from one rather than throwing the whole
      // line away.
      const milli = parseQtyToMilli(found.qtyText);
      const next = milli === null || milli <= 0 ? ONE_UNIT_MILLI : milli + ONE_UNIT_MILLI;
      return current.map((l) =>
        l.product.id === product.id ? { ...l, qtyText: formatQty(next) } : l,
      );
    });
    searchRef.current?.focus();
  }, []);

  function setQty(id: number, qtyText: string) {
    setCart((current) => current.map((l) => (l.product.id === id ? { ...l, qtyText } : l)));
  }

  // The two buttons move a line by one whole unit. A step that lands at or
  // below zero takes the line off, the way the × does: raising 0,5 kg to a
  // full kilo would charge the customer for weight the scale never read.
  // A box that does not read as a number yet (emptied, or a lone separator)
  // is left alone, the way `add` keeps such a line: reading it as zero would
  // delete the line on one button and write 1 over what is being typed on
  // the other.
  function step(id: number, by: number) {
    setCart((current) =>
      current.flatMap((l) => {
        if (l.product.id !== id) return [l];
        const milli = parseQtyToMilli(l.qtyText);
        if (milli === null) return [l];
        const next = milli + by;
        if (next <= 0) return [];
        return [{ ...l, qtyText: formatQty(next) }];
      }),
    );
  }

  const read = cart.map(readLine);
  const lineProblem = read.find((r) => r.problem !== null)?.problem ?? null;

  const globalDiscount =
    globalDiscountText.trim() === "" ? 0 : parseAmountToCentimes(globalDiscountText);
  const globalDiscountProblem: Key | null =
    globalDiscount === null || globalDiscount < 0 ? "error_discount_invalid" : null;

  const regime: RegimeDto = settings.data?.regime.regime ?? "reel";
  let preview: Totals | null = null;
  let totalsProblem: Key | null = null;
  if (lineProblem === null && globalDiscountProblem === null && settings.isSuccess) {
    const lines: TotalsLine[] = cart.map((line, i) => ({
      qtyMilli: read[i].qtyMilli,
      unitPrice: line.product.selling_centimes,
      lineDiscount: read[i].discount,
      rateBps: line.product.rate_bps,
    }));
    try {
      preview = computeTotals(lines, {
        globalDiscount: globalDiscount ?? 0,
        paymentMode: mode,
        stampEnabled: STAMP_ENABLED,
        regime,
      });
    } catch (error: unknown) {
      preview = null;
      totalsProblem =
        error instanceof MoneyError ? MONEY_ERROR_KEY[error.variant] ?? "error_money" : "error_money";
    }
  }

  const netToPay = preview?.netToPay ?? 0;
  const tenderedBlank = tenderedText.trim() === "";
  const tendered = tenderedBlank ? 0 : parseAmountToCentimes(tenderedText);
  // An empty box on a cash sale is a cashier who has not counted the notes
  // yet, not a mistake: the sale waits, and nothing turns red until an
  // amount has actually been typed.
  const tenderedMissing =
    mode === "cash" && cart.length > 0 && preview !== null && tenderedBlank;
  const tenderedProblem: Key | null =
    mode !== "cash" || preview === null || cart.length === 0 || tenderedBlank
      ? null
      : tendered === null || tendered < 0
        ? "error_price_invalid"
        : tendered < netToPay
          ? "error_tendered_short"
          : null;
  const change = mode === "cash" && tendered !== null ? tendered - netToPay : 0;

  const problem = lineProblem ?? globalDiscountProblem ?? totalsProblem ?? tenderedProblem;
  const canPay =
    cart.length > 0 && preview !== null && problem === null && !tenderedMissing && !pay.isPending;

  const submit = useCallback(() => {
    if (!canPay || preview === null) return;
    const body: NewSaleDto = {
      lines: cart.map((line, i) => ({
        product_id: line.product.id,
        qty_milli: read[i].qtyMilli,
        // Left unset on purpose: the core prices the line from the stored
        // product (services::sales::price). Sending the price the tile is
        // showing would let a stale screen pin an old price onto a fiscal
        // document.
        unit_price_centimes: null,
        line_discount_centimes: read[i].discount,
      })),
      global_discount_centimes: globalDiscount ?? 0,
      payment_mode: mode,
      tendered_centimes: mode === "cash" ? (tendered ?? 0) : null,
    };
    pay.mutate(body);
  }, [canPay, cart, globalDiscount, mode, pay, preview, read, tendered]);

  // F9 pays, the way a till keyboard does. Held in a ref so the listener is
  // installed once and still sees the cart as it is now.
  const submitRef = useRef(submit);
  submitRef.current = submit;
  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key !== "F9") return;
      event.preventDefault();
      submitRef.current();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  function onSearchKey(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      setSearch("");
      return;
    }
    if (event.key !== "Enter") return;
    event.preventDefault();
    const typed = search.trim();
    if (typed === "") return;
    // A scanner types the barcode and sends Enter. An exact barcode is a
    // decision, not a filter: it adds and clears, whatever else matched.
    const scanned = rows.find((p) => p.barcode === typed);
    if (scanned !== undefined) {
      add(scanned);
      setSearch("");
      return;
    }
    const only = visible.length === 1 ? visible[0] : undefined;
    if (only !== undefined) add(only);
  }

  return (
    <section className="grid gap-4 lg:grid-cols-[1fr_24rem]">
      <div className="flex flex-col gap-3">
        <h1 className="sr-only">{t("till_title")}</h1>
        <input
          ref={searchRef}
          type="search"
          className="w-full rounded border px-3 py-2 text-lg"
          aria-label={t("till_search")}
          placeholder={t("till_search")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={onSearchKey}
        />

        <div className="flex flex-wrap gap-2" role="group" aria-label={t("field_category")}>
          <button
            type="button"
            aria-pressed={categoryId === null}
            className={chipClass(categoryId === null)}
            onClick={() => setCategoryId(null)}
          >
            {t("till_all_categories")}
          </button>
          {(categories.data ?? []).map((c) => (
            <button
              key={c.id}
              type="button"
              aria-pressed={categoryId === c.id}
              className={chipClass(categoryId === c.id)}
              onClick={() => setCategoryId(c.id)}
            >
              {c.name}
            </button>
          ))}
        </div>

        {products.isPending ? <p>{t("products_loading")}</p> : null}
        {products.isError ? (
          <p role="alert" className="text-red-700">
            {t(errorKey(products.error))}
          </p>
        ) : null}

        <div
          data-testid="tiles"
          className="grid gap-2 [grid-template-columns:repeat(auto-fill,minmax(11rem,1fr))]"
        >
          {visible.map((p) => (
            <button
              key={p.id}
              type="button"
              className="flex flex-col gap-2 rounded border p-3 text-start"
              onClick={() => add(p)}
            >
              <span className="font-medium">{p.name}</span>
              <span className="flex items-center justify-between gap-2">
                <span className="font-mono" dir="ltr">
                  {formatCentimes(p.selling_centimes)}
                </span>
                {p.qty_on_hand_milli <= 0 ? (
                  <span className="rounded border px-1 text-xs">{t("till_out_of_stock")}</span>
                ) : (
                  <span className="rounded border px-1 text-xs font-mono" dir="ltr">
                    {formatQty(p.qty_on_hand_milli)}
                  </span>
                )}
              </span>
            </button>
          ))}
        </div>
        {products.isSuccess && visible.length === 0 ? <p>{t("till_no_product")}</p> : null}
      </div>

      <aside className="flex flex-col gap-3 rounded border p-3">
        <header className="flex items-center justify-between gap-2">
          <strong>{`${t("till_cart")} · ${cart.length}`}</strong>
          <button
            type="button"
            className="rounded border px-2 py-1 text-sm"
            disabled={cart.length === 0}
            onClick={() => setCart([])}
          >
            {t("till_clear")}
          </button>
        </header>

        {done !== null ? (
          <Confirmation
            sale={done}
            onPrint={() => setReceiptId(done.id)}
            onNew={() => {
              setDone(null);
              setReceiptId(null);
              searchRef.current?.focus();
            }}
          />
        ) : null}

        <div data-testid="cart" className="flex flex-col gap-3">
          {cart.length === 0 ? <p>{t("till_cart_empty")}</p> : null}
          {cart.map((line, i) => (
            <CartRow
              key={line.product.id}
              line={line}
              read={read[i]}
              onQty={(value) => setQty(line.product.id, value)}
              onDiscount={(value) =>
                setCart((current) =>
                  current.map((l) =>
                    l.product.id === line.product.id ? { ...l, discountText: value } : l,
                  ),
                )
              }
              onStep={(by) => step(line.product.id, by)}
              onRemove={() =>
                setCart((current) => current.filter((l) => l.product.id !== line.product.id))
              }
            />
          ))}
        </div>

        <label className="flex flex-col gap-1">
          <span>{t("field_global_discount")}</span>
          <input
            dir="ltr"
            inputMode="decimal"
            className="rounded border px-2 py-1 text-end font-mono"
            value={globalDiscountText}
            onChange={(e) => setGlobalDiscountText(e.target.value)}
          />
        </label>
        {globalDiscountProblem !== null || totalsProblem !== null ? (
          <p role="alert" className="text-sm text-red-700">
            {t(globalDiscountProblem ?? totalsProblem ?? "error_unknown")}
          </p>
        ) : null}

        {preview !== null ? <TotalsTable totals={preview} /> : null}

        <fieldset className="flex flex-wrap gap-3 border-0 p-0">
          <legend className="mb-1">{t("payment_mode")}</legend>
          <PaymentChoice mode="cash" current={mode} label={t("pay_cash")} onPick={setMode} />
          <PaymentChoice mode="card" current={mode} label={t("pay_card")} onPick={setMode} />
          {/* Drawn and disabled: features.md §3 gives credit a customer
              ledger, and customers arrive in M2. The mockup promises the
              choice, so the screen keeps it visible and says when. */}
          <PaymentChoice
            mode="credit"
            current={mode}
            label={t("pay_credit")}
            title={t("pay_credit_later")}
            disabled
            onPick={setMode}
          />
        </fieldset>

        {mode === "cash" ? (
          <label className="flex flex-col gap-1">
            <span>{t("field_tendered")}</span>
            <input
              dir="ltr"
              inputMode="decimal"
              className="rounded border px-2 py-1 text-end font-mono"
              value={tenderedText}
              onChange={(e) => setTenderedText(e.target.value)}
            />
          </label>
        ) : null}
        {mode === "cash" && tenderedProblem === null && !tenderedMissing && cart.length > 0 ? (
          <p className="flex items-center justify-between gap-2">
            <span>{t("till_change")}</span>
            <span data-testid="till-change" className="font-mono" dir="ltr">
              {formatCentimes(change)}
            </span>
          </p>
        ) : null}
        {tenderedProblem !== null ? (
          <p role="alert" className="text-sm text-red-700">
            {t(tenderedProblem)}
          </p>
        ) : null}

        {serverError !== null ? (
          <p role="alert" className="text-red-700">
            {t(serverError)}
          </p>
        ) : null}

        <button
          type="button"
          className="rounded border px-3 py-2 text-lg font-semibold"
          disabled={!canPay}
          onClick={submit}
        >
          {pay.isPending ? t("action_paying") : t("action_pay")}
        </button>

        {receiptId !== null ? <Receipt id={receiptId} /> : null}
      </aside>
    </section>
  );
}

function chipClass(active: boolean): string {
  return active ? "rounded-full border px-3 py-1 font-semibold" : "rounded-full border px-3 py-1";
}

function PaymentChoice({
  mode,
  current,
  label,
  title,
  disabled = false,
  onPick,
}: {
  mode: PaymentModeDto;
  current: PaymentModeDto;
  label: string;
  title?: string;
  disabled?: boolean;
  onPick: (mode: PaymentModeDto) => void;
}) {
  return (
    <label className="flex items-center gap-2" title={title}>
      <input
        type="radio"
        name="payment_mode"
        value={mode}
        checked={current === mode}
        disabled={disabled}
        onChange={() => onPick(mode)}
      />
      <span>{label}</span>
    </label>
  );
}

function CartRow({
  line,
  read,
  onQty,
  onDiscount,
  onStep,
  onRemove,
}: {
  line: CartLine;
  read: ReadLine;
  onQty: (value: string) => void;
  onDiscount: (value: string) => void;
  onStep: (by: number) => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation();
  const net = read.problem === null ? read.gross - read.discount : 0;
  return (
    <div className="flex flex-col gap-1 border-t pt-2">
      <div className="flex items-start justify-between gap-2">
        <span className="font-medium">{line.product.name}</span>
        <span className="font-mono" dir="ltr">
          {read.problem === null ? formatCentimes(net) : ""}
        </span>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          className="rounded border px-2"
          aria-label={`${t("till_qty_decrease")} ${line.product.name}`}
          onClick={() => onStep(-1_000)}
        >
          −
        </button>
        <input
          dir="ltr"
          inputMode="decimal"
          size={5}
          className="w-16 rounded border px-2 py-1 text-end font-mono"
          aria-label={`${t("field_qty")} ${line.product.name}`}
          value={line.qtyText}
          onChange={(e) => onQty(e.target.value)}
        />
        <button
          type="button"
          className="rounded border px-2"
          aria-label={`${t("till_qty_increase")} ${line.product.name}`}
          onClick={() => onStep(1_000)}
        >
          +
        </button>
        <span className="font-mono text-sm" dir="ltr">
          {`× ${formatCentimes(line.product.selling_centimes)}`}
        </span>
        <input
          dir="ltr"
          inputMode="decimal"
          size={6}
          className="w-20 rounded border px-2 py-1 text-end font-mono"
          aria-label={`${t("field_line_discount")} ${line.product.name}`}
          value={line.discountText}
          onChange={(e) => onDiscount(e.target.value)}
        />
        <button
          type="button"
          className="ms-auto rounded border px-2"
          aria-label={`${t("till_line_remove")} ${line.product.name}`}
          onClick={onRemove}
        >
          ✕
        </button>
      </div>
      {read.problem !== null ? (
        <span role="alert" className="text-sm text-red-700">
          {t(read.problem)}
        </span>
      ) : null}
    </div>
  );
}

/** The preview, column for column with the totals table of features.md §3.
 * A row worth nothing is not printed, the way the ticket does not print it. */
function TotalsTable({ totals }: { totals: Totals }) {
  const { t } = useTranslation();
  return (
    <table className="w-full" aria-label={t("total_net_to_pay")}>
      <tbody>
        <TotalsRow label={t("total_ht")} centimes={totals.totalHt} />
        {totals.discount > 0 ? (
          <TotalsRow label={t("total_discount")} centimes={totals.discount} />
        ) : null}
        {totals.tvaByRate.map((g) => (
          <TotalsRow
            key={g.rateBps}
            label={`${t("total_tva")} ${rateCellLabel(g.rateBps, t)}`}
            centimes={g.amount}
          />
        ))}
        {totals.stamp > 0 ? <TotalsRow label={t("total_stamp")} centimes={totals.stamp} /> : null}
        <TotalsRow
          label={t("total_net_to_pay")}
          centimes={totals.netToPay}
          testId="total-net-to-pay"
          strong
        />
      </tbody>
    </table>
  );
}

function TotalsRow({
  label,
  centimes,
  testId,
  strong = false,
}: {
  label: string;
  centimes: number;
  testId?: string;
  strong?: boolean;
}) {
  return (
    <tr className={strong ? "font-semibold" : undefined}>
      <th scope="row" className="py-0.5 text-start font-normal">
        {label}
      </th>
      <td data-testid={testId} className="py-0.5 text-end font-mono" dir="ltr">
        {formatCentimes(centimes)}
      </td>
    </tr>
  );
}

/** What the API answered, not what the screen computed. */
function Confirmation({
  sale,
  onPrint,
  onNew,
}: {
  sale: SaleDto;
  onPrint: () => void;
  onNew: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div role="status" className="flex flex-col gap-2 rounded border p-3">
      <strong>{t("till_paid")}</strong>
      <p className="flex items-center justify-between gap-2">
        <span>{t("till_ticket")}</span>
        {/* The number only. `series` is the code the core keys documents by
            ("doc_ticket", models/sql_types.rs), not a word in any of the
            three languages, and the label beside it already says ticket. */}
        <span className="font-mono" dir="ltr">
          {sale.number}
        </span>
      </p>
      <p className="flex items-center justify-between gap-2">
        <span>{t("total_net_to_pay")}</span>
        <span className="font-mono" dir="ltr">
          {formatCentimes(sale.totals.net_to_pay_centimes)}
        </span>
      </p>
      {sale.change_centimes !== null ? (
        <p className="flex items-center justify-between gap-2">
          <span>{t("till_change")}</span>
          <span className="font-mono" dir="ltr">
            {formatCentimes(sale.change_centimes)}
          </span>
        </p>
      ) : null}
      <div className="flex gap-2">
        <button type="button" className="rounded border px-3 py-1.5" onClick={onPrint}>
          {t("till_print")}
        </button>
        <button type="button" className="rounded border px-3 py-1.5" onClick={onNew}>
          {t("till_new_sale")}
        </button>
      </div>
    </div>
  );
}

/**
 * The ticket itself, not a screen that resembles it. `GET /sales/{id}/ticket`
 * hands back the 80 mm page the core rendered from the stored document, and
 * that page goes into an iframe as it came: the cashier is looking at what
 * the printer will put on paper, down to the rounding, rather than at a
 * second rendering of the same numbers that could disagree with it.
 *
 * The language is the one the till is being used in. The core prints in the
 * language it is told, and a cashier working in Arabic hands over an Arabic
 * ticket.
 *
 * `srcDoc` rather than a `src` URL: the page arrives as a string the client
 * already fetched with the launch token, and an iframe pointed at the route
 * would ask for it again without one.
 */
function Receipt({ id }: { id: number }) {
  const { t, lang } = useTranslation();
  const ticket = useQuery({
    queryKey: saleTicketQueryKey(id, lang),
    queryFn: () => api.getSaleTicket(id, lang),
  });
  return (
    <section aria-label={t("till_receipt")} className="flex flex-col gap-2 rounded border p-3">
      <strong>{t("till_receipt")}</strong>
      {ticket.isPending ? <p>{t("products_loading")}</p> : null}
      {ticket.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(ticket.error))}
        </p>
      ) : null}
      {ticket.isSuccess ? (
        <iframe
          title={t("till_receipt")}
          srcDoc={ticket.data}
          className="h-96 w-full border-0"
          data-testid="till-ticket"
        />
      ) : null}
    </section>
  );
}
