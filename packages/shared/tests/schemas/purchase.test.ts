// Contract: an order, its lines, the deliveries against it and the two forms
// that write them. One good payload and one bad one per schema, and every
// amount an exact integer, because a landed cost read wrong is a margin read
// wrong for as long as the stock sits on the shelf.

import { describe, expect, test } from "vitest";

import type { NewPurchaseDto } from "../../src/generated/NewPurchaseDto";
import type { PurchaseDetailDto } from "../../src/generated/PurchaseDetailDto";
import type { PurchaseDto } from "../../src/generated/PurchaseDto";
import {
  closeOrderSchema,
  newPurchaseSchema,
  newReceiptSchema,
  purchaseDetailSchema,
  purchaseSchema,
  purchaseStatusSchema,
} from "../../src/schemas/purchase";

const purchase: PurchaseDto = {
  id: 3,
  shop_id: 1,
  supplier_id: 7,
  supplier_document_number: "BL-77",
  purchase_date: "2026-09-10",
  due_date: "2026-10-10",
  transport_centimes: 50_000,
  extra_costs_centimes: 12_500,
  // 500,00 and 125,00 of other costs; the API answers the 625,00 and the
  // screens print it.
  extras_centimes: 62_500,
  status: "partially_received",
  user_id: 1,
  note: null,
  created_at: "2026-09-10 09:00:00",
};

const detail: PurchaseDetailDto = {
  purchase,
  lines: [
    {
      id: 11,
      product_id: 4,
      qty_ordered_milli: 10_000,
      unit_cost_centimes: 20_000,
      landed_unit_cost_centimes: 26_250,
      qty_received_milli: 4_000,
      qty_returned_milli: 1_000,
    },
  ],
  receipts: [
    {
      id: 2,
      series: "reception:2026",
      number: 1,
      received_at: "2026-09-10 09:05:00",
      user_id: 1,
      note: "quatre sacs",
      lines: [{ purchase_line_id: 11, qty_milli: 4_000 }],
    },
  ],
};

const order: NewPurchaseDto = {
  supplier_id: 7,
  supplier_document_number: null,
  purchase_date: "2026-09-10",
  due_date: null,
  transport_centimes: 50_000,
  extra_costs_centimes: 0,
  note: null,
  lines: [{ product_id: 4, qty_ordered_milli: 10_000, unit_cost_centimes: 20_000 }],
  paid_now: { amount_centimes: 100_000, payment_mode: "cash" },
  receive_now: true,
};

describe("purchase status", () => {
  test("the five states of an order cross", () => {
    for (const state of [
      "ordered",
      "partially_received",
      "received",
      "cancelled",
      "closed_short",
    ]) {
      expect(purchaseStatusSchema.safeParse(state).success).toBe(true);
    }
  });

  test("a state nobody wrote is refused", () => {
    expect(purchaseStatusSchema.safeParse("delivered").success).toBe(false);
  });
});

describe("purchase", () => {
  test("an order as the list reads it", () => {
    expect(purchaseSchema.parse(purchase)).toEqual(purchase);
  });

  test("a day spelled another way is refused", () => {
    expect(purchaseSchema.safeParse({ ...purchase, purchase_date: "10/09/2026" }).success).toBe(
      false,
    );
  });

  test("a cost JSON.parse had to round is refused", () => {
    expect(
      purchaseSchema.safeParse({ ...purchase, transport_centimes: 50_000.5 }).success,
    ).toBe(false);
    expect(purchaseSchema.safeParse({ ...purchase, extras_centimes: 62_500.5 }).success).toBe(
      false,
    );
  });

  test("an answer without the extras the core added is refused", () => {
    const without: Record<string, unknown> = { ...purchase };
    delete without.extras_centimes;
    expect(purchaseSchema.safeParse(without).success).toBe(false);
  });
});

describe("purchase detail", () => {
  test("the order, its lines and its deliveries", () => {
    expect(purchaseDetailSchema.parse(detail)).toEqual(detail);
  });

  test("a receipt line of a fractional quantity is refused", () => {
    const broken = {
      ...detail,
      receipts: [
        { ...detail.receipts[0], lines: [{ purchase_line_id: 11, qty_milli: 4_000.5 }] },
      ],
    };
    expect(purchaseDetailSchema.safeParse(broken).success).toBe(false);
  });
});

describe("the order form", () => {
  test("an order with a line and money handed over", () => {
    expect(newPurchaseSchema.parse(order)).toEqual(order);
  });

  test("an order with no line never leaves the screen", () => {
    expect(newPurchaseSchema.safeParse({ ...order, lines: [] }).success).toBe(false);
  });

  test("a line ordering nothing is refused", () => {
    const broken = {
      ...order,
      lines: [{ product_id: 4, qty_ordered_milli: 0, unit_cost_centimes: 20_000 }],
    };
    expect(newPurchaseSchema.safeParse(broken).success).toBe(false);
  });

  test("a negative transport cost is refused", () => {
    expect(newPurchaseSchema.safeParse({ ...order, transport_centimes: -1 }).success).toBe(false);
  });

  test("money handed over is more than nothing", () => {
    const broken = { ...order, paid_now: { amount_centimes: 0, payment_mode: "cash" } };
    expect(newPurchaseSchema.safeParse(broken).success).toBe(false);
  });

  test("a payment mode the ledger does not know is refused", () => {
    const broken = { ...order, paid_now: { amount_centimes: 100, payment_mode: "credit" } };
    expect(newPurchaseSchema.safeParse(broken).success).toBe(false);
  });
});

describe("the delivery and return form", () => {
  test("a delivery naming one line", () => {
    const receipt = { lines: [{ purchase_line_id: 11, qty_milli: 4_000 }], note: null };
    expect(newReceiptSchema.parse(receipt)).toEqual(receipt);
  });

  test("a delivery with no line is refused", () => {
    expect(newReceiptSchema.safeParse({ lines: [], note: null }).success).toBe(false);
  });

  test("a line taking in nothing is refused", () => {
    const broken = { lines: [{ purchase_line_id: 11, qty_milli: 0 }], note: null };
    expect(newReceiptSchema.safeParse(broken).success).toBe(false);
  });
});

describe("closing an order", () => {
  test("a reason", () => {
    expect(closeOrderSchema.parse({ reason: "le fournisseur ne livre plus" })).toEqual({
      reason: "le fournisseur ne livre plus",
    });
  });

  test("a blank reason is no reason", () => {
    expect(closeOrderSchema.safeParse({ reason: "" }).success).toBe(false);
  });
});
