// Contract: a document the till issued, the way the server sends it back.
// Every centime column is an exact integer, the kinds and the statuses are
// the words the Rust enums write, and the cancel effect is taken shape by
// shape so an amount never rides on the two shapes that carry none.

import { describe, expect, test } from "vitest";

import type { SaleDto } from "../../src/generated/SaleDto";
import {
  documentKindSchema,
  documentStatusSchema,
  paymentModeSchema,
  saleBalanceSchema,
  saleCancelEffectSchema,
  saleCancellationSchema,
  saleKindSchema,
  saleLineSchema,
  saleSchema,
  saleTotalsSchema,
  saleTvaSchema,
  saleWarningSchema,
} from "../../src/schemas/sale";

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

describe("documentKindSchema", () => {
  test("takes a kind the documents screen lists", () => {
    expect(documentKindSchema.parse("bon_de_livraison")).toBe("bon_de_livraison");
  });

  test("refuses a kind the API never writes", () => {
    expect(documentKindSchema.safeParse("recu").success).toBe(false);
  });

  test("carries the seven kinds the API writes and no eighth", () => {
    expect(documentKindSchema.options).toEqual([
      "ticket",
      "facture",
      "proforma",
      "bon_de_livraison",
      "avoir",
      "bon_de_reception",
      "quittance",
    ]);
  });
});

describe("saleKindSchema", () => {
  test("takes one of the three the till issues", () => {
    expect(saleKindSchema.parse("facture")).toBe("facture");
  });

  test("refuses a kind that exists but is not one the till issues", () => {
    expect(saleKindSchema.safeParse("avoir").success).toBe(false);
  });

  test("carries the three the till issues and no fourth", () => {
    expect(saleKindSchema.options).toEqual(["ticket", "facture", "proforma"]);
  });
});

describe("documentStatusSchema", () => {
  test("takes a status a document can be in", () => {
    expect(documentStatusSchema.parse("cancelled")).toBe("cancelled");
  });

  test("refuses a status the API has no column for", () => {
    expect(documentStatusSchema.safeParse("draft").success).toBe(false);
  });

  test("carries the two states a document can be in and no third", () => {
    expect(documentStatusSchema.options).toEqual(["issued", "cancelled"]);
  });
});

describe("paymentModeSchema", () => {
  test("takes a mode the till can be paid in", () => {
    expect(paymentModeSchema.parse("credit")).toBe("credit");
  });

  test("refuses a mode nothing settles in", () => {
    expect(paymentModeSchema.safeParse("bitcoin").success).toBe(false);
  });

  test("carries the three modes a sale is paid in and no fourth", () => {
    expect(paymentModeSchema.options).toEqual(["cash", "card", "credit"]);
  });
});

describe("saleWarningSchema", () => {
  test("takes the one warning a sale can carry", () => {
    expect(saleWarningSchema.parse("near_limit")).toBe("near_limit");
  });

  test("refuses a warning the core does not raise", () => {
    expect(saleWarningSchema.safeParse("over_limit").success).toBe(false);
  });

  test("carries the one warning the core raises and no second", () => {
    // A second one added to the Rust enum fails here and in warningKey's
    // exhaustive switch on the till, which is where a cashier would see it.
    expect(saleWarningSchema.options).toEqual(["near_limit"]);
  });
});

describe("saleLineSchema", () => {
  test("takes a line of the basket", () => {
    expect(saleLineSchema.parse(sale.lines[0])).toEqual(sale.lines[0]);
  });

  test("refuses a quantity JSON.parse had to round", () => {
    expect(saleLineSchema.safeParse({ ...sale.lines[0], qty_milli: 1.5 }).success).toBe(false);
  });
});

describe("saleTvaSchema", () => {
  test("takes one rate's base and amount", () => {
    expect(saleTvaSchema.parse(sale.tva[0])).toEqual(sale.tva[0]);
  });

  test("refuses a TVA amount that came back rounded", () => {
    expect(saleTvaSchema.safeParse({ ...sale.tva[0], amount_centimes: 41.8 }).success).toBe(false);
  });
});

