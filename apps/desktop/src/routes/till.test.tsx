// The till screen: what it filters, what it refuses before the network, and
// the exact JSON it posts. Business rules are the core's (architecture.md
// rule 2), so what is pinned here is the wiring: centimes and thousandths on
// the wire, the API's codes translated, and the two refusals the screen owns
// (a fractional quantity on a product sold by the piece, a line discount
// above its own line) never reaching the server at all.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import type { CategoryDto, CustomerDto, ProductDto, SaleDto, SettingsDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import { SessionProvider } from "@/lib/session";
import { TillScreen } from "./till";

// The two products of the till_cash_sale_two_rates fixture case: 400,00 DA
// at 19 % sold by the piece and 200,00 DA at 9 % sold by the kilo. Selling
// two of the first and 1,5 kg of the second is the basket that case pins.
const coffee: ProductDto = {
  id: 1,
  shop_id: 1,
  name: "Café Moulu 250g",
  barcode: "6130002000017",
  category_id: 1,
  unit: "piece",
  cost_centimes: 30_000,
  selling_centimes: 40_000,
  wholesale_centimes: null,
  qty_on_hand_milli: 10_000,
  low_stock_at_milli: 2_000,
  rate_bps: 1900,
  active: true,
};

const tomato: ProductDto = {
  ...coffee,
  id: 2,
  name: "Tomate fraîche",
  barcode: "6130002000024",
  category_id: 2,
  unit: "kg",
  selling_centimes: 20_000,
  qty_on_hand_milli: 5_000,
  rate_bps: 900,
};

/** The other whole unit: a case of water, sold by the box. */
const crate: ProductDto = {
  ...coffee,
  id: 4,
  name: "Pack eau 6x1,5L",
  barcode: "6130002000048",
  category_id: 2,
  unit: "box",
  selling_centimes: 60_000,
  qty_on_hand_milli: 8_000,
  rate_bps: 900,
};

const salt: ProductDto = {
  ...coffee,
  id: 3,
  name: "Sel de table",
  barcode: "6130002000031",
  category_id: 2,
  selling_centimes: 5_000,
  qty_on_hand_milli: 0,
  rate_bps: 0,
};

const categories: CategoryDto[] = [
  { id: 1, shop_id: 1, name: "Général", default_rate_bps: 1900 },
  { id: 2, shop_id: 1, name: "Alimentaire", default_rate_bps: 900 },
];

const settings: SettingsDto = {
  store: { name: "Mon magasin", rc: null, nif: null, nis: null, ai: null, address: null, phone: null },
  regime: { regime: "reel", valid_from: "2026-01-01" },
  regime_planned: null,
  theme: null,
  discount_threshold_bps: 0,
};

/** What the API answers for the fixture basket, so the confirmation shows
 * the server's amounts and not the screen's preview. */
const sale: SaleDto = {
  id: 7,
  shop_id: 1,
  kind: "ticket",
  // What the API actually stores: a document kind code, not a pretty
  // series a cashier could read (crates/core/src/models/sql_types.rs).
  series: "doc_ticket:2026",
  number: 12,
  printed_number: "TK-2026-000012",
  issued_at: "2026-09-09 10:00:00",
  user_id: 1,
  regime: "reel",
  payment_mode: "cash",
  seller: settings.store,
  customer_id: null,
  ref_document_id: null,
  buyer_name: null,
  balance: null,
  totals: {
    total_ht_centimes: 110_000,
    discount_centimes: 0,
    subtotal_ht_centimes: 110_000,
    tva_centimes: 17_900,
    total_ttc_centimes: 127_900,
    stamp_centimes: 1_300,
    net_to_pay_centimes: 129_200,
  },
  tva: [
    { rate_bps: 900, base_centimes: 30_000, amount_centimes: 2_700 },
    { rate_bps: 1900, base_centimes: 80_000, amount_centimes: 15_200 },
  ],
  tendered_centimes: 150_000,
  change_centimes: 20_800,
  status: "issued",
  cancellation: null,
  cancel_effect: null,
  lines: [
    {
      id: 1,
      position: 1,
      product_id: 1,
      name: coffee.name,
      barcode: coffee.barcode,
      qty_milli: 2_000,
      unit_price_centimes: 40_000,
      line_discount_centimes: 0,
      rate_bps: 1900,
      line_total_centimes: 80_000,
      ref_line_id: null,
    },
    {
      id: 2,
      position: 2,
      product_id: 2,
      name: tomato.name,
      barcode: tomato.barcode,
      qty_milli: 1_500,
      unit_price_centimes: 20_000,
      line_discount_centimes: 0,
      rate_bps: 900,
      line_total_centimes: 30_000,
      ref_line_id: null,
    },
  ],
  warning: null,
};

/** A customer who can buy on credit: 5 000,00 of limit, warned at 4 000,00,
 * owing nothing yet. The e2e drives the same three numbers. */
const amrani: CustomerDto = {
  id: 3,
  shop_id: 1,
  name: "Entreprise Amrani",
  party_kind: "company",
  phone: "0555 00 11 22",
  address: null,
  rc: null,
  nif: null,
  nis: null,
  ai: null,
  credit_limit_centimes: 500_000,
  warn_threshold_centimes: 400_000,
  notes: null,
  active: true,
  balance_centimes: 0,
};

