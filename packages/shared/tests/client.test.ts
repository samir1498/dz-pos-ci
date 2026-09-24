import { describe, expect, test } from "vitest";
import { ApiError, createClient, SESSION_HEADER } from "../src/client";
import type { BackupDto } from "../src/generated/BackupDto";
import type { CustomerDto } from "../src/generated/CustomerDto";
import type { CustomerLedgerDto } from "../src/generated/CustomerLedgerDto";
import type { CustomerPaymentsDto } from "../src/generated/CustomerPaymentsDto";
import type { CustomerWriteDto } from "../src/generated/CustomerWriteDto";
import type { PaymentDto } from "../src/generated/PaymentDto";
import type { DebtEntryDto } from "../src/generated/DebtEntryDto";
import type { BackupsDto } from "../src/generated/BackupsDto";
import type { NewProductDto } from "../src/generated/NewProductDto";
import type { NewSaleDto } from "../src/generated/NewSaleDto";
import type { RestoreDto } from "../src/generated/RestoreDto";
import type { SaleDto } from "../src/generated/SaleDto";
import type { SessionDto } from "../src/generated/SessionDto";
import type { ProductDto } from "../src/generated/ProductDto";
import type { ImportDryRunDto } from "../src/generated/ImportDryRunDto";
import type { LastStockRecountDto } from "../src/generated/LastStockRecountDto";
import type { SettingsDto } from "../src/generated/SettingsDto";
import type { StockDriftDto } from "../src/generated/StockDriftDto";
import type { StockRecountDto } from "../src/generated/StockRecountDto";
import type { StoreDto } from "../src/generated/StoreDto";

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
          JSON.stringify(
            String(url).endsWith("/health")
              ? { status: "ok", shop_id: 1, needs_first_setup: true }
              : [],
          ),
          { status: 200 },
        );
      },
    });
    await api.health();
    await api.listProducts();
    await api.listCategories();
    expect(seen).toEqual(["Bearer abc123", "Bearer abc123", "Bearer abc123"]);
  });

  test("a token given as a function is awaited and shown as a bearer on every request", async () => {
    // The desktop passes a function (apps/desktop/src/api.ts): the launch
    // token never sits in a variable of the client's own, only behind a
    // call that asks the Tauri side for it.
    let calls = 0;
    const seen: Array<string | undefined> = [];
    const api = createClient("http://x", {
      token: async () => {
        calls += 1;
        return "abc123";
      },
      fetch: async (_url, init) => {
        seen.push(new Headers(init?.headers).get("authorization") ?? undefined);
        return new Response("[]", { status: 200 });
      },
    });
    await api.listProducts();
    await api.listCategories();
    expect(seen).toEqual(["Bearer abc123", "Bearer abc123"]);
    expect(calls).toBe(2);
  });

  test("a token function answering undefined sends no authorization header", async () => {
    let seen: string | null = "unset";
    const api = createClient("http://x", {
      token: async () => undefined,
      fetch: async (_url, init) => {
        seen = new Headers(init?.headers).get("authorization");
        return new Response("[]", { status: 200 });
      },
    });
    await api.listProducts();
    expect(seen).toBeNull();
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
    // `product` is a `ProductDto` fixture, whose `cost_centimes` is nullable
    // on the wire (M4 T5 review, 2026-09-11: a cashier's own read gets
    // `null`); `NewProductDto`, what a write sends, still wants a number, so
    // the body this test sends fills the one field the two shapes disagree
    // on rather than pass on the fixture's own nullable answer.
    await expect(
      api.updateProduct(7, { ...product, cost_centimes: product.cost_centimes ?? 0 }),
    ).rejects.toMatchObject({
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

const sale: SaleDto = {
  id: 1,
  shop_id: 1,
  kind: "ticket",
  series: "doc_ticket",
  number: 1,
  printed_number: "TK-2026-000001",
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
  ref_document_id: null,
  buyer_name: null,
  balance: null,
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
  cancellation: null,
  cancel_effect: null,
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
      ref_line_id: null,
    },
  ],
  warning: null,
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
      idempotency_key: "test-key",
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
      customer_id: null,
      override: false,
      kind: "ticket",
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.createSale(basket)).resolves.toEqual(sale);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/sales");
    expect(calls[0]?.init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual(basket);
  });

  test("a warning the app does not know is refused, the way an unknown mode is", async () => {
    // `near_limit` is the only one there is. A screen that matches on the
    // union would fall through a second one silently, so the guard stops it
    // at the door instead.
    const stub: typeof fetch = async () =>
      new Response(JSON.stringify({ ...sale, warning: "over_limit" }), {
        status: 201,
        headers: { "content-type": "application/json" },
      });
    const api = createClient("http://127.0.0.1:4317", stub);
    await expect(
      api.createSale({
        idempotency_key: "test-key",
        lines: [],
        global_discount_centimes: 0,
        payment_mode: "credit",
        tendered_centimes: null,
        customer_id: 3,
        override: false,
        kind: "ticket",
      }),
    ).rejects.toMatchObject({ code: "bad_response" });
  });

  test("a credit refusal carries the balance after and the limit, other errors carry neither", async () => {
    const refusal: typeof fetch = async () =>
      new Response(
        JSON.stringify({
          error: {
            code: "credit_limit",
            message: "past the limit",
            balance_after_centimes: 550_000,
            credit_limit_centimes: 500_000,
          },
        }),
        { status: 422, headers: { "content-type": "application/json" } },
      );
    const api = createClient("http://127.0.0.1:4317", refusal);
    await expect(
      api.createSale({
        idempotency_key: "test-key",
        lines: [],
        global_discount_centimes: 0,
        payment_mode: "credit",
        tendered_centimes: null,
        customer_id: 3,
        override: false,
        kind: "ticket",
      }),
    ).rejects.toMatchObject({
      code: "credit_limit",
      status: 422,
      balanceAfterCentimes: 550_000,
      creditLimitCentimes: 500_000,
    });

    // Absent, not zero: a screen that read a missing amount as nothing would
    // tell a cashier the limit is 0,00 on every other refusal.
    const plain: typeof fetch = async () =>
      new Response(JSON.stringify({ error: { code: "validation", message: "no" } }), {
        status: 422,
        headers: { "content-type": "application/json" },
      });
    await expect(createClient("http://x", plain).getSale(1)).rejects.toMatchObject({
      code: "validation",
      balanceAfterCentimes: undefined,
      creditLimitCentimes: undefined,
    });
  });

  test("a sale is read back by id and the list is one call", async () => {
    const fetchStub: typeof fetch = async (input) =>
      new Response(JSON.stringify(String(input).includes("/sales?") || String(input).endsWith("/sales") ? [sale] : sale), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.getSale(1)).resolves.toEqual(sale);
    await expect(api.listSales()).resolves.toEqual([sale]);
  });

  test("the list asks for one kind only when it is given one", async () => {
    // No kind is every document the till issued, so a facture is reachable
    // once its print panel is closed; a kind narrows it to that series.
    const calls: string[] = [];
    const fetchStub: typeof fetch = async (input) => {
      calls.push(String(input));
      return new Response(JSON.stringify([sale]), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await api.listSales();
    await api.listSales("facture");
    await api.listSales("ticket");
    expect(calls).toEqual([
      "http://127.0.0.1:4317/sales",
      "http://127.0.0.1:4317/sales?kind=facture",
      "http://127.0.0.1:4317/sales?kind=ticket",
    ]);
  });

  test("a list with one row of the wrong shape is refused whole", async () => {
    const api = createClient("http://x", stub(200, [sale, { ...sale, kind: "reçu" }]));
    await expect(api.listSales("facture")).rejects.toMatchObject({ code: "bad_response" });
  });

  test("a sale with no printed number is refused rather than shown blank", async () => {
    // The number the paper carries comes from the server, so a body without
    // it would put an empty string where the cashier reads FA-2026-000001 back to
    // the customer. The guard stops it at the door.
    const withoutNumber: Record<string, unknown> = { ...sale };
    delete withoutNumber.printed_number;
    const api = createClient("http://x", stub(200, withoutNumber));
    await expect(api.getSale(1)).rejects.toMatchObject({ code: "bad_response" });
  });

  test("the cancel effect is taken shape by shape and its amount is not read off the others", async () => {
    // The amount belongs to one of the three shapes. A guard that only looked
    // for the word would let an amount through on `stock_back`, and the
    // confirm would show a figure the server never sent.
    for (const effect of [
      { effect: "nothing_to_reverse" },
      { effect: "stock_back" },
      { effect: "stock_back_and_avoir", amount_centimes: 300_000 },
    ]) {
      const api = createClient("http://x", stub(200, { ...sale, cancel_effect: effect }));
      await expect(api.getSale(1)).resolves.toMatchObject({ cancel_effect: effect });
    }

    for (const bad of [
      // The one that carries an amount, without it.
      { effect: "stock_back_and_avoir" },
      { effect: "stock_back_and_avoir", amount_centimes: 1.5 },
      // A word nothing matches on.
      { effect: "avoir" },
    ]) {
      const api = createClient("http://x", stub(200, { ...sale, cancel_effect: bad }));
      await expect(api.getSale(1)).rejects.toMatchObject({ code: "bad_response" });
    }
  });

  test("a ticket comes back as the page the core rendered, not as JSON", async () => {
    const page = '<!doctype html>\n<html lang="ar" dir="rtl"><body>تذكرة</body></html>\n';
    const calls: string[] = [];
    const fetchStub: typeof fetch = async (input) => {
      calls.push(String(input));
      return new Response(page, {
        status: 200,
        headers: { "content-type": "text/html; charset=utf-8" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", { fetch: fetchStub, token: "t" });
    await expect(api.getSaleTicket(7, "ar")).resolves.toBe(page);
    expect(calls[0]).toBe("http://127.0.0.1:4317/sales/7/ticket?lang=ar");
  });

  test("the facture asks for the sheet as well as the language", async () => {
    const page = '<!doctype html>\n<html lang="fr"><body>FACTURE</body></html>\n';
    const calls: string[] = [];
    const fetchStub: typeof fetch = async (input) => {
      calls.push(String(input));
      return new Response(page, {
        status: 200,
        headers: { "content-type": "text/html; charset=utf-8" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", { fetch: fetchStub, token: "t" });
    await expect(api.getSaleFacture(7, "fr", "a4")).resolves.toBe(page);
    await expect(api.getSaleFacture(7, "ar", "a5")).resolves.toBe(page);
    expect(calls).toEqual([
      "http://127.0.0.1:4317/sales/7/facture?lang=fr&paper=a4",
      "http://127.0.0.1:4317/sales/7/facture?lang=ar&paper=a5",
    ]);
  });

  test("a facture the id does not name surfaces the code and never the HTML", async () => {
    const api = createClient(
      "http://127.0.0.1:4317",
      stub(404, { error: { code: "not_found", message: "facture 7 does not exist in this shop" } }),
    );
    await expect(api.getSaleFacture(7, "fr", "a4")).rejects.toMatchObject({ code: "not_found" });
  });

  test("a facture the party blocks refuse carries the side and the missing ids", async () => {
    const api = createClient(
      "http://127.0.0.1:4317",
      stub(422, {
        error: {
          code: "party_ids",
          message: "the buyer block of a facture is missing rc, nis",
          party_side: "buyer",
          missing_ids: ["rc", "nis"],
        },
      }),
    );
    await expect(
      api.createSale({
        idempotency_key: "test-key",
        lines: [{ product_id: 1, qty_milli: 1_000, unit_price_centimes: null, line_discount_centimes: 0 }],
        global_discount_centimes: 0,
        payment_mode: "cash",
        tendered_centimes: 200_000,
        customer_id: 4,
        override: false,
        kind: "facture",
      }),
    ).rejects.toMatchObject({
      code: "party_ids",
      partySide: "buyer",
      missingIds: ["rc", "nis"],
    });
  });

  test("a ticket the server refused surfaces the code and never the HTML", async () => {
    const api = createClient(
      "http://127.0.0.1:4317",
      stub(404, { error: { code: "not_found", message: "document 7 does not exist in this shop" } }),
    );
    await expect(api.getSaleTicket(7, "fr")).rejects.toMatchObject({ code: "not_found" });
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

  test("a refused sale surfaces the code the UI translates", async () => {
    const fetchStub: typeof fetch = async () =>
      new Response(JSON.stringify({ error: { code: "validation", message: "short" } }), {
        status: 422,
        headers: { "content-type": "application/json" },
      });
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(
      api.createSale({
        idempotency_key: "test-key",
        lines: [],
        global_discount_centimes: 0,
        payment_mode: "cash",
        tendered_centimes: null,
        customer_id: null,
        override: false,
        kind: "ticket",
      }),
    ).rejects.toMatchObject({ code: "validation", status: 422 });
  });
});

const customer: CustomerDto = {
  id: 3,
  shop_id: 1,
  name: "Entreprise Benali",
  party_kind: "company",
  phone: "0770 11 22 33",
  address: null,
  rc: null,
  nif: null,
  nis: null,
  ai: null,
  credit_limit_centimes: 5_000_000,
  warn_threshold_centimes: 4_000_000,
  notes: null,
  active: true,
  balance_centimes: 150_000,
};

const entry: DebtEntryDto = {
  id: 9,
  customer_id: 3,
  document_id: null,
  kind: "opening",
  debit_centimes: 150_000,
  credit_centimes: 0,
  balance_after_centimes: 150_000,
  user_id: 1,
  note: "solde de départ",
  created_at: "2026-09-09 10:00:00",
};

const ledger: CustomerLedgerDto = {
  customer_id: 3,
  balance_centimes: 150_000,
  entries: [entry],
};

const payment: PaymentDto = {
  ledger_id: 11,
  customer_id: 3,
  amount_centimes: 70_000,
  payment_mode: "cash",
  note: "acompte",
  balance_after_centimes: 80_000,
  allocations: [{ document_id: 4, printed_number: "FA-2026-000004", amount_centimes: 70_000 }],
  without_document_centimes: 0,
  created_at: "2026-09-12 16:30:00",
};

const payments: CustomerPaymentsDto = {
  customer_id: 3,
  balance_centimes: 80_000,
  payments: [payment],
};

const write: CustomerWriteDto = {
  name: "Entreprise Benali",
  party_kind: "company",
  phone: "0770 11 22 33",
  address: null,
  rc: null,
  nif: null,
  nis: null,
  ai: null,
  credit_limit_centimes: 5_000_000,
  warn_threshold_centimes: 4_000_000,
  notes: null,
  active: true,
  // The fiche stays open, so there is nothing to say why: the reason is
  // asked for only when a fiche with an account behind it is closed.
  close_reason: null,
};

/** Records what was asked for and answers `body`. */
function recorder(body: unknown, status = 200) {
  const calls: { url: string; init: RequestInit | undefined }[] = [];
  const fetchStub: typeof fetch = async (input, init) => {
    calls.push({ url: String(input), init });
    return new Response(JSON.stringify(body), {
      status,
      headers: { "content-type": "application/json" },
    });
  };
  return { calls, fetchStub };
}

describe("customers", () => {
  test("lists them, and a search travels as an encoded query", async () => {
    const { calls, fetchStub } = recorder([customer]);
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.listCustomers()).resolves.toEqual([customer]);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/customers");

    await api.listCustomers("benali & fils");
    expect(calls[1]?.url).toBe("http://127.0.0.1:4317/customers?q=benali+%26+fils");
  });

  test("a blank search asks for the whole list rather than an empty filter", async () => {
    const { calls, fetchStub } = recorder([customer]);
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await api.listCustomers("   ");
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/customers");
  });

  test("creates one with the opening debt in centimes", async () => {
    const { calls, fetchStub } = recorder(customer, 201);
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    const posted = { ...write, opening_debt_centimes: 150_000 };
    await expect(api.createCustomer(posted)).resolves.toEqual(customer);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/customers");
    expect(calls[0]?.init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual(posted);
  });

  test("updates one with the whole row, nulls included", async () => {
    const cleared = { ...write, rc: null, credit_limit_centimes: null };
    const { calls, fetchStub } = recorder({ ...customer, ...cleared });
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await api.updateCustomer(3, cleared);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/customers/3");
    expect(calls[0]?.init?.method).toBe("PUT");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual(cleared);
  });

  test("reads one fiche and its ledger", async () => {
    const one = recorder(customer);
    await expect(
      createClient("http://127.0.0.1:4317", one.fetchStub).getCustomer(3),
    ).resolves.toEqual(customer);
    expect(one.calls[0]?.url).toBe("http://127.0.0.1:4317/customers/3");

    const rows = recorder(ledger);
    await expect(
      createClient("http://127.0.0.1:4317", rows.fetchStub).customerLedger(3),
    ).resolves.toEqual(ledger);
    expect(rows.calls[0]?.url).toBe("http://127.0.0.1:4317/customers/3/ledger");
  });

  test("an adjustment posts signed centimes and answers the ledger again", async () => {
    const lowered: CustomerLedgerDto = { ...ledger, balance_centimes: 100_000 };
    const { calls, fetchStub } = recorder(lowered, 201);
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(
      api.adjustCustomerDebt(3, { amount_centimes: -50_000, note: "erreur de saisie" }),
    ).resolves.toEqual(lowered);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/customers/3/adjustments");
    expect(calls[0]?.init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual({
      amount_centimes: -50_000,
      note: "erreur de saisie",
    });
  });

  test("a fiche or a movement of the wrong shape is refused, never handed to the UI", async () => {
    for (const bad of [
      { ...customer, balance_centimes: 9.5 },
      { ...customer, party_kind: "societe" },
      { ...customer, credit_limit_centimes: "5000" },
      { ...customer, active: "yes" },
    ]) {
      const api = createClient("http://x", stub(200, [bad]));
      await expect(api.listCustomers()).rejects.toMatchObject({ code: "bad_response" });
    }
    for (const bad of [
      { ...ledger, balance_centimes: null },
      { ...ledger, entries: [{ ...entry, kind: "remise" }] },
      { ...ledger, entries: [{ ...entry, balance_after_centimes: 2 ** 53 }] },
    ]) {
      const api = createClient("http://x", stub(200, bad));
      await expect(api.customerLedger(3)).rejects.toMatchObject({ code: "bad_response" });
    }
  });

  test("the server's refusal keeps its code", async () => {
    const api = createClient(
      "http://x",
      stub(422, { error: { code: "validation", message: "amount ..." } }),
    );
    await expect(
      api.adjustCustomerDebt(3, { amount_centimes: 0, note: null }),
    ).rejects.toMatchObject({ code: "validation", status: 422 });
  });

  test("payments come back with what each one settled", async () => {
    const { calls, fetchStub } = recorder(payments);
    const api = createClient("http://127.0.0.1:4317", fetchStub);

    await expect(api.customerPayments(3)).resolves.toEqual(payments);
    await expect(
      api.payCustomer(3, { amount_centimes: 70_000, payment_mode: "cash", note: "acompte" }),
    ).resolves.toEqual(payments);

    expect(calls[0].url).toBe("http://127.0.0.1:4317/customers/3/payments");
    expect(calls[1].init?.method).toBe("POST");
    expect(JSON.parse(String(calls[1].init?.body))).toEqual({
      amount_centimes: 70_000,
      payment_mode: "cash",
      note: "acompte",
    });
  });

  test("a payment of the wrong shape is refused, never handed to the UI", async () => {
    for (const bad of [
      // The envelope itself.
      { ...payments, balance_centimes: 9.5 },
      { ...payments, customer_id: null },
      { ...payments, payments: {} },
      // One payment inside it. An amount JSON.parse had to round and a mode
      // the app does not know are both answers it cannot show.
      { ...payments, payments: [{ ...payment, amount_centimes: 2 ** 53 }] },
      { ...payments, payments: [{ ...payment, payment_mode: "cheque" }] },
      { ...payments, payments: [{ ...payment, balance_after_centimes: null }] },
      // And one allocation inside that.
      {
        ...payments,
        payments: [{ ...payment, allocations: [{ document_id: 4, amount_centimes: 1.5 }] }],
      },
      { ...payments, payments: [{ ...payment, allocations: [{ document_id: 4 }] }] },
    ]) {
      const api = createClient("http://x", stub(200, bad));
      await expect(api.customerPayments(3)).rejects.toMatchObject({ code: "bad_response" });
    }
  });

  test("a payment above the debt carries the field and what is owed; other errors carry neither", async () => {
    const api = createClient(
      "http://x",
      stub(422, {
        error: {
          code: "validation",
          message: "a payment is never more than what is owed",
          field: "amount_centimes",
          outstanding_centimes: 150_000,
        },
      }),
    );
    await expect(
      api.payCustomer(3, { amount_centimes: 200_000, payment_mode: "cash", note: null }),
    ).rejects.toMatchObject({
      code: "validation",
      status: 422,
      field: "amount_centimes",
      outstandingCentimes: 150_000,
    });

    // Absent, not zero: a form that read a missing figure as nothing would
    // tell a cashier the customer owes 0,00 on every other refusal.
    const plain = createClient(
      "http://x",
      stub(422, { error: { code: "validation", message: "no" } }),
    );
    await expect(
      plain.payCustomer(3, { amount_centimes: 1, payment_mode: "cash", note: null }),
    ).rejects.toMatchObject({
      code: "validation",
      field: undefined,
      outstandingCentimes: undefined,
      balanceAfterCentimes: undefined,
      creditLimitCentimes: undefined,
    });

    // A figure of the wrong type is not a refusal this client can read, so
    // the whole envelope is one it does not know rather than one it half
    // believes.
    for (const bad of [
      { code: "validation", message: "no", outstanding_centimes: "150000" },
      { code: "validation", message: "no", outstanding_centimes: 1.5 },
      { code: "validation", message: "no", field: 7 },
      { code: "credit_limit", message: "no", balance_after_centimes: "550000" },
      { code: "credit_limit", message: "no", credit_limit_centimes: null },
    ]) {
      const wrong = createClient("http://x", stub(422, { error: bad }));
      await expect(
        wrong.payCustomer(3, { amount_centimes: 1, payment_mode: "cash", note: null }),
      ).rejects.toMatchObject({ code: "unreachable", status: 422 });
    }
  });
});

describe("the exports, the product import and the labels", () => {
  const clean: ImportDryRunDto = {
    rows: [{ row: 2, name: "Café moulu 250 g", outcome: "created", field: null, reason: null }],
    accepted: 1,
    refused: 0,
  };

  function file(): Blob {
    return new Blob([new Uint8Array([0x50, 0x4b, 0x03, 0x04])]);
  }

  /** A workbook answer: the bytes, the media type and the name the server
   * chose. `disposition` null is the header a proxy stripped. */
  function workbook(disposition: string | null): typeof fetch {
    return async () =>
      new Response(new Blob([new Uint8Array([0x50, 0x4b, 0x03, 0x04])]), {
        status: 200,
        headers:
          disposition === null
            ? { "content-type": "application/octet-stream" }
            : { "content-type": "application/octet-stream", "content-disposition": disposition },
      });
  }

  test("an export asks its kind and its language, and the range only when it is given", async () => {
    const asked: string[] = [];
    const fetchStub: typeof fetch = async (input) => {
      asked.push(String(input));
      return new Response(new Blob([new Uint8Array([0x50, 0x4b, 0x03, 0x04])]), {
        status: 200,
        headers: { "content-disposition": 'attachment; filename="produits-2026-09-10.xlsx"' },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);

    const got = await api.exportWorkbook("products", "fr");
    expect(got.filename).toBe("produits-2026-09-10.xlsx");
    expect(await got.blob.size).toBe(4);
    expect(asked[0]).toBe("http://127.0.0.1:4317/export/products?lang=fr");

    await api.exportWorkbook("sales", "ar", { from: "2026-01-01", to: "2026-12-31" });
    expect(asked[1]).toContain("from=2026-01-01");
    expect(asked[1]).toContain("to=2026-12-31");

    // A blank day is not sent at all. `from=` empty is a date the server
    // cannot read, and it would answer 422 for a range nobody asked for.
    await api.exportWorkbook("sales", "fr", { from: "", to: "" });
    expect(asked[2]).toBe("http://127.0.0.1:4317/export/sales?lang=fr");
  });

  test("a workbook whose name the answer does not carry still saves under one", async () => {
    const api = createClient("http://127.0.0.1:4317", workbook(null));
    await expect(api.exportWorkbook("customers", "en")).resolves.toMatchObject({
      filename: "export.xlsx",
    });
  });

  test("a refused export leaves as the envelope, not as a file of the error text", async () => {
    const api = createClient(
      "http://127.0.0.1:4317",
      stub(422, { error: { code: "bad_request", message: "lang must be fr, en or ar" } }),
    );
    await expect(api.exportWorkbook("products", "fr")).rejects.toBeInstanceOf(ApiError);
    await expect(api.exportWorkbook("products", "fr")).rejects.toMatchObject({
      code: "bad_request",
    });
  });

  test("the dry run posts the file itself and reads the report back", async () => {
    const seen: { url: string; method: string | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      seen.push({ url: String(input), method: init?.method });
      return new Response(JSON.stringify(clean), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.dryRunProductImport(file())).resolves.toEqual(clean);
    expect(seen[0]).toEqual({
      url: "http://127.0.0.1:4317/import/products/dry-run",
      method: "POST",
    });
  });

  test("a report the schema refuses is a bad_response and never a half-read table", async () => {
    const api = createClient(
      "http://127.0.0.1:4317",
      stub(200, { rows: [{ row: 2, name: "Café", outcome: "maybe", field: null, reason: null }], accepted: 1, refused: 0 }),
    );
    await expect(api.dryRunProductImport(file())).rejects.toMatchObject({ code: "bad_response" });

    const counted = createClient(
      "http://127.0.0.1:4317",
      stub(200, { rows: [], accepted: "1", refused: 0 }),
    );
    await expect(counted.dryRunProductImport(file())).rejects.toMatchObject({
      code: "bad_response",
    });
  });

  test("applying reads the three counts, and a fractional one is refused", async () => {
    const api = createClient(
      "http://127.0.0.1:4317",
      stub(200, { created: 2, updated: 1, categories_created: 0 }),
    );
    await expect(api.applyProductImport(file())).resolves.toEqual({
      created: 2,
      updated: 1,
      categories_created: 0,
    });

    // The counts are exact integers: a 1.5 in a count is a server this
    // client does not understand, not a number to round.
    const fractional = createClient(
      "http://127.0.0.1:4317",
      stub(200, { created: 1.5, updated: 0, categories_created: 0 }),
    );
    await expect(fractional.applyProductImport(file())).rejects.toMatchObject({
      code: "bad_response",
    });
  });

  test("the template and the labels ask the routes the API mounts", async () => {
    const seen: { url: string; method: string | undefined; body: unknown }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      seen.push({ url: String(input), method: init?.method, body: init?.body });
      return new Response("<html>label</html>", {
        status: 200,
        headers: { "content-type": "text/html" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);

    await expect(api.getProductLabel(7, "ar")).resolves.toBe("<html>label</html>");
    expect(seen[0]?.url).toBe("http://127.0.0.1:4317/products/7/label?lang=ar");

    await api.getLabelSheet([3, 1], "fr");
    expect(seen[1]?.url).toBe("http://127.0.0.1:4317/labels/sheet?lang=fr");
    expect(seen[1]?.method).toBe("POST");
    // The order the caller named, kept: the sheet is the selection as it
    // was ticked, not a set the client sorted.
    expect(JSON.parse(String(seen[1]?.body))).toEqual({ ids: [3, 1] });
  });

  test("a label the server refuses raises the code the screen translates", async () => {
    const api = createClient(
      "http://127.0.0.1:4317",
      stub(422, { error: { code: "validation", field: "barcode", message: "no bars" } }),
    );
    await expect(api.getProductLabel(7, "fr")).rejects.toMatchObject({
      code: "validation",
      field: "barcode",
    });
  });
});

describe("the session on the wire (M4 T2)", () => {
  const signedIn: SessionDto = {
    me: { user_id: 3, name: "Karim", role: "cashier", permissions: ["sell"] },
    token: "6f".repeat(32),
    idle_minutes: 15,
  };

  /** A fetch that keeps the init of every call it was handed. */
  function watching(body: unknown): { fetch: typeof fetch; seen: RequestInit[] } {
    const seen: RequestInit[] = [];
    const fetchImpl: typeof fetch = async (_url, init) => {
      seen.push(init ?? {});
      return new Response(JSON.stringify(body), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    return { fetch: fetchImpl, seen };
  }

  test("a session the client holds travels on every call, beside the launch token", async () => {
    const { fetch: fetchImpl, seen } = watching([product]);
    const api = createClient("http://x", { fetch: fetchImpl, token: "launch", session: "abc" });
    await api.listProducts();
    const headers = new Headers(seen[0]?.headers);
    expect(headers.get(SESSION_HEADER)).toBe("abc");
    expect(headers.get("authorization")).toBe("Bearer launch");
    // The cookie is the browser's half of the same thing, and it only rides
    // along on a cross-origin call when the call asks for it.
    expect(seen[0]?.credentials).toBe("include");
  });

  test("with no session in hand the header is absent rather than empty", async () => {
    const { fetch: fetchImpl, seen } = watching([product]);
    const api = createClient("http://x", { fetch: fetchImpl, token: "launch" });
    await api.listProducts();
    expect(new Headers(seen[0]?.headers).has(SESSION_HEADER)).toBe(false);
  });

  test("setSession puts one on and null takes it off again", async () => {
    const { fetch: fetchImpl, seen } = watching([product]);
    const api = createClient("http://x", { fetch: fetchImpl });
    api.setSession("abc");
    await api.listProducts();
    api.setSession(null);
    await api.listProducts();
    expect(new Headers(seen[0]?.headers).get(SESSION_HEADER)).toBe("abc");
    expect(new Headers(seen[1]?.headers).has(SESSION_HEADER)).toBe(false);
  });

  test("signing in hands the token back and does not remember it by itself", async () => {
    // The sign-in answer first, a product list after it: one body for every
    // call would have the second call fail on the shape rather than on the
    // header, which is what this is about.
    const bodies: unknown[] = [signedIn, [product]];
    const seen: RequestInit[] = [];
    const api = createClient("http://x", {
      fetch: async (_url, init) => {
        seen.push(init ?? {});
        return new Response(JSON.stringify(bodies.shift()), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      },
    });
    await expect(api.login({ user_id: 3, pin: "1379" })).resolves.toEqual(signedIn);
    // The browser already has the session as an httpOnly cookie; a client
    // that also kept the token in a variable would undo that. The desktop,
    // which has no cookie, calls setSession with what it was handed (T4).
    await api.listProducts();
    expect(new Headers(seen[1]?.headers).has(SESSION_HEADER)).toBe(false);
  });

  test("signing out forgets the token even when the server refuses the call", async () => {
    const seen: RequestInit[] = [];
    const api = createClient("http://x", {
      fetch: async (_url, init) => {
        seen.push(init ?? {});
        return new Response(JSON.stringify({ error: { code: "boom", message: "no" } }), {
          status: 500,
        });
      },
      session: "abc",
    });
    await expect(api.logout()).rejects.toBeInstanceOf(ApiError);
    await expect(api.listProducts()).rejects.toBeInstanceOf(ApiError);
    expect(new Headers(seen[1]?.headers).has(SESSION_HEADER)).toBe(false);
  });
});
