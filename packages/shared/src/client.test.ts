import { describe, expect, test } from "vitest";
import { ApiError, createClient, isApiErrorBody, isSale } from "./client";
import type { NewProductDto } from "./generated/NewProductDto";
import type { NewSaleDto } from "./generated/NewSaleDto";
import type { SaleDto } from "./generated/SaleDto";
import type { ProductDto } from "./generated/ProductDto";
import type { SettingsDto } from "./generated/SettingsDto";
import type { StoreDto } from "./generated/StoreDto";

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

function stub(status: number, body: unknown): typeof fetch {
  return async () =>
    new Response(body === undefined ? "" : JSON.stringify(body), {
      status,
      headers: { "content-type": "application/json" },
    });
}

describe("createClient", () => {
  test("lists products", async () => {
    const api = createClient("http://127.0.0.1:4317/", stub(200, [product]));
    await expect(api.listProducts()).resolves.toEqual([product]);
  });

  test("trims a trailing slash off the base url", () => {
    expect(createClient("http://127.0.0.1:4317//").baseUrl).toBe("http://127.0.0.1:4317");
  });

  test("turns the server's error body into a code the UI can translate", async () => {
    const api = createClient(
      "http://x",
      stub(409, { error: { code: "duplicate_barcode", message: "already used" } }),
    );
    await expect(api.listProducts()).rejects.toMatchObject({
      code: "duplicate_barcode",
      status: 409,
    });
  });

  test("a failure without the agreed shape still carries a code", async () => {
    const api = createClient("http://x", stub(502, "<html>bad gateway</html>"));
    await expect(api.listProducts()).rejects.toBeInstanceOf(ApiError);
    await expect(api.listProducts()).rejects.toMatchObject({ code: "unreachable" });
  });

  test("a response of the wrong shape is refused, never handed to the UI", async () => {
    const api = createClient("http://x", stub(200, [{ id: 1, name: "half a product" }]));
    await expect(api.listProducts()).rejects.toMatchObject({ code: "bad_response" });
  });

  test("a price past what a number carries exactly is refused, and so is a fraction", async () => {
    // JSON.parse has already rounded 2^53 + 1 by the time the guard sees it;
    // the guard refuses anything outside the safe-integer range or with a
    // fractional part, so a lossy amount never reaches a screen.
    for (const selling_centimes of [2 ** 53, 9.5, Number.NaN]) {
      const broken = { ...product, selling_centimes };
      const api = createClient("http://x", stub(200, [broken]));
      await expect(api.listProducts()).rejects.toMatchObject({ code: "bad_response" });
    }
  });

  test("every amount and quantity field is guarded, not only the selling price", async () => {
    const fields = [
      "cost_centimes",
      "selling_centimes",
      "wholesale_centimes",
      "qty_on_hand_milli",
      "low_stock_at_milli",
    ];
    for (const field of fields) {
      for (const bad of [2 ** 53, 9.5, "920"]) {
        const broken = { ...product, [field]: bad };
        const api = createClient("http://x", stub(200, [broken]));
        await expect(api.listProducts(), `${field} = ${String(bad)}`).rejects.toMatchObject({
          code: "bad_response",
        });
      }
    }
  });

  test("a unit the app does not know is refused", async () => {
    const api = createClient("http://x", stub(200, [{ ...product, unit: "carton" }]));
    await expect(api.listProducts()).rejects.toMatchObject({ code: "bad_response" });
  });

  test("a category list is guarded the same way", async () => {
    const category = { id: 1, shop_id: 1, name: "Alimentation", default_rate_bps: 1900 };
    await expect(
      createClient("http://x", stub(200, [category])).listCategories(),
    ).resolves.toEqual([category]);
    for (const broken of [
      { id: 1, shop_id: 1, name: "Alimentation" },
      { ...category, default_rate_bps: "19" },
      { ...category, name: 3 },
    ]) {
      const api = createClient("http://x", stub(200, [broken]));
      await expect(api.listCategories()).rejects.toMatchObject({ code: "bad_response" });
    }
  });

  test("a price that arrives as a string is refused", async () => {
    const broken = { ...product, selling_centimes: "920" };
    const api = createClient("http://x", stub(200, [broken]));
    await expect(api.listProducts()).rejects.toMatchObject({ code: "bad_response" });
  });

  test("an unreachable server is a code, not a raw network error", async () => {
    const api = createClient("http://x", async () => {
      throw new TypeError("fetch failed");
    });
    await expect(api.listProducts()).rejects.toMatchObject({ code: "unreachable" });
  });

  test("every request shows the launch token as a bearer, /health included", async () => {
    // The API refuses anything without it (crates/api, launch token), so a
    // client built with a token must send it on every call, GET and POST.
    const seen: Array<string | undefined> = [];
    const api = createClient("http://x", {
      token: "abc123",
      fetch: async (url, init) => {
        seen.push(new Headers(init?.headers).get("authorization") ?? undefined);
        return new Response(
          JSON.stringify(String(url).endsWith("/health") ? { status: "ok", shop_id: 1 } : []),
          { status: 200 },
        );
      },
    });
    await api.health();
    await api.listProducts();
    await api.listCategories();
    expect(seen).toEqual(["Bearer abc123", "Bearer abc123", "Bearer abc123"]);
  });

  test("a client built without a token sends no authorization header", async () => {
    let seen: string | null = "unset";
    const api = createClient("http://x", {
      fetch: async (_url, init) => {
        seen = new Headers(init?.headers).get("authorization");
        return new Response("[]", { status: 200 });
      },
    });
    await api.listProducts();
    expect(seen).toBeNull();
  });

  test("updating a product puts JSON to its own path and returns the row", async () => {
    let seenUrl = "";
    let seen: RequestInit | undefined;
    const api = createClient("http://x", async (url, init) => {
      seenUrl = String(url);
      seen = init;
      return new Response(JSON.stringify({ ...product, name: "renamed" }), { status: 200 });
    });
    const input = {
      name: "renamed",
      barcode: null,
      category_id: 1,
      unit: "piece",
      cost_centimes: 820,
      selling_centimes: 920,
      wholesale_centimes: null,
      qty_on_hand_milli: 24_000,
      low_stock_at_milli: 10_000,
      rate_bps: 1900,
      active: false,
    } satisfies NewProductDto;
    await expect(api.updateProduct(7, input)).resolves.toMatchObject({ name: "renamed" });
    expect(seenUrl).toBe("http://x/products/7");
    expect(seen?.method).toBe("PUT");
    expect(new Headers(seen?.headers).get("content-type")).toBe("application/json");
    expect(JSON.parse(String(seen?.body))).toEqual(input);
  });

  test("an update answered with the wrong shape is refused like a read", async () => {
    const api = createClient("http://x", stub(200, { id: 7 }));
    await expect(api.updateProduct(7, { ...product })).rejects.toMatchObject({
      code: "bad_response",
    });
  });

  test("creating a product posts JSON and returns the created row", async () => {
    let seen: RequestInit | undefined;
    const api = createClient("http://x", async (_url, init) => {
      seen = init;
      return new Response(JSON.stringify(product), { status: 201 });
    });
    await expect(
      api.createProduct({
        name: "Huile Elio 5L",
        barcode: null,
        category_id: 1,
        unit: "piece",
        cost_centimes: 820,
        selling_centimes: 920,
        wholesale_centimes: null,
        qty_on_hand_milli: 24_000,
        low_stock_at_milli: 10_000,
        rate_bps: null,
        active: true,
      }),
    ).resolves.toEqual(product);
    expect(seen?.method).toBe("POST");
  });
});

