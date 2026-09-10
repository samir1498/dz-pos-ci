// The till: the screen a cashier lives on. Search or scan on the start
// side, the cart and the totals on the end side, one POST /sales at the end.
//
// This file holds the state of a basket and the one call that turns it into a
// document. The three panels it is made of are in `-till/`: the cart, the
// cash box and the customer. Each owns the reading of its own refusal, so the
// server's answer is turned into something a cashier reads in the file that
// shows it.
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
  parseAmountToCentimes,
  parseQtyToMilli,
} from "@dzpos/shared";
import type {
  CustomerDto,
  DocumentKindDto,
  NewSaleDto,
  PaymentModeDto,
  PrintPaper,
  ProductDto,
  RegimeDto,
  SaleDto,
  SaleKindDto,
  SaleWarningDto,
  Totals,
  TotalsLine,
} from "@dzpos/shared";
import {
  api,
  categoriesQueryKey,
  customersQueryKey,
  productsQueryKey,
  saleFactureQueryKey,
  saleTicketQueryKey,
  settingsQueryKey,
} from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { Cart, CartHeader, ONE_UNIT_MILLI, readLine } from "./-till/cart";
import type { CartLine } from "./-till/cart";
import { CustomerPanel, PartyIdsRefused, partyRefusal, takesCredit } from "./-till/customer";
import type { PartyRefusal } from "./-till/customer";
import { PaymentPanel, creditRefusal } from "./-till/payment";
import type { CreditRefusal } from "./-till/payment";

export const Route = createFileRoute("/till")({ component: TillScreen });

/** Whether the droit de timbre applies at all. `STAMP_ENABLED` in
 * crates/core/src/services/sales.rs is the same constant: there is no shop
 * setting for it yet, and a cash payment is still what makes it due. When it
 * becomes a setting, both read the setting. */
const STAMP_ENABLED = true;

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  duplicate_barcode: "error_duplicate_barcode",
  not_found: "error_not_found",
  money: "error_money",
  print: "error_print",
  storage: "error_storage",
  exhausted: "error_exhausted",
  bad_request: "error_bad_request",
  credit_limit: "error_credit_limit",
  party_ids: "error_party_ids",
  unauthorized: "error_unauthorized",
  bad_response: "error_bad_response",
  unreachable: "error_unreachable",
};

/** The server sends a code, never a sentence; the UI owns the wording. */
function errorKey(error: unknown): Key {
  if (error instanceof ApiError) return ERROR_KEY[error.code] ?? "error_unknown";
  return "error_unknown";
}

/** What the till says about a sale the server let through anyway. The switch
 * is exhaustive on the union the server publishes: a second warning added to
 * `SaleWarningDto` fails to compile here rather than passing the cashier in
 * silence. */
