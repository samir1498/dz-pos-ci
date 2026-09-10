// The screen is checked for rendering and wiring, not for business rules:
// those are the API crate's tests (architecture.md, consequence of rule 2).

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { CategoryDto, ProductDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import ar from "@/i18n/ar.json";
import en from "@/i18n/en.json";
import fr from "@/i18n/fr.json";
import { ProductsScreen } from "./products";

const product: ProductDto = {
  id: 1,
  shop_id: 1,
  name: "Huile Elio 5L",
  barcode: "2000010000017",
  category_id: 1,
  unit: "piece",
  cost_centimes: 820,
  selling_centimes: 920,
  wholesale_centimes: null,
  qty_on_hand_milli: 24_000,
  low_stock_at_milli: 10_000,
  rate_bps: 1900,
  active: true,
};

const general: CategoryDto = { id: 1, shop_id: 1, name: "Général", default_rate_bps: 1900 };
const alimentaire: CategoryDto = {
  id: 2,
  shop_id: 1,
  name: "Alimentaire",
  default_rate_bps: 900,
};

/** A printed page: the core renders it, so the screen only ever shows it. */
function html(body: string): Response {
  return new Response(body, { status: 200, headers: { "content-type": "text/html" } });
}

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function isInit(value: unknown): value is RequestInit {
  return typeof value === "object" && value !== null;
}

/** The init of the POST the form made, if it made one. */
function postInit(): RequestInit | undefined {
  return initOf("POST");
}

function initOf(method: string): RequestInit | undefined {
  for (const call of fetchMock.mock.calls) {
    const init: unknown = call[1];
    if (isInit(init) && init.method === method) return init;
  }
  return undefined;
}

/** The URL and JSON body of the PUT the edit form made. */
function putRequest(): { url: string; body: Record<string, unknown> } {
  for (const call of fetchMock.mock.calls) {
    const init: unknown = call[1];
    if (isInit(init) && init.method === "PUT") {
      if (typeof init.body !== "string") throw new Error("the edit posted no JSON body");
      return { url: String(call[0]), body: JSON.parse(init.body) };
    }
  }
  throw new Error("the form never put");
}

function posted(): boolean {
  return postInit() !== undefined;
}

function sentBody(): Record<string, unknown> {
  const init = postInit();
  if (init === undefined) throw new Error("the form never posted");
  if (typeof init.body !== "string") throw new Error("the form posted no JSON body");
  return JSON.parse(init.body);
}

function mount(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <ProductsScreen />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
// The screen now reads two endpoints, so the stub answers by URL rather than
// by call order: a test that counted calls would break the next time a query
// is added.
let rows: ProductDto[];
let categories: CategoryDto[];
let createAnswer: (() => Response) | null;
let updateAnswer: (() => Response) | null;
let labelAnswer: (() => Response) | null;

beforeEach(() => {
  rows = [];
  categories = [general];
  createAnswer = null;
  updateAnswer = null;
  labelAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST" && url.includes("/labels/sheet")) {
      if (labelAnswer !== null) return Promise.resolve(labelAnswer());
      const ids: unknown = JSON.parse(String(init.body));
      const named =
        typeof ids === "object" && ids !== null && Array.isArray(Reflect.get(ids, "ids"))
          ? Reflect.get(ids, "ids")
          : [];
      return Promise.resolve(html(`<html><body>SHEET ${String(named)}</body></html>`));
    }
    if (url.includes("/label?lang=")) {
      if (labelAnswer !== null) return Promise.resolve(labelAnswer());
      return Promise.resolve(html(`<html><body>LABEL ${url.split("/products/")[1] ?? ""}</body></html>`));
    }
    if (init?.method === "PUT") {
      if (updateAnswer !== null) return Promise.resolve(updateAnswer());
      const id = Number(url.slice(url.lastIndexOf("/") + 1));
      const sent: Record<string, unknown> = JSON.parse(String(init.body));
      const before = rows.find((r) => r.id === id);
      if (before === undefined) {
        return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
      }
      const after: ProductDto = {
        ...before,
        name: typeof sent.name === "string" ? sent.name : before.name,
        selling_centimes:
          typeof sent.selling_centimes === "number" ? sent.selling_centimes : before.selling_centimes,
        wholesale_centimes:
          typeof sent.wholesale_centimes === "number" ? sent.wholesale_centimes : null,
        low_stock_at_milli:
          typeof sent.low_stock_at_milli === "number" ? sent.low_stock_at_milli : 0,
        rate_bps: typeof sent.rate_bps === "number" ? sent.rate_bps : before.rate_bps,
        active: sent.active === true,
      };
      rows = rows.map((r) => (r.id === id ? after : r));
      return Promise.resolve(json(200, after));
    }
    if (init?.method === "POST") {
      if (createAnswer !== null) return Promise.resolve(createAnswer());
      const sent: Record<string, unknown> = JSON.parse(String(init.body));
      const made: ProductDto = {
        ...product,
        id: rows.length + 2,
        name: typeof sent.name === "string" ? sent.name : product.name,
        selling_centimes:
          typeof sent.selling_centimes === "number" ? sent.selling_centimes : 0,
      };
      rows = [...rows, made];
      return Promise.resolve(json(201, made));
    }
    if (url.endsWith("/categories")) return Promise.resolve(json(200, categories));
    return Promise.resolve(json(200, rows));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

async function openTheForm(user: ReturnType<typeof userEvent.setup>) {
  await screen.findByText("Aucun produit pour le moment.");
  await user.click(screen.getByRole("button", { name: "Ajouter un produit" }));
  await screen.findByLabelText("Catégorie");
}

describe("the products list", () => {
  test("renders what the API returned", async () => {
    rows = [product];
    mount();
    expect(await screen.findByText("Huile Elio 5L")).toBeInTheDocument();
    const row = screen.getByText("Huile Elio 5L").closest("tr");
    expect(row).not.toBeNull();
    if (row !== null) {
      // 920 centimes is 9,20 DA, formatted from the integer, never a float.
      expect(within(row).getByText("9,20")).toBeInTheDocument();
      expect(within(row).getByText("24")).toBeInTheDocument();
      expect(within(row).getByText("2000010000017")).toBeInTheDocument();
    }
  });

  test("says so when there is nothing yet", async () => {
    mount();
    expect(await screen.findByText("Aucun produit pour le moment.")).toBeInTheDocument();
  });

  test("shows a translated message when the server cannot be reached", async () => {
    fetchMock.mockRejectedValue(new TypeError("fetch failed"));
    mount();
    expect(await screen.findByRole("alert")).toHaveTextContent("Serveur injoignable.");
  });
});

describe("the add form", () => {
  test("posts the typed values as integer centimes and shows the new row", async () => {
    const user = userEvent.setup();
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Sucre Cristal 1kg");
    await user.type(screen.getByLabelText("Prix de vente"), "1,10");
    await user.type(screen.getByLabelText("Quantité en stock"), "60");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    await waitFor(() => expect(posted()).toBe(true));
    const sent = sentBody();
    expect(sent.selling_centimes).toBe(110);
    expect(sent.qty_on_hand_milli).toBe(60_000);
    expect(sent.name).toBe("Sucre Cristal 1kg");
    expect(sent.barcode).toBeNull();

    expect(await screen.findByText("Sucre Cristal 1kg")).toBeInTheDocument();
  });

  test("offers the shop's categories and posts the one that was chosen", async () => {
    // category_id was hardcoded to 1, so a shop with more than one category
    // could only ever file a product under the first.
    const user = userEvent.setup();
    categories = [general, alimentaire];
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Farine");
    await user.type(screen.getByLabelText("Prix de vente"), "10");
    await user.selectOptions(screen.getByLabelText("Catégorie"), "2");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(sentBody().category_id).toBe(2);
  });

  test("posts 900 when 9 % is chosen", async () => {
    // rate_bps was hardcoded to null, so every product came out at 19 %.
    const user = userEvent.setup();
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Farine");
    await user.type(screen.getByLabelText("Prix de vente"), "10");
    await user.selectOptions(screen.getByLabelText("TVA"), "900");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(sentBody().rate_bps).toBe(900);
  });

  test("a category rate the fixed list lacks is shown as what it will post", async () => {
    // The migration allows any rate from 0 to 10 000 bps on a category; a
    // 700 bps one used to display "19 %" while the form posted 700.
    const user = userEvent.setup();
    categories = [{ ...general, default_rate_bps: 700 }];
    mount();
    await openTheForm(user);

    const rate = screen.getByLabelText("TVA");
    expect(rate).toHaveValue("700");
    expect(screen.getByRole("option", { name: "7 %" })).toBeInTheDocument();

    await user.type(screen.getByLabelText("Nom"), "Farine");
    await user.type(screen.getByLabelText("Prix de vente"), "10");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(sentBody().rate_bps).toBe(700);
  });

  test("the rate follows the category that was chosen", async () => {
    const user = userEvent.setup();
    categories = [general, alimentaire];
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Farine");
    await user.type(screen.getByLabelText("Prix de vente"), "10");
    await user.selectOptions(screen.getByLabelText("Catégorie"), "2");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(sentBody().rate_bps).toBe(900);
  });

  test("when the categories cannot be loaded the form stays closed and nothing is posted", async () => {
    const user = userEvent.setup();
    fetchMock.mockImplementation((input: unknown) =>
      Promise.resolve(
        String(input).endsWith("/categories")
          ? json(500, { error: { code: "storage", message: "x" } })
          : json(200, rows),
      ),
    );
    mount();
    await screen.findByText("Aucun produit pour le moment.");
    await user.click(screen.getByRole("button", { name: "Ajouter un produit" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Erreur d'enregistrement.");
    expect(screen.queryByLabelText("Nom")).not.toBeInTheDocument();
    expect(posted()).toBe(false);
  });

  test("with no category at all the product posts no category and the 19 % fallback", async () => {
    const user = userEvent.setup();
    categories = [];
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Sans catégorie");
    await user.type(screen.getByLabelText("Prix de vente"), "10");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(sentBody().category_id).toBeNull();
    expect(sentBody().rate_bps).toBe(1900);
  });

  test("refuses a cost that is not a number instead of storing zero", async () => {
    // parseAmountToCentimes(value.cost) ?? 0 turned "12 DA" into a cost of
    // nothing and saved it, so the shop's margin was quietly wrong.
    const user = userEvent.setup();
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Sucre Cristal 1kg");
    await user.type(screen.getByLabelText("Prix de vente"), "1,10");
    await user.type(screen.getByLabelText("Prix d'achat"), "12 DA");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(await screen.findByText("Prix d'achat invalide.")).toBeInTheDocument();
    expect(posted()).toBe(false);
  });

  test("refuses a stock that is not a number instead of storing zero", async () => {
    const user = userEvent.setup();
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Sucre Cristal 1kg");
    await user.type(screen.getByLabelText("Prix de vente"), "1,10");
    await user.type(screen.getByLabelText("Quantité en stock"), "60 kg");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(await screen.findByText("Quantité invalide.")).toBeInTheDocument();
    expect(posted()).toBe(false);
  });

  test("leaves a blank cost and a blank stock meaning zero", async () => {
    const user = userEvent.setup();
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Sucre Cristal 1kg");
    await user.type(screen.getByLabelText("Prix de vente"), "1,10");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    await waitFor(() => expect(posted()).toBe(true));
    expect(sentBody().cost_centimes).toBe(0);
    expect(sentBody().qty_on_hand_milli).toBe(0);
  });

  test("shows the API's validation error, translated from its code", async () => {
    const user = userEvent.setup();
    createAnswer = () =>
      json(409, { error: { code: "duplicate_barcode", message: "already used" } });
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Doublon");
    await user.type(screen.getByLabelText("Prix de vente"), "10");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(await screen.findByText("Ce code-barres est déjà utilisé.")).toBeInTheDocument();
  });

  test("renders the money code the API now sends for a bad rate", async () => {
    // The API returns the core's own code, so `money` reaches the UI where
    // `validation` used to. error_money is the key that has to render.
    const user = userEvent.setup();
    createAnswer = () =>
      json(422, { error: { code: "money", message: "rate above 10 000 basis points" } });
    mount();
    await openTheForm(user);

    await user.type(screen.getByLabelText("Nom"), "Taux impossible");
    await user.type(screen.getByLabelText("Prix de vente"), "10");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(await screen.findByText("Montant ou taux invalide.")).toBeInTheDocument();
  });

  test("refuses an empty name before it reaches the network", async () => {
    const user = userEvent.setup();
    mount();
    await openTheForm(user);

    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(await screen.findByText("Le nom est obligatoire.")).toBeInTheDocument();
    expect(posted()).toBe(false);
  });
});

describe("the table", () => {
  test("shows the stored rate and marks a product that is not for sale", async () => {
    rows = [product, { ...product, id: 2, name: "Ancien", rate_bps: 900, active: false }];
    mount();
    const first = (await screen.findByText("Huile Elio 5L")).closest("tr");
    const second = screen.getByText("Ancien").closest("tr");
    if (first === null || second === null) throw new Error("no rows");
    expect(within(first).getByText("19 %")).toBeInTheDocument();
    expect(within(first).queryByText("Inactif")).not.toBeInTheDocument();
    expect(within(second).getByText("9 %")).toBeInTheDocument();
    expect(within(second).getByText("Inactif")).toBeInTheDocument();
  });
});

describe("the edit form", () => {
  async function openTheEdit(user: ReturnType<typeof userEvent.setup>) {
    rows = [{ ...product, wholesale_centimes: 850, low_stock_at_milli: 10_000 }];
    mount();
    await screen.findByText("Huile Elio 5L");
    await user.click(screen.getByRole("button", { name: "Modifier Huile Elio 5L" }));
    await screen.findByLabelText("Catégorie");
  }

  test("opens with every field of the product filled in", async () => {
    const user = userEvent.setup();
    await openTheEdit(user);
    expect(screen.getByLabelText("Nom")).toHaveValue("Huile Elio 5L");
    // The barcode label carries its hint too, so an exact match never finds it.
    expect(screen.getByLabelText(/^Code-barres/)).toHaveValue("2000010000017");
    expect(screen.getByLabelText("Prix de vente")).toHaveValue("9,20");
    expect(screen.getByLabelText("Prix d'achat")).toHaveValue("8,20");
    expect(screen.getByLabelText("Prix de gros")).toHaveValue("8,50");
    expect(screen.getByLabelText("Quantité en stock")).toHaveValue("24");
    expect(screen.getByLabelText("Alerte stock bas à")).toHaveValue("10");
    expect(screen.getByRole("combobox", { name: "TVA" })).toHaveValue("1900");
    expect(screen.getByRole("combobox", { name: "Unité de mesure" })).toHaveValue("piece");
    expect(screen.getByLabelText("En vente")).toBeChecked();
  });

  test("puts the whole product to its own path and shows the new values", async () => {
    const user = userEvent.setup();
    await openTheEdit(user);
    const price = screen.getByLabelText("Prix de vente");
    await user.clear(price);
    await user.type(price, "9,90");
    await user.selectOptions(screen.getByRole("combobox", { name: "TVA" }), "900");
    await user.click(screen.getByLabelText("En vente"));
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    await waitFor(() => expect(initOf("PUT")).toBeDefined());
    const { url, body } = putRequest();
    expect(url.endsWith("/products/1")).toBe(true);
    expect(body).toEqual({
      name: "Huile Elio 5L",
      barcode: "2000010000017",
      category_id: 1,
      unit: "piece",
      cost_centimes: 820,
      selling_centimes: 990,
      wholesale_centimes: 850,
      qty_on_hand_milli: 24_000,
      low_stock_at_milli: 10_000,
      rate_bps: 900,
      active: false,
    });
    expect(posted()).toBe(false);

    const row = (await screen.findByText("Huile Elio 5L")).closest("tr");
    if (row === null) throw new Error("no row");
    await waitFor(() => expect(within(row).getByText("9,90")).toBeInTheDocument());
    expect(within(row).getByText("9 %")).toBeInTheDocument();
    expect(within(row).getByText("Inactif")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Enregistrer" })).not.toBeInTheDocument();
  });

  test("can take the product out of its category, and says nothing about renumbering", async () => {
    const user = userEvent.setup();
    await openTheEdit(user);
    expect(screen.queryByText("Laisser vide pour numéroter automatiquement")).toBeNull();
    await user.selectOptions(screen.getByRole("combobox", { name: "Catégorie" }), "");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(initOf("PUT")).toBeDefined());
    expect(putRequest().body.category_id).toBeNull();
  });

  test("shows the stock but does not let the fiche change it", async () => {
    const user = userEvent.setup();
    await openTheEdit(user);
    const stock = screen.getByLabelText("Quantité en stock");
    expect(stock).toHaveValue("24");
    expect(stock).toHaveAttribute("readonly");
    await user.type(stock, "9");
    expect(stock).toHaveValue("24");
  });

  test("shows the API's refusal on the form and keeps it open", async () => {
    const user = userEvent.setup();
    updateAnswer = () =>
      json(409, { error: { code: "duplicate_barcode", message: "already used" } });
    await openTheEdit(user);
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));
    expect(await screen.findByText("Ce code-barres est déjà utilisé.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Enregistrer" })).toBeInTheDocument();
  });

  test("refuses a wholesale price that is not a number", async () => {
    const user = userEvent.setup();
    await openTheEdit(user);
    const wholesale = screen.getByLabelText("Prix de gros");
    await user.clear(wholesale);
    await user.type(wholesale, "abc");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));
    expect(await screen.findByText("Prix de gros invalide.")).toBeInTheDocument();
    expect(initOf("PUT")).toBeUndefined();
  });
});

describe("the add form, every spec field", () => {
  test("offers no category as a choice and posts null for it", async () => {
    const user = userEvent.setup();
    mount();
    await openTheForm(user);
    await user.type(screen.getByLabelText("Nom"), "Divers");
    await user.type(screen.getByLabelText("Prix de vente"), "5");
    await user.selectOptions(screen.getByRole("combobox", { name: "Catégorie" }), "");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(posted()).toBe(true));
    expect(sentBody().category_id).toBeNull();
    expect(sentBody().rate_bps).toBe(1900);
  });

  test("posts the wholesale price and the low stock threshold when typed", async () => {
    const user = userEvent.setup();
    mount();
    await openTheForm(user);
    await user.type(screen.getByLabelText("Nom"), "Farine 25kg");
    await user.type(screen.getByLabelText("Prix de vente"), "3200");
    await user.type(screen.getByLabelText("Prix de gros"), "3050,50");
    await user.type(screen.getByLabelText("Alerte stock bas à"), "5");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(posted()).toBe(true));
    expect(sentBody().wholesale_centimes).toBe(305_050);
    expect(sentBody().low_stock_at_milli).toBe(5_000);
    expect(sentBody().active).toBe(true);
  });

  test("a blank wholesale price is null, not zero", async () => {
    const user = userEvent.setup();
    mount();
    await openTheForm(user);
    await user.type(screen.getByLabelText("Nom"), "Farine 25kg");
    await user.type(screen.getByLabelText("Prix de vente"), "3200");
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(posted()).toBe(true));
    expect(sentBody().wholesale_centimes).toBeNull();
    expect(sentBody().low_stock_at_milli).toBe(0);
  });
});