describe("settings", () => {
  const store: StoreDto = {
    name: "Superette El Baraka",
    rc: "16/00-1234567 B 20",
    nif: "000016001234567",
    nis: null,
    ai: null,
    address: "12 rue Didouche Mourad, Alger",
    phone: "0555 12 34 56",
  };
  const settings: SettingsDto = {
    store,
    regime: { regime: "reel", valid_from: "2026-01-01" },
    regime_planned: null,
  };

  test("reads the settings page and keeps a planned change", async () => {
    const planned: SettingsDto = {
      ...settings,
      regime_planned: { regime: "ifu", valid_from: "2027-01-01" },
    };
    const api = createClient("http://127.0.0.1:4317", stub(200, planned));
    await expect(api.getSettings()).resolves.toEqual(planned);
  });

  test("a settings answer with a régime the app does not know is refused", async () => {
    for (const bad of [
      { ...settings, regime: { regime: "forfait", valid_from: "2026-01-01" } },
      { ...settings, regime: { regime: "reel", valid_from: "2026-1-1" } },
      { ...settings, regime_planned: { regime: "ifu" } },
      { ...settings, store: { ...store, name: null } },
      { ...settings, store: { ...store, rc: 12 } },
    ]) {
      const api = createClient("http://127.0.0.1:4317", stub(200, bad));
      await expect(api.getSettings()).rejects.toMatchObject({ code: "bad_response" });
    }
  });

  test("updating the store puts the whole block and returns it", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(store), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.updateStore(store)).resolves.toEqual(store);
    expect(calls).toHaveLength(1);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/settings/store");
    expect(calls[0]?.init?.method).toBe("PUT");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual(store);
  });

  test("a régime change posts the day and returns the whole page", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(settings), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.changeRegime({ regime: "ifu", valid_from: "2027-01-01" })).resolves.toEqual(
      settings,
    );
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/settings/regime");
    expect(calls[0]?.init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual({
      regime: "ifu",
      valid_from: "2027-01-01",
    });
  });
});