function warningKey(warning: SaleWarningDto | null): Key | null {
  if (warning === null) return null;
  switch (warning) {
    case "near_limit":
      return "till_near_limit_sold";
    default: {
      const unreachable: never = warning;
      return unreachable;
    }
  }
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
  // The picked fiche itself, not its id: the search box narrows the list
  // under it, and a customer who drops out of the results is still the one
  // this basket is for.
  const [customer, setCustomer] = useState<CustomerDto | null>(null);
  const [customerSearch, setCustomerSearch] = useState("");
  const [refusal, setRefusal] = useState<CreditRefusal | null>(null);
  // Ticket or facture, and on a facture the sheet the print panel lays out.
  // The choice is made before the sale is saved and never by a reprint
  // (features.md §3: the document is due « dès la réalisation de la vente »).
  const [kind, setKind] = useState<SaleKindDto>("ticket");
  const [paper, setPaper] = useState<PrintPaper>("a4");
  const [partyProblem, setPartyProblem] = useState<PartyRefusal | null>(null);

  const products = useQuery({ queryKey: productsQueryKey, queryFn: () => api.listProducts() });
  const categories = useQuery({ queryKey: categoriesQueryKey, queryFn: () => api.listCategories() });
  const settings = useQuery({ queryKey: settingsQueryKey, queryFn: () => api.getSettings() });
  // The server filters on name and phone, so the box sends what was typed
  // rather than filtering a list the till happens to hold.
  const customers = useQuery({
    queryKey: [...customersQueryKey, "till", customerSearch.trim()],
    queryFn: () => api.listCustomers(customerSearch),
  });

  const pay = useMutation({
    mutationFn: (input: NewSaleDto) => api.createSale(input),
    onSuccess: async (issued: SaleDto) => {
      setServerError(null);
      setRefusal(null);
      setDone(issued);
      setReceiptId(null);
      setCart([]);
      setGlobalDiscountText("");
      setTenderedText("");
      // The next basket starts on nobody, and the fiche the sale moved is
      // re-read: its balance is the ledger's answer and this sale changed it.
      setCustomer(null);
      setCustomerSearch("");
      setMode("cash");
      // The next basket is a ticket until somebody says otherwise: a
      // consumer is the till's ordinary case and asks nothing of the buyer.
      setKind("ticket");
      setPartyProblem(null);
      await queryClient.invalidateQueries({ queryKey: customersQueryKey });
      // The next customer's first scan goes into this box; a filter left
      // over from the last basket would take its digits on the end and
      // match nothing.
      setSearch("");
      // The sale moved stock, so the tiles owe the shop a new count.
      await queryClient.invalidateQueries({ queryKey: productsQueryKey });
      searchRef.current?.focus();
    },
    onError: (error: unknown, input: NewSaleDto) => {
      const refused = creditRefusal(error, input);
      const short = partyRefusal(error);
      setRefusal(refused);
      setPartyProblem(short);
      // Each panel says the whole thing, amounts or fields included, so the
      // one line of generic text underneath would only repeat it worse.
      setServerError(refused === null && short === null ? errorKey(error) : null);
    },
  });

  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  // A refusal is about one basket. The override button resends the body the
  // server refused, so the moment any part of that body is edited the
  // refusal is about a basket that no longer exists: pressing it would
  // issue the old sale and throw away what the cashier just typed. Every
  // field the body is built from is watched, and the panel goes with the
  // first keystroke.
  useEffect(() => {
    setRefusal(null);
  }, [cart, mode, globalDiscountText, tenderedText]);

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

  function setDiscount(id: number, discountText: string) {
    setCart((current) => current.map((l) => (l.product.id === id ? { ...l, discountText } : l)));
  }

  function remove(id: number) {
    setCart((current) => current.filter((l) => l.product.id !== id));
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

  // The core refuses a credit sale with no customer on the customer_id
  // field; the screen keeps the cashier from posting one at all, which is
  // the same rule kept in two places on purpose (the comment at the top).
  const creditProblem: Key | null =
    mode === "credit" && !takesCredit(customer) ? "error_credit_needs_customer" : null;

  // What is wrong with the basket, and what is wrong at the cash box. A
  // quotation takes no money, so the second half is not part of what makes it
  // sendable: a proforma is written from a basket and a customer alone. The
  // first half still is. A quantity that is not a number and a global discount
  // above the basket are as wrong on a quotation as on a sale, and the price a
  // customer is quoted is the one they will be charged.
  //
  // Split rather than dropped whole. `preview` is null while any of the basket
  // problems stands, so dropping them all happened to gate correctly, and the
  // day the preview is computed some other way it would stop doing so.
  const basketProblem = lineProblem ?? globalDiscountProblem ?? totalsProblem;
  const tillProblem = tenderedProblem ?? creditProblem;
  const quoting = kind === "proforma";
  const canPay =
    cart.length > 0 &&
    preview !== null &&
    basketProblem === null &&
    (quoting || tillProblem === null) &&
    (quoting || !tenderedMissing) &&
    !pay.isPending;

  const body = useCallback(
    (override: boolean): NewSaleDto => ({
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
      // Nothing is handed over on a quotation, because nothing has been
      // bought: the core refuses an amount on one, and the mode is still
      // sent because it is what decides the droit de timbre the facture
      // would carry (features.md §3).
      tendered_centimes: mode === "cash" && kind !== "proforma" ? (tendered ?? 0) : null,
      customer_id: customer?.id ?? null,
      override,
      kind,
    }),
    [cart, customer, globalDiscount, kind, mode, read, tendered],
  );

  const submit = useCallback(() => {
    if (!canPay || preview === null) return;
    pay.mutate(body(false));
  }, [body, canPay, pay, preview]);

  /** The same basket again, past the limit this time. The body is the one
   * the server refused, so the sale that goes through is the sale that was
   * refused and not a second basket the cashier could have edited between
   * the two calls. */
  const override = useCallback(() => {
    if (refusal === null) return;
    if (!window.confirm(t("till_override_confirm"))) return;
    pay.mutate({ ...refusal.body, override: true });
  }, [pay, refusal, t]);

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
        <CartHeader count={cart.length} onClear={() => setCart([])} />

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

        <CustomerPanel
          picked={customer}
          rows={customers.data ?? []}
          search={customerSearch}
          onSearch={setCustomerSearch}
          onPick={(next) => {
            setCustomer(next);
            setRefusal(null);
            setPartyProblem(null);
            // A facture is made out to somebody. Unpicking the customer
            // falls back to a ticket rather than leaving the switch on a
            // choice the pay button would refuse.
            if (next === null) setKind("ticket");
            // A customer who cannot buy on credit cannot leave the till on
            // credit either: the mode falls back rather than sitting on a
            // choice the pay button silently refuses.
            if (!takesCredit(next)) setMode((m) => (m === "credit" ? "cash" : m));
          }}
          kind={kind}
          onKind={setKind}
        />

        <Cart
          lines={cart}
          read={read}
          discountText={globalDiscountText}
          onDiscountText={setGlobalDiscountText}
          discountProblem={globalDiscountProblem}
          totalsProblem={totalsProblem}
          totals={preview}
          onQty={setQty}
          onDiscount={setDiscount}
          onStep={step}
          onRemove={remove}
        />

        <PaymentPanel
          mode={mode}
          onMode={setMode}
          creditAllowed={takesCredit(customer)}
          tenderedText={tenderedText}
          onTendered={setTenderedText}
          showChange={!tenderedMissing && cart.length > 0}
          change={change}
          problem={tenderedProblem}
          refusal={refusal}
          onOverride={override}
          pending={pay.isPending}
        />

        {partyProblem !== null ? <PartyIdsRefused refusal={partyProblem} /> : null}

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

        {receiptId !== null && done !== null ? (
          <Receipt id={receiptId} kind={done.kind} paper={paper} onPaper={setPaper} />
        ) : null}
      </aside>
    </section>
  );
}