/** Zero limit is no credit at all, which is not the same answer as no limit
 * (features.md §1). */
const noCredit: CustomerDto = {
  ...amrani,
  id: 4,
  name: "Boutique Kaci",
  credit_limit_centimes: 0,
  warn_threshold_centimes: null,
};

/** Already past what the shop allowed: the banner says so before a line is
 * even in the cart. */
const overLimit: CustomerDto = {
  ...amrani,
  id: 5,
  name: "Garage Sadi",
  balance_centimes: 600_000,
};

const anonymous: CustomerDto = { ...amrani, id: 6, name: "Fiche fermée", active: false };

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function isInit(value: unknown): value is RequestInit {
  return typeof value === "object" && value !== null;
}

/** The 80 mm page the core renders from the stored document. The panel shows
 * this, not a second rendering of the same numbers, so the string here is
 * what the assertions look for. */
const TICKET_HTML =
  '<!doctype html><html><body><div class="amount-net-to-pay">1 292,00</div></body></html>';

/** The A4 page the core renders from a stored facture. Its own string, so
 * an assertion cannot pass on the ticket's bytes by accident. */
const FACTURE_HTML =
  '<!doctype html><html><body><div class="title">FACTURE</div></body></html>';

function html(status: number, body: string): Response {
  return new Response(body, { status, headers: { "content-type": "text/html; charset=utf-8" } });
}

let fetchMock: ReturnType<typeof vi.fn>;
let rows: ProductDto[];
let customerRows: CustomerDto[];
let saleAnswer: (() => Response) | null;
let ticketAnswer: (() => Response) | null;

/** Every JSON body posted to /sales, oldest first. An override sends the
 * basket a second time, so the two calls have to be told apart. */
function salePosts(): Record<string, unknown>[] {
  const bodies: Record<string, unknown>[] = [];
  for (const call of fetchMock.mock.calls) {
    const init: unknown = call[1];
    if (isInit(init) && init.method === "POST" && String(call[0]).endsWith("/sales")) {
      if (typeof init.body !== "string") throw new Error("the till posted no JSON body");
      bodies.push(JSON.parse(init.body));
    }
  }
  return bodies;
}

/** The JSON body of the first POST to /sales, or undefined if none was made. */
function salePost(): Record<string, unknown> | undefined {
  return salePosts()[0];
}

function posted(): boolean {
  return salePost() !== undefined;
}

/** The URL the print button asked the ticket for, or undefined if it never
 * asked. The query string carries the language, so it is kept whole. */
function ticketFetch(): string | undefined {
  return fetchMock.mock.calls
    .map((call) => String(call[0]))
    .find((url) => url.includes(`/sales/${sale.id}/ticket`));
}

/** The last facture URL the panel asked for, sheet and language included. */
function facturePrinted(): string | undefined {
  return fetchMock.mock.calls
    .map((call) => String(call[0]))
    .filter((url) => url.includes(`/sales/${sale.id}/facture`))
    .pop();
}

