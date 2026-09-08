// The screen is checked for rendering and wiring, not for business rules:
// those are the API crate's tests (architecture.md, consequence of rule 2).

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ProductDto } from "@dzpos/shared";
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

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
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

beforeEach(() => {
  fetchMock = vi.fn();
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the products list", () => {
  test("renders what the API returned", async () => {
    fetchMock.mockResolvedValue(json(200, [product]));
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
    fetchMock.mockResolvedValue(json(200, []));
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
    fetchMock.mockResolvedValueOnce(json(200, []));
    mount();
    await screen.findByText("Aucun produit pour le moment.");

    await user.click(screen.getByRole("button", { name: "Ajouter un produit" }));
    await user.type(screen.getByLabelText("Nom"), "Sucre Cristal 1kg");
    await user.type(screen.getByLabelText("Prix de vente"), "1,10");
    await user.type(screen.getByLabelText("Quantité en stock"), "60");

    const created = { ...product, id: 2, name: "Sucre Cristal 1kg", selling_centimes: 110 };
    fetchMock.mockResolvedValueOnce(json(201, created));
    fetchMock.mockResolvedValueOnce(json(200, [created]));

    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    const post = fetchMock.mock.calls[1];
    expect(post[1].method).toBe("POST");
    const sent = JSON.parse(post[1].body);
    expect(sent.selling_centimes).toBe(110);
    expect(sent.qty_on_hand_milli).toBe(60_000);
    expect(sent.name).toBe("Sucre Cristal 1kg");
    expect(sent.barcode).toBeNull();

    expect(await screen.findByText("Sucre Cristal 1kg")).toBeInTheDocument();
  });

  test("shows the API's validation error, translated from its code", async () => {
    const user = userEvent.setup();
    fetchMock.mockResolvedValueOnce(json(200, []));
    mount();
    await screen.findByText("Aucun produit pour le moment.");

    await user.click(screen.getByRole("button", { name: "Ajouter un produit" }));
    await user.type(screen.getByLabelText("Nom"), "Doublon");
    await user.type(screen.getByLabelText("Prix de vente"), "10");

    fetchMock.mockResolvedValueOnce(
      json(409, { error: { code: "duplicate_barcode", message: "already used" } }),
    );
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(await screen.findByText("Ce code-barres est déjà utilisé.")).toBeInTheDocument();
  });

  test("renders the money code the API now sends for a bad rate", async () => {
    // The API returns the core's own code, so `money` reaches the UI where
    // `validation` used to. error_money is the key that has to render.
    const user = userEvent.setup();
    fetchMock.mockResolvedValueOnce(json(200, []));
    mount();
    await screen.findByText("Aucun produit pour le moment.");

    await user.click(screen.getByRole("button", { name: "Ajouter un produit" }));
    await user.type(screen.getByLabelText("Nom"), "Taux impossible");
    await user.type(screen.getByLabelText("Prix de vente"), "10");

    fetchMock.mockResolvedValueOnce(
      json(422, { error: { code: "money", message: "rate above 10 000 basis points" } }),
    );
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(await screen.findByText("Montant ou taux invalide.")).toBeInTheDocument();
  });

  test("refuses an empty name before it reaches the network", async () => {
    const user = userEvent.setup();
    fetchMock.mockResolvedValueOnce(json(200, []));
    mount();
    await screen.findByText("Aucun produit pour le moment.");

    await user.click(screen.getByRole("button", { name: "Ajouter un produit" }));
    await user.click(screen.getByRole("button", { name: "Enregistrer" }));

    expect(await screen.findByText("Le nom est obligatoire.")).toBeInTheDocument();
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});