function chipClass(active: boolean): string {
  return active ? "rounded-full border px-3 py-1 font-semibold" : "rounded-full border px-3 py-1";
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
  const facture = sale.kind === "facture";
  return (
    <div role="status" className="flex flex-col gap-2 rounded border p-3">
      <strong>{t(facture ? "till_paid_facture" : "till_paid")}</strong>
      <p className="flex items-center justify-between gap-2">
        <span>{t(facture ? "till_facture" : "till_ticket")}</span>
        {/* `printed_number` and not the integer beside it: FA-2026-000001 is what
            the paper says and what a customer quotes back, and the core
            spells it once (print::number) so the screen cannot spell it
            differently. `series` is a column value and no word at all. */}
        <span data-testid="till-document-number" className="font-mono" dir="ltr">
          {sale.printed_number}
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
      {/* The balance the document stores, not one the screen worked out:
          the ledger has one answer and the core gave it. */}
      {sale.balance !== null ? (
        <p className="flex items-center justify-between gap-2">
          <span>{t("till_new_balance")}</span>
          <span data-testid="till-new-balance" className="font-mono" dir="ltr">
            {formatCentimes(sale.balance.total_debt_centimes)}
          </span>
        </p>
      ) : null}
      {warningKey(sale.warning) !== null ? (
        <p data-testid="till-near-limit" className="text-sm text-amber-700">
          {t(warningKey(sale.warning) ?? "error_unknown")}
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
function Receipt({
  id,
  kind,
  paper,
  onPaper,
}: {
  id: number;
  kind: DocumentKindDto;
  paper: PrintPaper;
  onPaper: (paper: PrintPaper) => void;
}) {
  const { t, lang } = useTranslation();
  const facture = kind === "facture";
  const page = useQuery({
    queryKey: facture ? saleFactureQueryKey(id, lang, paper) : saleTicketQueryKey(id, lang),
    queryFn: () =>
      facture ? api.getSaleFacture(id, lang, paper) : api.getSaleTicket(id, lang),
  });
  return (
    <section aria-label={t("till_receipt")} className="flex flex-col gap-2 rounded border p-3">
      <strong>{t("till_receipt")}</strong>
      {/* A4 is what a facture is filed on; the A5 half sheet is the one a
          counter printer is loaded with. The same page either way: the
          sheet changes the @page size the core writes and nothing else
          (features.md §4). */}
      {facture ? (
        <fieldset className="flex flex-wrap gap-3 border-0 p-0">
          <legend className="mb-1">{t("till_paper")}</legend>
          <PaperChoice paper="a4" current={paper} label={t("till_paper_a4")} onPick={onPaper} />
          <PaperChoice paper="a5" current={paper} label={t("till_paper_a5")} onPick={onPaper} />
        </fieldset>
      ) : null}
      {page.isPending ? <p>{t("products_loading")}</p> : null}
      {page.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(page.error))}
        </p>
      ) : null}
      {page.isSuccess ? (
        <iframe
          title={t("till_receipt")}
          srcDoc={page.data}
          // An empty sandbox: the page carries no script and needs no
          // origin, so what it renders cannot reach this one even if a
          // product name ever slipped past the template's escaping.
          sandbox=""
          className="h-96 w-full border-0"
          data-testid={facture ? "till-facture" : "till-ticket"}
        />
      ) : null}
    </section>
  );
}

function PaperChoice({
  paper,
  current,
  label,
  onPick,
}: {
  paper: PrintPaper;
  current: PrintPaper;
  label: string;
  onPick: (paper: PrintPaper) => void;
}) {
  return (
    <label className="flex items-center gap-2">
      <input
        type="radio"
        name="facture_paper"
        value={paper}
        checked={current === paper}
        onChange={() => onPick(paper)}
      />
      <span>{label}</span>
    </label>
  );
}
