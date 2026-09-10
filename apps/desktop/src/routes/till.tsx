// The till: the screen a cashier lives on. Search or scan on the start
// side, the cart and the totals on the end side, one POST /sales at the end.
//
// This file holds the state of a basket and the one call that turns it into a
// document. The panels it is made of are in `-till/`: the cart, the cash box,
// the customer and the row of keys the three single choices are drawn with.
// Each owns the reading of its own refusal, so the server's answer is turned
// into something a cashier reads in the file that shows it.
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
//
// Every amount on the screen goes through `Money` or `MoneyInput`, so the
// integer centimes never become a float and a column of figures lines up on
// the digit. The one thing set in figures that is not an amount is the
// document number, which is a name and not a sum.

import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  ApiError,
  MoneyError,
  computeTotals,
  formatQty,
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
import { PackageSearch, Printer, ScanLine, ShoppingBasket } from "lucide-react";

import {
  api,
  categoriesQueryKey,
  customersQueryKey,
  productsQueryKey,
  saleFactureQueryKey,
  saleTicketQueryKey,
  settingsQueryKey,
} from "@/api";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { PageHeader } from "@/components/PageHeader";
import { PayButton } from "@/components/PayButton";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useTranslation, type Key } from "@/i18n";

import { Cart, CartHeader, ONE_UNIT_MILLI, readLine } from "./-till/cart";
import type { CartLine } from "./-till/cart";
import { Choice, ChoiceGroup } from "./-till/choice";
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
  const [globalDiscount, setGlobalDiscount] = useState<number | null>(null);
  const [tendered, setTendered] = useState<number | null>(null);
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
      setGlobalDiscount(null);
      setTendered(null);
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
  }, [cart, mode, globalDiscount, tendered]);

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
        return [...current, { product, qtyText: formatQty(ONE_UNIT_MILLI), discount: null }];
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

  function setLineDiscount(id: number, discount: number | null) {
    setCart((current) => current.map((l) => (l.product.id === id ? { ...l, discount } : l)));
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

  const discount = globalDiscount ?? 0;
  const globalDiscountProblem: Key | null = discount < 0 ? "error_discount_invalid" : null;

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
        globalDiscount: discount,
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
  // An empty box on a cash sale is a cashier who has not counted the notes
  // yet, not a mistake: the sale waits, and nothing turns red until an
  // amount has actually been typed.
  const tenderedMissing = mode === "cash" && cart.length > 0 && preview !== null && tendered === null;
  const tenderedProblem: Key | null =
    mode !== "cash" || preview === null || cart.length === 0 || tendered === null
      ? null
      : tendered < 0
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
      global_discount_centimes: discount,
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
    [cart, customer, discount, kind, mode, read, tendered],
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
    <section className="grid gap-4 lg:grid-cols-[1fr_28rem]">
      <div className="flex min-w-0 flex-col gap-3">
        <PageHeader
          title={t("till_title")}
          className="pb-0"
          actions={
            <div className="relative w-80">
              <Icon
                as={ScanLine}
                size={18}
                className="pointer-events-none absolute inset-y-0 start-3 my-auto text-faint"
              />
              <Input
                ref={searchRef}
                type="search"
                className="h-(--control-h-lg) ps-9 text-md"
                aria-label={t("till_search")}
                placeholder={t("till_search")}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                onKeyDown={onSearchKey}
              />
            </div>
          }
        />

        <div className="flex flex-wrap gap-2" role="group" aria-label={t("field_category")}>
          <CategoryChip
            label={t("till_all_categories")}
            active={categoryId === null}
            onPick={() => setCategoryId(null)}
          />
          {(categories.data ?? []).map((c) => (
            <CategoryChip
              key={c.id}
              label={c.name}
              active={categoryId === c.id}
              onPick={() => setCategoryId(c.id)}
            />
          ))}
        </div>

        {products.isPending ? (
          <div className="grid gap-2 [grid-template-columns:repeat(auto-fill,minmax(11rem,1fr))]">
            {Array.from({ length: 8 }, (_, index) => (
              <Skeleton key={index} className="h-24" />
            ))}
            <span className="sr-only">{t("products_loading")}</span>
          </div>
        ) : null}
        {products.isError ? (
          <p role="alert" className="text-fg-danger">
            {t(errorKey(products.error))}
          </p>
        ) : null}

        <div
          data-testid="tiles"
          role="group"
          aria-label={t("till_products")}
          className="grid gap-2 [grid-template-columns:repeat(auto-fill,minmax(11rem,1fr))]"
        >
          {visible.map((p) => (
            <ProductTile key={p.id} product={p} onAdd={() => add(p)} />
          ))}
        </div>
        {products.isSuccess && visible.length === 0 ? (
          <EmptyState icon={PackageSearch} title={t("till_no_product")} />
        ) : null}
      </div>

      <aside className="flex min-w-0 flex-col gap-3 rounded-lg border border-border bg-card p-3">
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
          discount={globalDiscount}
          onDiscount={setGlobalDiscount}
          discountProblem={globalDiscountProblem}
          totalsProblem={totalsProblem}
          totals={preview}
          onQty={setQty}
          onLineDiscount={setLineDiscount}
          onStep={step}
          onRemove={remove}
        />

        <PaymentPanel
          mode={mode}
          onMode={setMode}
          creditAllowed={takesCredit(customer)}
          tendered={tendered}
          onTendered={setTendered}
          onEnter={submit}
          showChange={!tenderedMissing && cart.length > 0}
          change={change}
          problem={tenderedProblem}
          refusal={refusal}
          onOverride={override}
          pending={pay.isPending}
        />

        {partyProblem !== null ? <PartyIdsRefused refusal={partyProblem} /> : null}

        {serverError !== null ? (
          <p role="alert" className="text-fg-danger">
            {t(serverError)}
          </p>
        ) : null}

        <PayButton className="w-full" disabled={!canPay} onClick={submit}>
          {pay.isPending ? t("action_paying") : t("action_pay")}
        </PayButton>

        {receiptId !== null && done !== null ? (
          <Receipt id={receiptId} kind={done.kind} paper={paper} onPaper={setPaper} />
        ) : null}
      </aside>
    </section>
  );
}