describe("in Arabic", () => {
  test("the table's rate cell reads the fixed word, not a computed Latin percent sign", async () => {
    // rate_900 in ar.json is "9 ٪" (Arabic percent sign). The table cell
    // used to call the same rateLabel() as a category rate the fixed list
    // does not carry, which always builds a Latin "%": the row showed
    // "9 %" while the add form's own dropdown, right above it, showed
    // "9 ٪" for the same value.
    rows = [{ ...product, rate_bps: 900 }];
    mount("ar");
    const row = (await screen.findByText("Huile Elio 5L")).closest("tr");
    if (row === null) throw new Error("no row");
    expect(within(row).getByText(ar.rate_900)).toBeInTheDocument();
    expect(within(row).queryByText("9 %")).not.toBeInTheDocument();
  });

  test("a category rate outside the fixed list still uses the Arabic percent sign", async () => {
    categories = [{ ...general, default_rate_bps: 700 }];
    rows = [{ ...product, rate_bps: 700 }];
    mount("ar");
    const row = (await screen.findByText("Huile Elio 5L")).closest("tr");
    if (row === null) throw new Error("no row");
    expect(within(row).getByText(`7 ${ar.percent_sign}`)).toBeInTheDocument();
  });

  test("a fractional category rate uses the Arabic decimal separator", async () => {
    // rateLabel used to hardcode a French comma regardless of language,
    // so this happened to already read right in Arabic; pinned here so a
    // future change to decimal_separator alone still catches a regression.
    categories = [{ ...general, default_rate_bps: 750 }];
    rows = [{ ...product, rate_bps: 750 }];
    mount("ar");
    const row = (await screen.findByText("Huile Elio 5L")).closest("tr");
    if (row === null) throw new Error("no row");
    expect(
      within(row).getByText(`7${ar.decimal_separator}50 ${ar.percent_sign}`),
    ).toBeInTheDocument();
  });

  test("the barcode, price, rate and stock cells stay left to right", async () => {
    rows = [product];
    mount("ar");
    const row = (await screen.findByText("Huile Elio 5L")).closest("tr");
    if (row === null) throw new Error("no row");
    const cells = within(row).getAllByRole("cell");
    // tick, name, barcode, unit, price, rate, stock, edit: barcode (2),
    // price (4), rate (5) and stock (6) are the ones read left to right.
    expect(cells[2]).toHaveAttribute("dir", "ltr");
    expect(cells[4]).toHaveAttribute("dir", "ltr");
    expect(cells[5]).toHaveAttribute("dir", "ltr");
    expect(cells[6]).toHaveAttribute("dir", "ltr");
  });
});