describe("isApiErrorBody", () => {
  test("accepts the shape the API promises and nothing else", () => {
    expect(isApiErrorBody({ error: { code: "x", message: "y" } })).toBe(true);
    expect(isApiErrorBody({ error: { code: "x" } })).toBe(false);
    expect(isApiErrorBody({ code: "x", message: "y" })).toBe(false);
    expect(isApiErrorBody(null)).toBe(false);
    expect(isApiErrorBody("nope")).toBe(false);
  });
});

const sale: SaleDto = {
  id: 1,
  shop_id: 1,
  kind: "ticket",
  series: "doc_ticket",
  number: 1,
  issued_at: "2026-09-09 10:00:00",
  user_id: 1,
  regime: "reel",
  payment_mode: "cash",
  seller: {
    name: "Mon magasin",
    rc: null,
    nif: null,
    nis: null,
    ai: null,
    address: null,
    phone: null,
  },
  customer_id: null,
  totals: {
    total_ht_centimes: 22_000,
    discount_centimes: 0,
    subtotal_ht_centimes: 22_000,
    tva_centimes: 4_180,
    total_ttc_centimes: 26_180,
    stamp_centimes: 0,
    net_to_pay_centimes: 26_180,
  },
  tva: [{ rate_bps: 1900, base_centimes: 22_000, amount_centimes: 4_180 }],
  tendered_centimes: 30_000,
  change_centimes: 3_820,
  status: "issued",
  lines: [
    {
      id: 1,
      position: 0,
      product_id: 1,
      name: "Sucre",
      barcode: "2000010000017",
      qty_milli: 2_000,
      unit_price_centimes: 11_000,
      line_discount_centimes: 0,
      rate_bps: 1900,
      line_total_centimes: 22_000,
    },
  ],
};

describe("sales", () => {
  test("ringing up a basket posts it and narrows the answer", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(sale), {
        status: 201,
        headers: { "content-type": "application/json" },
      });
    };
    const basket: NewSaleDto = {
      lines: [
        {
          product_id: 1,
          qty_milli: 2_000,
          unit_price_centimes: null,
          line_discount_centimes: 0,
        },
      ],
      global_discount_centimes: 0,
      payment_mode: "cash",
      tendered_centimes: 30_000,
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.createSale(basket)).resolves.toEqual(sale);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/sales");
    expect(calls[0]?.init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual(basket);
  });

  test("a sale is read back by id and the list is one call", async () => {
    const fetchStub: typeof fetch = async (input) =>
      new Response(JSON.stringify(String(input).endsWith("/sales") ? [sale] : sale), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.getSale(1)).resolves.toEqual(sale);
    await expect(api.listSales()).resolves.toEqual([sale]);
  });

  test("a total the server could not send exactly is refused, never shown", async () => {
    // Centimes are safe below 2^53. A number past it came out of JSON.parse
    // already rounded, so printing it would print a wrong amount.
    const rounded = {
      ...sale,
      totals: { ...sale.totals, net_to_pay_centimes: Number.MAX_SAFE_INTEGER + 2 },
    };
    const fetchStub: typeof fetch = async () =>
      new Response(JSON.stringify(rounded), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.getSale(1)).rejects.toBeInstanceOf(ApiError);
  });

  test("isSale refuses a kind and a payment mode the API does not use", () => {
    expect(isSale(sale)).toBe(true);
    expect(isSale({ ...sale, kind: "recu" })).toBe(false);
    expect(isSale({ ...sale, payment_mode: "bitcoin" })).toBe(false);
    expect(isSale({ ...sale, status: "draft" })).toBe(false);
    expect(isSale({ ...sale, lines: [{ ...sale.lines[0], qty_milli: 1.5 }] })).toBe(false);
    expect(isSale(null)).toBe(false);
  });

  test("a refused sale surfaces the code the UI translates", async () => {
    const fetchStub: typeof fetch = async () =>
      new Response(JSON.stringify({ error: { code: "validation", message: "short" } }), {
        status: 422,
        headers: { "content-type": "application/json" },
      });
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(
      api.createSale({
        lines: [],
        global_discount_centimes: 0,
        payment_mode: "cash",
        tendered_centimes: null,
      }),
    ).rejects.toMatchObject({ code: "validation", status: 422 });
  });
});