beforeEach(() => {
  rows = [coffee, tomato, crate, salt];
  customerRows = [amrani, noCredit, overLimit, anonymous];
  saleAnswer = null;
  ticketAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST" && url.endsWith("/sales")) {
      return Promise.resolve(saleAnswer !== null ? saleAnswer() : json(201, sale));
    }
    if (url.includes("/customers")) return Promise.resolve(json(200, customerRows));
    if (url.endsWith("/categories")) return Promise.resolve(json(200, categories));
    if (url.endsWith("/settings")) return Promise.resolve(json(200, settings));
    if (url.includes(`/sales/${sale.id}/ticket`)) {
      return Promise.resolve(ticketAnswer !== null ? ticketAnswer() : html(200, TICKET_HTML));
    }
    if (url.includes(`/sales/${sale.id}/facture`)) {
      return Promise.resolve(html(200, FACTURE_HTML));
    }
    if (url.endsWith(`/sales/${sale.id}`)) return Promise.resolve(json(200, sale));
    return Promise.resolve(json(200, rows));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

/** The till inside a router of its own, and nothing else in it: the screen
 * carries `Link`s to the customers and the settings screens, and a `Link`
 * outside a router throws. The real route tree would drag the root layout
 * and its navigation into every assertion here, so this is one route with
 * no layout, on a memory history. */
function mount(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const rootRoute = createRootRoute();
  const tillRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: TillScreen,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([tillRoute]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        {/* TillScreen reads `useSession()` for the one flag that guards F9
            under a lock (`locked`); this test never locks anything, so the
            stub's catch-all 200 for `/auth/me` and `/auth/idle` is enough
            to keep the provider out of a rejected-promise churn, and
            `locked` stays the `false` it starts at regardless of what
            those two calls answer. */}
        <SessionProvider>
          {/* The router this test builds is not the app's, and the app's is
              what the `Register` declaration types `Link` against. */}
          <RouterProvider router={router} />
        </SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

/** A tile, scoped to the product grid: the cart's own buttons carry the
 * product name too (the aria-labels of −, + and ✕), so an unscoped query by
 * name matches four things the moment a line exists. */
function tile(p: ProductDto): HTMLElement {
  return within(screen.getByTestId("tiles")).getByRole("button", { name: new RegExp(p.name) });
}

function noTile(p: ProductDto): boolean {
  return (
    within(screen.getByTestId("tiles")).queryByRole("button", { name: new RegExp(p.name) }) === null
  );
}

async function findTile(p: ProductDto): Promise<HTMLElement> {
  return within(await screen.findByTestId("tiles")).findByRole("button", {
    name: new RegExp(p.name),
  });
}

/** The answers to the customer search box. The picked fiche sits above the
 * list rather than inside it, and the tiles carry names too, so the answers
 * are asked for inside their own group. */
function customerList(): HTMLElement {
  return screen.getByRole("group", { name: "Client" });
}

/** Picks a fiche the way a cashier does: the answers are buttons and the one
 * with that name is clicked. The name is a pattern because the row says the
 * balance beside it, the way a tile says its price. */
async function pickCustomer(
  user: ReturnType<typeof userEvent.setup>,
  customer: CustomerDto,
): Promise<void> {
  const name = new RegExp(customer.name);
  await within(customerList()).findByRole("button", { name });
  await user.click(within(customerList()).getByRole("button", { name }));
}

/** Waits for the tiles, then rings up the fixture basket: two coffees and
 * 1,5 kg of tomatoes. */
async function ringUpTheFixtureBasket(user: ReturnType<typeof userEvent.setup>) {
  await findTile(coffee);
  await user.click(tile(coffee));
  await user.click(tile(coffee));
  await user.click(tile(tomato));
  const qty = screen.getByLabelText(`Quantité ${tomato.name}`);
  await user.clear(qty);
  await user.type(qty, "1,5");
}

describe("the product list", () => {
  test("filters on a piece of the name", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.type(screen.getByLabelText("Chercher un produit ou scanner un code-barres"), "tomate");
    expect(tile(tomato)).toBeInTheDocument();
    expect(noTile(coffee)).toBe(true);
  });

  test("filters on a piece of the barcode", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.type(screen.getByLabelText("Chercher un produit ou scanner un code-barres"), "2000024");
    expect(tile(tomato)).toBeInTheDocument();
    expect(noTile(coffee)).toBe(true);
  });

  test("a scanner types the whole barcode and sends Enter: one unit is added and the box clears", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    const box = screen.getByLabelText("Chercher un produit ou scanner un code-barres");
    await user.type(box, `${coffee.barcode}{Enter}`);
    expect(box).toHaveValue("");
    expect(screen.getByLabelText(`Quantité ${coffee.name}`)).toHaveValue("1");
  });

  test("Enter with a single visible tile adds that tile", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.type(
      screen.getByLabelText("Chercher un produit ou scanner un code-barres"),
      "tomate{Enter}",
    );
    expect(screen.getByLabelText(`Quantité ${tomato.name}`)).toHaveValue("1");
  });

  test("Escape clears the search", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    const box = screen.getByLabelText("Chercher un produit ou scanner un code-barres");
    await user.type(box, "tomate{Escape}");
    expect(box).toHaveValue("");
    expect(tile(coffee)).toBeInTheDocument();
  });

  test("a product with nothing left is still sellable and says so", async () => {
    // A shop's count is often wrong before its first inventory, so the tile
    // adds; the tag is what tells the cashier to look at the shelf.
    const user = userEvent.setup();
    mount();
    const saltTile = await findTile(salt);
    expect(within(saltTile).getByText("Rupture")).toBeInTheDocument();
    await user.click(saltTile);
    expect(screen.getByLabelText(`Quantité ${salt.name}`)).toHaveValue("1");
  });
});