/** One filter of the grid. A chip is a toggle and says so (`aria-pressed`),
 * because "all categories" is not a fourth category and pressing it twice
 * must not mean two different things. */
function CategoryChip({
  label,
  active,
  onPick,
}: {
  label: string;
  active: boolean;
  onPick: () => void;
}) {
  return (
    <Button
      type="button"
      size="sm"
      variant="outline"
      aria-pressed={active}
      className={active ? "rounded-full border-primary bg-primary-soft text-primary" : "rounded-full"}
      onClick={onPick}
    >
      {label}
    </Button>
  );
}

/**
 * The tile the grid is made of: a name, a price and what is left on the
 * shelf. It is local to this screen on purpose and for a day only. The kit's
 * `ProductTile` is being drawn on the products screen at the same hour as
 * this file, and the two agents cannot both land it; when it merges, this
 * function goes and the grid renders that one, which is where the tile's
 * radius, its card surface and its shadow are pinned.
 *
 * A product with nothing on the shelf is still sellable. A shop sells what it
 * has just been handed and counts it in later, and the till refusing the sale
 * would send that customer away; the tile says the count is at zero and the
 * stock movement goes negative, which is a truth the recount fixes.
 */
function ProductTile({ product, onAdd }: { product: ProductDto; onAdd: () => void }) {
  const { t } = useTranslation();
  const out = product.qty_on_hand_milli <= 0;
  return (
    <Button
      type="button"
      variant="outline"
      className="h-auto flex-col items-start gap-2 p-3 text-start whitespace-normal"
      onClick={onAdd}
    >
      <span className="font-medium">{product.name}</span>
      <span className="flex w-full items-center justify-between gap-2">
        <Money centimes={product.selling_centimes} />
        {out ? (
          <Badge variant="secondary">{t("till_out_of_stock")}</Badge>
        ) : (
          <Badge variant="outline" className="font-numeric tabular-nums" dir="ltr">
            {formatQty(product.qty_on_hand_milli)}
          </Badge>
        )}
      </span>
    </Button>
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
  const facture = sale.kind === "facture";
  return (
    <Card role="status" className="gap-2 border-primary p-3">
      <strong>{t(facture ? "till_paid_facture" : "till_paid")}</strong>
      <p className="flex items-center justify-between gap-2">
        <span>{t(facture ? "till_facture" : "till_ticket")}</span>
        {/* `printed_number` and not the integer beside it: FA-2026-000001 is what
            the paper says and what a customer quotes back, and the core
            spells it once (print::number) so the screen cannot spell it
            differently. `series` is a column value and no word at all. It is
            not an amount, so it wears the figure face by hand rather than
            going through `Money`. */}
        <span
          data-testid="till-document-number"
          dir="ltr"
          className="font-numeric font-medium tabular-nums"
        >
          {sale.printed_number}
        </span>
      </p>
      <p className="flex items-center justify-between gap-2">
        <span>{t("total_net_to_pay")}</span>
        <Money centimes={sale.totals.net_to_pay_centimes} />
      </p>
      {sale.change_centimes !== null ? (
        <p className="flex items-center justify-between gap-2">
          <span>{t("till_change")}</span>
          <Money centimes={sale.change_centimes} />
        </p>
      ) : null}
      {/* The balance the document stores, not one the screen worked out:
          the ledger has one answer and the core gave it. */}
      {sale.balance !== null ? (
        <p className="flex items-center justify-between gap-2">
          <span>{t("till_new_balance")}</span>
          <Money centimes={sale.balance.total_debt_centimes} data-testid="till-new-balance" />
        </p>
      ) : null}
      {warningKey(sale.warning) !== null ? (
        <p data-testid="till-near-limit" className="text-sm text-warn">
          {t(warningKey(sale.warning) ?? "error_unknown")}
        </p>
      ) : null}
      <div className="flex gap-2">
        <Button type="button" variant="outline" onClick={onPrint}>
          <Icon as={Printer} size={18} />
          {t("till_print")}
        </Button>
        <Button type="button" variant="ghost" onClick={onNew}>
          <Icon as={ShoppingBasket} size={18} />
          {t("till_new_sale")}
        </Button>
      </div>
    </Card>
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
    <section aria-label={t("till_receipt")} className="flex flex-col gap-2">
      <strong>{t("till_receipt")}</strong>
      {/* A4 is what a facture is filed on; the A5 half sheet is the one a
          counter printer is loaded with. The same page either way: the
          sheet changes the @page size the core writes and nothing else
          (features.md §4). */}
      {facture ? (
        <ChoiceGroup label={t("till_paper")}>
          <Choice
            checked={paper === "a4"}
            label={t("till_paper_a4")}
            onPick={() => onPaper("a4")}
          />
          <Choice
            checked={paper === "a5"}
            label={t("till_paper_a5")}
            onPick={() => onPaper("a5")}
          />
        </ChoiceGroup>
      ) : null}
      {page.isPending ? <Skeleton className="h-96 w-full" /> : null}
      {page.isError ? (
        <p role="alert" className="text-fg-danger">
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
          className="h-96 w-full rounded-md border border-border bg-background"
          data-testid={facture ? "till-facture" : "till-ticket"}
        />
      ) : null}
    </section>
  );
}
