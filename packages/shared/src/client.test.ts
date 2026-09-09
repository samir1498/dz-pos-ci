import { describe, expect, test } from "vitest";
import { ApiError, createClient, isApiErrorBody } from "./client";
import type { NewProductDto } from "./generated/NewProductDto";
import type { ProductDto } from "./generated/ProductDto";

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

describe("isApiErrorBody", () => {
  test("accepts the shape the API promises and nothing else", () => {
    expect(isApiErrorBody({ error: { code: "x", message: "y" } })).toBe(true);
    expect(isApiErrorBody({ error: { code: "x" } })).toBe(false);
    expect(isApiErrorBody({ code: "x", message: "y" })).toBe(false);
    expect(isApiErrorBody(null)).toBe(false);
    expect(isApiErrorBody("nope")).toBe(false);
  });
});
