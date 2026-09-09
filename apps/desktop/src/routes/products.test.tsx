// The screen is checked for rendering and wiring, not for business rules:
// those are the API crate's tests (architecture.md, consequence of rule 2).

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { CategoryDto, ProductDto } from "@dzpos/shared";
import { I18nProvider } from "@/i18n";
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
  for (const call of fetchMock.mock.calls) {
    const init: unknown = call[1];
    if (isInit(init) && init.method === "POST") return init;
  }
  return undefined;
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

function mount() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang="fr">
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

beforeEach(() => {
  rows = [];
  categories = [general];
  createAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
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