describe("the quantity a line carries", () => {
  test("a product sold by the kilo takes 1,5 and posts 1500 thousandths", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    await waitFor(() => expect(posted()).toBe(true));
    const lines: unknown = salePost()?.lines;
    expect(lines).toEqual([
      { product_id: 1, qty_milli: 2_000, unit_price_centimes: null, line_discount_centimes: 0 },
      { product_id: 2, qty_milli: 1_500, unit_price_centimes: null, line_discount_centimes: 0 },
    ]);
  });

  test("a product sold by the piece refuses 1,5 and nothing is posted", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));
    const qty = screen.getByLabelText(`Quantité ${coffee.name}`);
    await user.clear(qty);
    await user.type(qty, "1,5");

    expect(await screen.findByText("Ce produit se vend à l'unité entière.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    expect(posted()).toBe(false);
  });

  test("a product sold by the box refuses 1,5 the same way", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(crate);
    await user.click(tile(crate));
    const qty = screen.getByLabelText(`Quantité ${crate.name}`);
    await user.clear(qty);
    await user.type(qty, "1,5");

    expect(await screen.findByText("Ce produit se vend à l'unité entière.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    expect(posted()).toBe(false);
  });

  test("the minus button takes the quantity the line carries down by one", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));
    await user.click(tile(coffee));
    expect(screen.getByLabelText(`Quantité ${coffee.name}`)).toHaveValue("2");
    await user.click(screen.getByRole("button", { name: `Un de moins ${coffee.name}` }));
    expect(screen.getByLabelText(`Quantité ${coffee.name}`)).toHaveValue("1");
  });

  test("a weighed line steps down by one kilo and keeps its grams", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(tomato));
    const qty = screen.getByLabelText(`Quantité ${tomato.name}`);
    await user.clear(qty);
    await user.type(qty, "1,5");
    await user.click(screen.getByRole("button", { name: `Un de moins ${tomato.name}` }));
    expect(screen.getByLabelText(`Quantité ${tomato.name}`)).toHaveValue("0,5");
  });

  test("stepping a weighed line below zero takes the line off, it never rounds the scale up", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(tomato));
    const qty = screen.getByLabelText(`Quantité ${tomato.name}`);
    await user.clear(qty);
    await user.type(qty, "0,5");
    await user.click(screen.getByRole("button", { name: `Un de moins ${tomato.name}` }));
    // Clamping up to one kilo would charge 100,00 DA the scale never said.
    expect(screen.queryByLabelText(`Quantité ${tomato.name}`)).not.toBeInTheDocument();
    expect(screen.getByText("Le panier est vide.")).toBeInTheDocument();
  });

  test("a quantity that does not read as a number survives both buttons", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(tomato));
    const qty = () => screen.getByLabelText(`Quantité ${tomato.name}`);
    // An emptied box on the way to a new weight. Reading it as zero would
    // delete the line on minus and write 1 over it on plus; `add` keeps such
    // a line too.
    await user.clear(qty());
    await user.click(screen.getByRole("button", { name: `Un de moins ${tomato.name}` }));
    expect(qty()).toHaveValue("");
    await user.click(screen.getByRole("button", { name: `Un de plus ${tomato.name}` }));
    expect(qty()).toHaveValue("");
    // A lone separator is the same: digits are still to come.
    await user.type(qty(), ",");
    await user.click(screen.getByRole("button", { name: `Un de moins ${tomato.name}` }));
    expect(qty()).toHaveValue(",");
  });

  test("the minus button on a single unit takes the line off", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));
    await user.click(screen.getByRole("button", { name: `Un de moins ${coffee.name}` }));
    expect(screen.queryByLabelText(`Quantité ${coffee.name}`)).not.toBeInTheDocument();
  });
});

describe("the discounts the screen refuses on its own", () => {
  test("a line discount above its own line never reaches the API", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));
    // One coffee is 400,00 DA; 400,01 is a centime past the line.
    await user.type(screen.getByLabelText(`Remise ligne (DA) ${coffee.name}`), "400,01");

    expect(await screen.findByText("La remise dépasse la ligne.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    expect(posted()).toBe(false);
  });

  test("a global discount above the basket never reaches the API", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));
    await user.type(screen.getByLabelText("Remise globale (DA)"), "500");

    expect(await screen.findByText("La remise dépasse le panier.")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    expect(posted()).toBe(false);
  });
});

describe("the discounts that do reach the API", () => {
  test("a 50 DA line discount is posted in centimes on its own line", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));
    await user.click(tile(tomato));
    await user.type(screen.getByLabelText(`Remise ligne (DA) ${tomato.name}`), "50");
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1000");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    await waitFor(() => expect(posted()).toBe(true));
    const lines: unknown = salePost()?.lines;
    expect(lines).toEqual([
      { product_id: 1, qty_milli: 1_000, unit_price_centimes: null, line_discount_centimes: 0 },
      { product_id: 2, qty_milli: 1_000, unit_price_centimes: null, line_discount_centimes: 5_000 },
    ]);
  });

  test("a 50 DA global discount is posted in centimes on the basket", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));
    await user.type(screen.getByLabelText("Remise globale (DA)"), "50");
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1000");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(salePost()?.global_discount_centimes).toBe(5_000);
    expect(salePost()?.lines).toEqual([
      { product_id: 1, qty_milli: 1_000, unit_price_centimes: null, line_discount_centimes: 0 },
    ]);
  });
});

describe("the live totals", () => {
  test("show the fixture case the API is pinned by, before anything is posted", async () => {
    // till_cash_sale_two_rates in fixtures/money/tva_rounding_once_per_rate:
    // HT 1 100,00, TVA 27,00 + 152,00, stamp 13,00, net 1 292,00.
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    // The totals are a few named amounts and not a list, so they are a
    // description block rather than a table with a header row saying nothing.
    const totals = screen.getByTestId("till-totals");
    expect(within(totals).getByText("1 100,00")).toBeInTheDocument();
    expect(within(totals).getByText("27,00")).toBeInTheDocument();
    expect(within(totals).getByText("152,00")).toBeInTheDocument();
    expect(within(totals).getByText("13,00")).toBeInTheDocument();
    expect(within(totals).getByText("1 292,00")).toBeInTheDocument();
    expect(posted()).toBe(false);
  });

  test("the stamp goes away when the sale is paid by card", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.click(screen.getByRole("radio", { name: "Carte" }));
    // The totals are a few named amounts and not a list, so they are a
    // description block rather than a table with a header row saying nothing.
    const totals = screen.getByTestId("till-totals");
    expect(within(totals).queryByText("13,00")).not.toBeInTheDocument();
    expect(within(totals).getByText("1 279,00")).toBeInTheDocument();
  });

  test("the change follows what the customer hands over", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    expect(screen.getByTestId("till-change")).toHaveTextContent("208,00");
  });

  test("too little cash is refused before the request", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1000");
    expect(
      await screen.findByText("Le montant reçu est inférieur au net à payer."),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    expect(posted()).toBe(false);
  });

  test("an empty tendered box is a cashier who has not typed yet, not a mistake", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    // Nothing red on a basket that was only just rung up.
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.queryByTestId("till-change")).not.toBeInTheDocument();
    // And nothing goes out either, by the button or by the key.
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    await user.keyboard("{F9}");
    expect(posted()).toBe(false);
  });
});