describe("saleTotalsSchema", () => {
  test("takes every column of the totals table", () => {
    expect(saleTotalsSchema.parse(sale.totals)).toEqual(sale.totals);
  });

  test("refuses a total past the exact-integer bound", () => {
    const rounded = { ...sale.totals, net_to_pay_centimes: Number.MAX_SAFE_INTEGER + 2 };
    expect(saleTotalsSchema.safeParse(rounded).success).toBe(false);
  });
});

describe("saleBalanceSchema", () => {
  test("takes the three amounts together", () => {
    const balance = {
      old_balance_centimes: 100_000,
      remaining_debt_centimes: 26_180,
      total_debt_centimes: 126_180,
    };
    expect(saleBalanceSchema.parse(balance)).toEqual(balance);
  });

  test("refuses two of the three", () => {
    const partial = { old_balance_centimes: 100_000, total_debt_centimes: 126_180 };
    expect(saleBalanceSchema.safeParse(partial).success).toBe(false);
  });
});

describe("saleCancellationSchema", () => {
  test("takes when, by whom, why and with which avoir", () => {
    const cancellation = {
      cancelled_at: "2026-09-10 11:00:00",
      cancelled_by: 1,
      reason: "erreur de caisse",
      avoir_document_id: null,
    };
    expect(saleCancellationSchema.parse(cancellation)).toEqual(cancellation);
  });

  test("refuses a date with no reason beside it", () => {
    const half = { cancelled_at: "2026-09-10 11:00:00", cancelled_by: 1, avoir_document_id: null };
    expect(saleCancellationSchema.safeParse(half).success).toBe(false);
  });
});

describe("saleCancelEffectSchema", () => {
  test("takes each of the three shapes and the amount only on the one that has it", () => {
    expect(saleCancelEffectSchema.parse({ effect: "nothing_to_reverse" })).toEqual({
      effect: "nothing_to_reverse",
    });
    // The amount is dropped rather than passed on: a confirm that read it off
    // `stock_back` would show a figure the server never sent.
    expect(saleCancelEffectSchema.parse({ effect: "stock_back", amount_centimes: 300_000 })).toEqual(
      { effect: "stock_back" },
    );
    expect(
      saleCancelEffectSchema.parse({ effect: "stock_back_and_avoir", amount_centimes: 300_000 }),
    ).toEqual({ effect: "stock_back_and_avoir", amount_centimes: 300_000 });
  });

  test("refuses the shape that carries an amount without one, and a word nothing matches", () => {
    expect(saleCancelEffectSchema.safeParse({ effect: "stock_back_and_avoir" }).success).toBe(false);
    expect(saleCancelEffectSchema.safeParse({ effect: "avoir" }).success).toBe(false);
  });
});

describe("saleSchema", () => {
  test("takes the document the till got back", () => {
    expect(saleSchema.parse(sale)).toEqual(sale);
  });

  test("refuses a kind, a mode, a status and a quantity the API does not use", () => {
    expect(saleSchema.safeParse({ ...sale, kind: "recu" }).success).toBe(false);
    expect(saleSchema.safeParse({ ...sale, payment_mode: "bitcoin" }).success).toBe(false);
    expect(saleSchema.safeParse({ ...sale, status: "draft" }).success).toBe(false);
    const fractional = { ...sale, lines: [{ ...sale.lines[0], qty_milli: 1.5 }] };
    expect(saleSchema.safeParse(fractional).success).toBe(false);
    expect(saleSchema.safeParse(null).success).toBe(false);
  });

  test("refuses a document with no printed number, so none is shown blank", () => {
    const withoutNumber: Record<string, unknown> = { ...sale };
    delete withoutNumber.printed_number;
    expect(saleSchema.safeParse(withoutNumber).success).toBe(false);
  });
});