describe("in English", () => {
  test("a fractional category rate uses the shop's comma, not an English period", async () => {
    // Ruling (coordinator review, 2026-09-09): numbers follow the
    // shop's own format, not the UI language. formatCentimes and
    // formatQty already read comma-decimal in every language
    // ("9,20", "24,5"); rateLabel's fallback for a rate the fixed list
    // does not carry has to match, so English reads "7,50 %" here too,
    // not "7.50 %".
    categories = [{ ...general, default_rate_bps: 750 }];
    rows = [{ ...product, rate_bps: 750 }];
    mount("en");
    const row = (await screen.findByText("Huile Elio 5L")).closest("tr");
    if (row === null) throw new Error("no row");
    expect(
      within(row).getByText(`7${en.decimal_separator}50 ${en.percent_sign}`),
    ).toBeInTheDocument();
    expect(within(row).queryByText("7.50 %")).not.toBeInTheDocument();
  });
});

describe("the labels", () => {
  test("the drawer prints the stored product's label and nothing before it is stored", async () => {
    rows = [product];
    const user = userEvent.setup();
    mount();
    await screen.findByText("Huile Elio 5L");

    // The add form has no product to print: a label is a picture of a
    // barcode and a product being typed has neither an id nor one.
    await user.click(screen.getByRole("button", { name: "Ajouter un produit" }));
    await screen.findByLabelText("Catégorie");
    expect(screen.queryByTestId("print-label")).toBeNull();
    // Two buttons read "Annuler" while the add form is open: the header's,
    // which closes it, and the form's own.
    await user.click(screen.getAllByRole("button", { name: "Annuler" })[0] ?? document.body);

    await user.click(screen.getByRole("button", { name: /Huile Elio 5L/ }));
    await screen.findByLabelText("Catégorie");
    await user.click(screen.getByTestId("print-label"));

    const frame = await screen.findByTestId("product-label");
    expect(frame.getAttribute("srcdoc")).toContain("LABEL 1");
    expect(frame).toHaveAttribute("sandbox", "");
    const asked = fetchMock.mock.calls.map((call) => String(call[0]));
    expect(asked.some((url) => url.includes("/products/1/label?lang=fr"))).toBe(true);
  });

  test("a product with no EAN-13 says so instead of showing an empty page", async () => {
    rows = [product];
    labelAnswer = () =>
      json(422, { error: { code: "validation", field: "barcode", message: "no bars" } });
    const user = userEvent.setup();
    mount();
    await screen.findByText("Huile Elio 5L");
    await user.click(screen.getByRole("button", { name: /Huile Elio 5L/ }));
    await screen.findByLabelText("Catégorie");
    await user.click(screen.getByTestId("print-label"));

    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_label_no_barcode);
    expect(screen.queryByTestId("product-label")).toBeNull();
  });

  test("the sheet is the ticked rows, and nothing is offered while none is ticked", async () => {
    const second: ProductDto = { ...product, id: 2, name: "Semoule 5 kg", barcode: "2000010000024" };
    rows = [product, second];
    const user = userEvent.setup();
    mount();
    await screen.findByText("Semoule 5 kg");
    expect(screen.getByTestId("print-selected-labels")).toBeDisabled();

    await user.click(screen.getByRole("checkbox", { name: /Huile Elio 5L/ }));
    await user.click(screen.getByRole("checkbox", { name: /Semoule 5 kg/ }));
    await user.click(screen.getByTestId("print-selected-labels"));

    const frame = await screen.findByTestId("product-label");
    expect(frame.getAttribute("srcdoc")).toContain("SHEET 1,2");
    const posted = fetchMock.mock.calls.find((call) => String(call[0]).includes("/labels/sheet"));
    expect(posted).toBeDefined();
    const init: unknown = posted?.[1];
    const body: unknown = isInit(init) ? JSON.parse(String(init.body)) : null;
    expect(body).toEqual({ ids: [1, 2] });
  });
});