describe("paying", () => {
  test("a cash sale posts the whole basket, the mode and the tendered amount", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(salePost()).toEqual({
      lines: [
        { product_id: 1, qty_milli: 2_000, unit_price_centimes: null, line_discount_centimes: 0 },
        { product_id: 2, qty_milli: 1_500, unit_price_centimes: null, line_discount_centimes: 0 },
      ],
      global_discount_centimes: 0,
      payment_mode: "cash",
      tendered_centimes: 150_000,
      customer_id: null,
      override: false,
      kind: "ticket",
    });
  });

  test("a card sale posts no tendered amount", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.click(screen.getByRole("radio", { name: "Carte" }));
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(salePost()?.payment_mode).toBe("card");
    expect(salePost()?.tendered_centimes).toBeNull();
  });

  /**
   * The pad a thumb hits. It types whole dinars, so 1, 5 then the double zero
   * is 1 500 DA, and the box above it shows the same integer the keyboard
   * would have put there. Its wide key does what F9 and the brass button do.
   */
  test("the pad types the amount handed over and its wide key takes the sale", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);

    const pad = within(screen.getByTestId("keypad"));
    for (const key of ["1", "5", "00"]) {
      await user.click(pad.getByRole("button", { name: key }));
    }
    expect(screen.getByTestId("till-change")).toHaveTextContent("208,00");

    await user.click(pad.getByRole("button", { name: "Valider" }));
    await waitFor(() => expect(posted()).toBe(true));
    expect(salePost()?.tendered_centimes).toBe(150_000);
  });

  test("the pad's backspace empties the box rather than leaving a zero in it", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);

    const pad = within(screen.getByTestId("keypad"));
    await user.click(pad.getByRole("button", { name: "9" }));
    await user.click(pad.getByRole("button", { name: "Effacer un chiffre" }));

    // Nothing counted yet: no change is shown and the sale waits.
    expect(screen.queryByTestId("till-change")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Encaisser" })).toBeDisabled();
  });

  test("F9 pays without touching the button", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.keyboard("{F9}");

    await waitFor(() => expect(posted()).toBe(true));
    expect(salePost()?.payment_mode).toBe("cash");
  });

  test("F9 on an empty cart posts nothing", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.keyboard("{F9}");
    expect(posted()).toBe(false);
  });

  test("credit is drawn, disabled and says what it needs until a customer is picked", async () => {
    mount();
    const credit = await screen.findByRole("radio", { name: "Crédit" });
    expect(credit).toBeDisabled();
    // The reason is on the control itself, so hovering the thing that
    // refuses is what explains it.
    expect(credit).toHaveAttribute(
      "title",
      "Choisissez un client qui peut acheter à crédit.",
    );
  });

  test("the confirmation shows the server's number, net to pay and change, and the cart clears", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    const done = await screen.findByRole("status");
    expect(done).toHaveTextContent("12");
    // The series is a code the API keys documents by, so it stays off a
    // screen where every other word is translated.
    expect(done).not.toHaveTextContent("doc_ticket:2026");
    expect(done).toHaveTextContent("1 292,00");
    expect(done).toHaveTextContent("208,00");
    expect(screen.getByText("Le panier est vide.")).toBeInTheDocument();
  });

  test("the search box is empty again for the next customer", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    const box = screen.getByLabelText("Chercher un produit ou scanner un code-barres");
    await user.type(box, "tomate");
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    await screen.findByRole("status");
    // A filter left behind would take the next scan's digits on its end and
    // match nothing.
    expect(box).toHaveValue("");
  });

  test("the print button shows the page the core rendered, in an iframe", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    await screen.findByRole("status");
    await user.click(screen.getByRole("button", { name: "Imprimer" }));

    const receipt = await screen.findByRole("region", { name: "Détail du ticket" });
    const frame = await within(receipt).findByTestId("till-ticket");
    // The page goes in as it came. A panel that re-rendered the numbers
    // could disagree with the paper the printer puts out; this cannot.
    expect(frame).toHaveAttribute("srcdoc", TICKET_HTML);
    expect(ticketFetch()).toBe(`http://127.0.0.1:4317/sales/${sale.id}/ticket?lang=fr`);
  });

  test("a ticket the core could not render is shown by its code", async () => {
    const user = userEvent.setup();
    ticketAnswer = () =>
      json(500, {
        error: { code: "print", message: "ticket_80mm.html: unknown variable net_to_pay" },
      });
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    await screen.findByRole("status");
    await user.click(screen.getByRole("button", { name: "Imprimer" }));

    expect(
      await screen.findByText("Le ticket n'a pas pu être préparé pour l'impression."),
    ).toBeInTheDocument();
    expect(screen.queryByText(/ticket_80mm/)).not.toBeInTheDocument();
    expect(screen.queryByTestId("till-ticket")).not.toBeInTheDocument();
  });

  test("the API's refusal is shown by its code, never its message", async () => {
    const user = userEvent.setup();
    saleAnswer = () =>
      json(422, { error: { code: "validation", message: "qty_milli: a sold quantity is above zero" } });
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    expect(await screen.findByText("Vérifiez les champs saisis.")).toBeInTheDocument();
    expect(screen.queryByText(/qty_milli/)).not.toBeInTheDocument();
  });
});

