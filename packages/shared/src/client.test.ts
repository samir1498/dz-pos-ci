import { describe, expect, test } from "vitest";
import { ApiError, createClient, isApiErrorBody } from "./client";
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
