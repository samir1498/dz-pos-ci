// Contract: the supplier fiche and the ledger behind what the shop owes it.
// One good payload and one bad one per schema, and every amount an exact
// integer, because a fraction of a centime is a figure a shop would read to a
// supplier over the phone.

import { describe, expect, test } from "vitest";

import type { SupplierDto } from "../../src/generated/SupplierDto";
import type { SupplierEntryDto } from "../../src/generated/SupplierEntryDto";
import type { SupplierStatementDto } from "../../src/generated/SupplierStatementDto";
import {
  supplierAllocationSchema,
  supplierDebtKindSchema,
  supplierEntrySchema,
  supplierLedgerSchema,
  supplierSchema,
  supplierStatementSchema,
} from "../../src/schemas/supplier";

const supplier: SupplierDto = {
  id: 7,
  shop_id: 1,
  name: "Sarl Amrani",
  phone: "0550112233",
  address: null,
  rc: "16/00-7654321 B 22",
  nif: null,
  nis: null,
  ai: null,
  notes: "livre le mardi",
  active: true,
  balance_centimes: 250_000,
};

const payment: SupplierEntryDto = {
  id: 12,
  supplier_id: 7,
  purchase_id: null,
  kind: "payment",
  debit_centimes: 0,
  credit_centimes: 100_000,
  balance_after_centimes: 150_000,
  payment_mode: "cash",
  user_id: 1,
  note: "acompte",
  allocations: [{ purchase_id: 3, amount_centimes: 100_000 }],
  created_at: "2026-09-10 09:00:00",
};

const statement: SupplierStatementDto = {
  supplier_id: 7,
  from: "2026-09-01",
  to: "2026-09-10",
  opening_centimes: 0,
  entries: [payment],
  closing_centimes: 150_000,
};

describe("supplierDebtKindSchema", () => {
  test("takes a movement kind the supplier ledger writes", () => {
    expect(supplierDebtKindSchema.parse("purchase")).toBe("purchase");
  });

  test("refuses a sale, which is a supplier buying at the till", () => {
    expect(supplierDebtKindSchema.safeParse("sale").success).toBe(false);
  });

  test("carries the five the ledger writes and no sixth", () => {
    expect(supplierDebtKindSchema.options).toEqual([
      "opening",
      "purchase",
      "payment",
      "return",
      "adjustment",
    ]);
  });
});

describe("supplierSchema", () => {
  test("takes the fiche with the balance the core summed", () => {
    expect(supplierSchema.parse(supplier)).toEqual(supplier);
  });

  test("refuses a name longer than the core will store", () => {
    expect(supplierSchema.safeParse({ ...supplier, name: "a".repeat(201) }).success).toBe(false);
    expect(supplierSchema.safeParse({ ...supplier, name: "a".repeat(200) }).success).toBe(true);
  });

  test("refuses a balance JSON.parse had to round", () => {
    expect(supplierSchema.safeParse({ ...supplier, balance_centimes: 2_500.5 }).success).toBe(
      false,
    );
  });
});

describe("supplierAllocationSchema", () => {
  test("takes what a payment placed on one order", () => {
    const allocation = { purchase_id: 3, amount_centimes: 100_000 };
    expect(supplierAllocationSchema.parse(allocation)).toEqual(allocation);
  });

  test("refuses a fractional centime on the amount", () => {
    expect(
      supplierAllocationSchema.safeParse({ purchase_id: 3, amount_centimes: 100_000.5 }).success,
    ).toBe(false);
  });
});

describe("supplierEntrySchema", () => {
  test("takes a payment with the orders it settled", () => {
    expect(supplierEntrySchema.parse(payment)).toEqual(payment);
  });

  test("refuses a fraction in any of the three columns", () => {
    // One case per column. A single case would leave the other two free to
    // be loosened to `z.number()` with the suite still green.
    for (const column of ["debit_centimes", "credit_centimes", "balance_after_centimes"]) {
      expect(supplierEntrySchema.safeParse({ ...payment, [column]: 0.5 }).success).toBe(false);
    }
  });

  test("refuses a payment mode the ledger has no column for", () => {
    expect(supplierEntrySchema.safeParse({ ...payment, payment_mode: "credit" }).success).toBe(
      false,
    );
  });
});

describe("supplierLedgerSchema", () => {
  test("takes the movements and the balance they sum to", () => {
    const ledger = { supplier_id: 7, balance_centimes: 150_000, entries: [payment] };
    expect(supplierLedgerSchema.parse(ledger)).toEqual(ledger);
  });

  test("refuses an envelope whose balance came back rounded", () => {
    expect(
      supplierLedgerSchema.safeParse({
        supplier_id: 7,
        balance_centimes: 150_000.5,
        entries: [],
      }).success,
    ).toBe(false);
  });
});

describe("supplierStatementSchema", () => {
  test("takes the range with both balances the core read off its own column", () => {
    expect(supplierStatementSchema.parse(statement)).toEqual(statement);
  });

  test("refuses a day the API does not write", () => {
    expect(supplierStatementSchema.safeParse({ ...statement, from: "2026-9-1" }).success).toBe(
      false,
    );
  });
});