describe("in Arabic", () => {
  test("the amounts of the totals block stay left to right", async () => {
    const user = userEvent.setup();
    mount("ar");
    await findTile(coffee);
    await user.click(tile(coffee));
    const cell = screen.getByTestId("total-net-to-pay");
    expect(cell).toHaveAttribute("dir", "ltr");
  });

  test("the ticket is asked for in the language the till is being used in", async () => {
    // The core prints in the language it is told. A cashier working in
    // Arabic hands over an Arabic ticket, so the UI language is what goes on
    // the query string; the French test above passes on a hardcoded "fr".
    const user = userEvent.setup();
    mount("ar");
    await findTile(coffee);
    await user.click(tile(coffee));
    await user.type(screen.getByLabelText("المبلغ المدفوع (دج)"), "1500");
    await user.click(screen.getByRole("button", { name: "الدفع" }));
    await screen.findByRole("status");
    await user.click(screen.getByRole("button", { name: "طباعة" }));

    await screen.findByTestId("till-ticket");
    expect(ticketFetch()).toBe(`http://127.0.0.1:4317/sales/${sale.id}/ticket?lang=ar`);
  });
});

// The sale on credit. The rules are the core's (features.md §1); what is
// pinned here is what the screen posts, what it refuses to post, and what it
// shows a cashier when the server refuses.
describe("on credit", () => {
  /** Rings up one coffee and picks the customer, which is the shortest
   * basket that can go on credit. */
  async function ringUpFor(
    user: ReturnType<typeof userEvent.setup>,
    customer: CustomerDto,
  ): Promise<void> {
    await findTile(coffee);
    await user.click(tile(coffee));
    await pickCustomer(user, customer);
  }

  async function payOnCredit(user: ReturnType<typeof userEvent.setup>): Promise<void> {
    await user.click(screen.getByRole("radio", { name: "Crédit" }));
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
  }

  test("the body carries the customer and the credit mode", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpFor(user, amrani);
    await payOnCredit(user);

    await screen.findByRole("status");
    expect(salePost()).toMatchObject({
      payment_mode: "credit",
      customer_id: amrani.id,
      tendered_centimes: null,
      override: false,
    });
  });

  test("credit is not on offer without a customer, and nothing is posted", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));
    const credit = screen.getByRole("radio", { name: "Crédit" });
    expect(credit).toBeDisabled();
    await user.click(credit);
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    expect(posted()).toBe(false);
  });

  test("a customer with a limit of zero cannot buy on credit", async () => {
    // Zero is no credit at all; null would be no limit at all. The screen
    // has to tell the two apart the way the core does.
    const user = userEvent.setup();
    mount();
    await ringUpFor(user, noCredit);
    expect(screen.getByRole("radio", { name: "Crédit" })).toBeDisabled();
  });

  test("a customer already past the limit is flagged before a line is rung up", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpFor(user, overLimit);
    const banner = await screen.findByTestId("till-limit-banner");
    expect(banner).toHaveTextContent("Plafond dépassé");
    // The two amounts, the customer's own balance against their limit.
    expect(banner).toHaveTextContent("6 000,00");
    expect(banner).toHaveTextContent("5 000,00");
    // The two amounts are one expression, not two islands: each `Money`
    // brings its own `dir`, and on an Arabic till the slash between them
    // put the ceiling on the left and the balance on the right, which is
    // the comparison read backwards.
    const pair = within(banner).getByTestId("till-limit-pair");
    expect(pair.getAttribute("dir")).toBe("ltr");
    expect(pair).toHaveTextContent("6 000,00 / 5 000,00");
  });

  test("a closed fiche is never offered", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));
    await within(customerList()).findByRole("button", { name: new RegExp(amrani.name) });
    expect(
      within(customerList()).queryByRole("button", { name: new RegExp(anonymous.name) }),
    ).not.toBeInTheDocument();
  });

  test("the refusal shows both amounts and the override resends the same basket", async () => {
    const user = userEvent.setup();
    saleAnswer = () =>
      json(422, {
        error: {
          code: "credit_limit",
          message: "past the limit",
          balance_after_centimes: 550_000,
          credit_limit_centimes: 500_000,
        },
      });
    mount();
    await ringUpFor(user, amrani);
    await payOnCredit(user);

    expect(await screen.findByTestId("till-balance-after")).toHaveTextContent("5 500,00");
    expect(screen.getByTestId("till-credit-limit")).toHaveTextContent("5 000,00");
    // The other answer to the refusal: the customer's own fiche, where money
    // comes off what they already owe. The link carries their id, so nobody
    // types the name back into a search box.
    expect(screen.getByRole("link", { name: "Ouvrir la fiche du client" })).toHaveAttribute(
      "href",
      `/customers/${amrani.id}`,
    );
    const refused = salePost();
    expect(refused).toMatchObject({ override: false });

    // The override asks first, and a cashier who says no sends nothing.
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    await user.click(screen.getByRole("button", { name: "Forcer la vente" }));
    expect(confirm).toHaveBeenCalled();
    expect(fetchMock.mock.calls.filter((c) => String(c[0]).endsWith("/sales")).length).toBe(1);

    // Said yes, the same basket goes again with the flag on.
    confirm.mockReturnValue(true);
    saleAnswer = () =>
      json(201, { ...sale, payment_mode: "credit", customer_id: amrani.id, tendered_centimes: null });
    await user.click(screen.getByRole("button", { name: "Forcer la vente" }));
    await screen.findByRole("status");
    const posts = salePosts();
    expect(posts).toHaveLength(2);
    expect(posts[1]).toEqual({ ...refused, override: true });
  });

  test("editing the basket takes the refusal away, so the override cannot resend what is gone", async () => {
    const user = userEvent.setup();
    saleAnswer = () =>
      json(422, {
        error: {
          code: "credit_limit",
          message: "past the limit",
          balance_after_centimes: 550_000,
          credit_limit_centimes: 500_000,
        },
      });
    mount();
    await ringUpFor(user, amrani);
    await payOnCredit(user);
    await screen.findByTestId("till-balance-after");

    // The line the refusal was about goes off the cart. What the button
    // would resend is no longer what the cashier is looking at, so there
    // is no button.
    await user.click(screen.getByRole("button", { name: `Un de moins ${coffee.name}` }));
    expect(screen.queryByRole("button", { name: "Forcer la vente" })).not.toBeInTheDocument();
    expect(screen.queryByTestId("till-balance-after")).not.toBeInTheDocument();
    expect(salePosts()).toHaveLength(1);
  });

  test("a sale that reaches the warning threshold still goes through and says so", async () => {
    const user = userEvent.setup();
    saleAnswer = () =>
      json(201, {
        ...sale,
        payment_mode: "credit",
        customer_id: amrani.id,
        tendered_centimes: null,
        change_centimes: null,
        balance: {
          old_balance_centimes: 0,
          remaining_debt_centimes: 450_000,
          total_debt_centimes: 450_000,
        },
        warning: "near_limit",
      });
    mount();
    await ringUpFor(user, amrani);
    await payOnCredit(user);

    const done = await screen.findByRole("status");
    expect(within(done).getByTestId("till-near-limit")).toBeInTheDocument();
    // The balance the document stores, not one the screen worked out.
    expect(within(done).getByTestId("till-new-balance")).toHaveTextContent("4 500,00");
  });
});

