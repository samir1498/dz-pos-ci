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
import type { CategoryDto, ProductDto, SaleDto, SettingsDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
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
};

/** What the API answers for the fixture basket, so the confirmation shows
 * the server's amounts and not the screen's preview. */
const sale: SaleDto = {
  id: 7,
  shop_id: 1,
  kind: "ticket",
  series: "T-2026",
  number: 12,
  issued_at: "2026-09-09 10:00:00",
  user_id: 1,
  regime: "reel",
  payment_mode: "cash",
  seller: settings.store,
  customer_id: null,
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
    },
  ],
};

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function isInit(value: unknown): value is RequestInit {
  return typeof value === "object" && value !== null;
}

let fetchMock: ReturnType<typeof vi.fn>;
let rows: ProductDto[];
let saleAnswer: (() => Response) | null;

/** The JSON body of the POST to /sales, or undefined if none was made. */
function salePost(): Record<string, unknown> | undefined {
  for (const call of fetchMock.mock.calls) {
    const init: unknown = call[1];
    if (isInit(init) && init.method === "POST" && String(call[0]).endsWith("/sales")) {
      if (typeof init.body !== "string") throw new Error("the till posted no JSON body");
      return JSON.parse(init.body);
    }
  }
  return undefined;
}

function posted(): boolean {
  return salePost() !== undefined;
}

beforeEach(() => {
  rows = [coffee, tomato, salt];
  saleAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST" && url.endsWith("/sales")) {
      return Promise.resolve(saleAnswer !== null ? saleAnswer() : json(201, sale));
    }
    if (url.endsWith("/categories")) return Promise.resolve(json(200, categories));
    if (url.endsWith("/settings")) return Promise.resolve(json(200, settings));
    if (url.endsWith(`/sales/${sale.id}`)) return Promise.resolve(json(200, sale));
    return Promise.resolve(json(200, rows));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function mount(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <TillScreen />
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

describe("the live totals", () => {
  test("show the fixture case the API is pinned by, before anything is posted", async () => {
    // till_cash_sale_two_rates in fixtures/money/tva_rounding_once_per_rate:
    // HT 1 100,00, TVA 27,00 + 152,00, stamp 13,00, net 1 292,00.
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    const totals = screen.getByRole("table", { name: "Net à payer" });
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
    const totals = screen.getByRole("table", { name: "Net à payer" });
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

  test("credit is drawn, disabled, and says when it arrives", async () => {
    mount();
    const credit = await screen.findByRole("radio", { name: "Crédit" });
    expect(credit).toBeDisabled();
    expect(credit.closest("label")).toHaveAttribute(
      "title",
      "Une vente à crédit a besoin d'un compte client, prévu dans la prochaine version.",
    );
  });

  test("the confirmation shows the server's number, net to pay and change, and the cart clears", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));

    const done = await screen.findByRole("status");
    expect(done).toHaveTextContent("T-2026");
    expect(done).toHaveTextContent("12");
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

  test("the print stub opens the stored ticket, lines and totals from the DTO", async () => {
    const user = userEvent.setup();
    mount();
    await ringUpTheFixtureBasket(user);
    await user.type(screen.getByLabelText("Montant reçu (DA)"), "1500");
    await user.click(screen.getByRole("button", { name: "Encaisser" }));
    await screen.findByRole("status");
    await user.click(screen.getByRole("button", { name: "Imprimer" }));

    const receipt = await screen.findByRole("region", { name: "Détail du ticket" });
    expect(within(receipt).getByText(coffee.name)).toBeInTheDocument();
    expect(within(receipt).getByText(tomato.name)).toBeInTheDocument();
    expect(within(receipt).getByText("1 292,00")).toBeInTheDocument();
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
});