describe("the facture at the till", () => {
  /** One coffee, and the customer picked, which is the shortest basket a
   * facture can be made out for. */
  async function ringUpFor(
    user: ReturnType<typeof userEvent.setup>,
    customer: CustomerDto,
  ): Promise<void> {
    await findTile(coffee);
    await user.click(tile(coffee));
    await pickCustomer(user, customer);
  }

  /** The answer the API gives for a facture: its own kind, its own series
   * and the number as the paper prints it. */
  const issued: SaleDto = {
    ...sale,
    kind: "facture",
    series: "doc_facture:2026",
    number: 1,
    printed_number: "FA-2026-000001",
    customer_id: amrani.id,
  };

  test("the facture is not on offer without a customer and the body says ticket", async () => {
    const user = userEvent.setup();
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));

    const facture = screen.getByRole("radio", { name: "Facture" });
    expect(facture).toBeDisabled();
    // Disabled and saying why, the way the credit choice does.
    expect(facture).toHaveAttribute(
      "title",
      "Une facture est établie au nom d'un client.",
    );
    expect(screen.getByRole("radio", { name: "Ticket" })).toBeChecked();

    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    await waitFor(() => expect(posted()).toBe(true));
    expect(salePost()).toMatchObject({ kind: "ticket" });
  });

  test("the proforma needs a customer too and posts its own kind", async () => {
    const user = userEvent.setup();
    saleAnswer = () => json(201, { ...issued, kind: "proforma" });
    mount();
    await findTile(coffee);
    await user.click(tile(coffee));

    // A quotation is made out to somebody the way a facture is, so it is
    // offered on the same terms and says why when it is not.
    const proforma = screen.getByRole("radio", { name: "Proforma" });
    expect(proforma).toBeDisabled();
    expect(proforma).toHaveAttribute(
      "title",
      "Une proforma est établie au nom d'un client : choisissez-en un.",
    );

    await ringUpFor(user, amrani);
    await user.click(screen.getByRole("radio", { name: "Proforma" }));
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(salePost()).toMatchObject({ kind: "proforma", customer_id: amrani.id });
  });

  test("a quotation is still refused when the basket itself is wrong", async () => {
    // A proforma takes no money, so the cash box and the credit limit are not
    // part of what makes it sendable. What is in the basket still is: a
    // quantity that is not a number is as wrong on a quotation as on a sale,
    // and nothing may be posted while it stands.
    //
    // The quantity and not the discount, which is where this test used to
    // look: the discount is a `MoneyInput` now and an amount is the only
    // thing it can hand back, so "abc" never becomes a discount to refuse.
    // The quantity box is still free text, because "1," on the way to "1,5"
    // has to survive being typed.
    const user = userEvent.setup();
    saleAnswer = () => json(201, { ...issued, kind: "proforma" });
    mount();
    await ringUpFor(user, amrani);
    await user.click(screen.getByRole("radio", { name: "Proforma" }));
    const qty = screen.getByLabelText(`Quantité ${coffee.name}`);
    await user.clear(qty);
    await user.type(qty, "abc");

    // Said out loud, and the button will not send it.
    expect(screen.getByText("Quantité invalide.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Encaisser" })).toBeDisabled();
    await user.clear(qty);
    await user.type(qty, "1");

    // Corrected, and the quotation goes.
    await user.clear(screen.getByLabelText("Remise globale (DA)"));
    await user.type(screen.getByLabelText("Remise globale (DA)"), "50");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    await waitFor(() => expect(posted()).toBe(true));
    expect(salePost()).toMatchObject({ kind: "proforma", global_discount_centimes: 5_000 });
  });

  test("picking a customer opens the switch and the body posts the facture kind", async () => {
    const user = userEvent.setup();
    saleAnswer = () => json(201, issued);
    mount();
    await ringUpFor(user, amrani);
    await user.click(screen.getByRole("radio", { name: "Facture" }));
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    await screen.findByRole("status");
    expect(salePost()).toMatchObject({ kind: "facture", customer_id: amrani.id });
  });

  test("unpicking the customer takes the sale back to a ticket", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpFor(user, amrani);
    await user.click(screen.getByRole("radio", { name: "Facture" }));
    expect(screen.getByRole("radio", { name: "Facture" })).toBeChecked();

    // Back to the walk-in customer: a facture is made out to somebody, so
    // the switch does not stay on a choice the server would refuse.
    await user.click(screen.getByRole("button", { name: "Client de passage" }));
    expect(screen.getByRole("radio", { name: "Ticket" })).toBeChecked();
    expect(screen.getByRole("radio", { name: "Facture" })).toBeDisabled();

    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    await waitFor(() => expect(posted()).toBe(true));
    expect(salePost()).toMatchObject({ kind: "ticket" });
  });

  test("the refusal names the side, the identifiers and where to fill them in", async () => {
    const user = userEvent.setup();
    saleAnswer = () =>
      json(422, {
        error: {
          code: "party_ids",
          message: "the buyer block of a facture is missing rc, nis",
          party_side: "buyer",
          missing_ids: ["rc", "nis"],
        },
      });
    mount();
    await ringUpFor(user, amrani);
    await user.click(screen.getByRole("radio", { name: "Facture" }));
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    const panel = await screen.findByTestId("till-party-ids");
    expect(within(panel).getByText("Il manque à la fiche du client :")).toBeInTheDocument();
    const missing = within(panel).getByTestId("till-party-ids-missing");
    expect(missing).toHaveTextContent("RC");
    expect(missing).toHaveTextContent("NIS");
    // The screen sends the cashier to the half it can fix, not to a
    // generic settings page.
    expect(within(panel).getByRole("link", { name: "Compléter la fiche du client" })).toHaveAttribute(
      "href",
      "/customers",
    );
    // The panel says the whole thing, so the generic line is not repeated
    // underneath it.
    expect(screen.queryByText("Identifiants manquants.")).toBeNull();
  });

  test("a seller block that is short sends the cashier to the settings instead", async () => {
    const user = userEvent.setup();
    saleAnswer = () =>
      json(422, {
        error: {
          code: "party_ids",
          message: "the seller block of a facture is missing nis",
          party_side: "seller",
          missing_ids: ["nis"],
        },
      });
    mount();
    await ringUpFor(user, amrani);
    await user.click(screen.getByRole("radio", { name: "Facture" }));
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    const panel = await screen.findByTestId("till-party-ids");
    expect(within(panel).getByText("Il manque au bloc du magasin :")).toBeInTheDocument();
    expect(
      within(panel).getByRole("link", { name: "Compléter les paramètres du magasin" }),
    ).toHaveAttribute("href", "/settings");
  });

  test("the confirmation and the print panel are the facture's, on the sheet asked for", async () => {
    const user = userEvent.setup();
    saleAnswer = () => json(201, issued);
    mount();
    await ringUpFor(user, amrani);
    await user.click(screen.getByRole("radio", { name: "Facture" }));
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    const done = await screen.findByRole("status");
    expect(within(done).getByText("Facture émise")).toBeInTheDocument();
    // The number as the paper spells it, which is what a customer quotes.
    expect(within(done).getByTestId("till-document-number")).toHaveTextContent("FA-2026-000001");

    await user.click(within(done).getByRole("button", { name: "Imprimer" }));
    const frame = await screen.findByTestId("till-facture");
    expect(frame).toHaveAttribute("srcdoc", FACTURE_HTML);
    expect(facturePrinted()).toContain("paper=a4");
    expect(facturePrinted()).toContain("lang=fr");

    // The half sheet is the same facture on smaller paper, asked for again.
    await user.click(screen.getByRole("radio", { name: "A5" }));
    await waitFor(() => expect(facturePrinted()).toContain("paper=a5"));
    expect(screen.queryByTestId("till-ticket")).toBeNull();
  });
});
