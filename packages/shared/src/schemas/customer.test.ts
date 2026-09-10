// Contract: the fiche and the two ways its balance moves. Every amount is an
// exact integer, because a debt JSON.parse had to round is a figure a shop
// would read to a customer; the payment method is cash or card and never
// credit, since paying a debt on credit is not a payment.

import { describe, expect, test } from "vitest";

import type { CustomerDto } from "../generated/CustomerDto";
import type { DebtEntryDto } from "../generated/DebtEntryDto";
import type { PaymentDto } from "../generated/PaymentDto";
import {
  customerLedgerSchema,
  customerPaymentsSchema,
  customerSchema,
  debtEntrySchema,
  debtKindSchema,
  partyKindSchema,
  paymentAllocationSchema,
  paymentMethodSchema,
  paymentSchema,
} from "./customer";

const customer: CustomerDto = {
  id: 4,
  shop_id: 1,
  name: "Boulangerie Rahma",
  party_kind: "company",
  phone: "0550112233",
  address: null,
  rc: "16/00-1234567B25",
  nif: null,
  nis: null,
  ai: null,
  credit_limit_centimes: 1_000_000,
  warn_threshold_centimes: 800_000,
  notes: null,
  active: true,
  balance_centimes: 250_000,
};

const entry: DebtEntryDto = {
  id: 9,
  customer_id: 4,
  document_id: 12,
  kind: "sale",
  debit_centimes: 26_180,
  credit_centimes: 0,
  balance_after_centimes: 250_000,
  user_id: 1,
  note: null,
  created_at: "2026-09-09 10:00:00",
};

const payment: PaymentDto = {
  ledger_id: 10,
  customer_id: 4,
  amount_centimes: 50_000,
  payment_mode: "cash",
  note: null,
  balance_after_centimes: 200_000,
  allocations: [{ document_id: 12, amount_centimes: 26_180 }],
  created_at: "2026-09-10 09:00:00",
};

describe("partyKindSchema", () => {
  test("takes the half of the facture the fiche is", () => {
    expect(partyKindSchema.parse("consumer")).toBe("consumer");
  });

  test("refuses a party kind the core has no rule for", () => {
    expect(partyKindSchema.safeParse("association").success).toBe(false);
  });

  test("carries the two the Rust enum writes and no third", () => {
    expect(partyKindSchema.options).toEqual(["company", "consumer"]);
  });
});

describe("debtKindSchema", () => {
  test("takes a movement kind the ledger writes", () => {
    expect(debtKindSchema.parse("adjustment")).toBe("adjustment");
  });

  test("refuses a kind nothing writes", () => {
    expect(debtKindSchema.safeParse("refund").success).toBe(false);
  });

  test("carries the five the ledger writes and no sixth", () => {
    expect(debtKindSchema.options).toEqual(["opening", "sale", "payment", "avoir", "adjustment"]);
  });
});

describe("paymentMethodSchema", () => {
  test("takes cash and card", () => {
    expect(paymentMethodSchema.parse("card")).toBe("card");
  });

  test("refuses credit, which is how a debt is made and not how it is paid", () => {
    expect(paymentMethodSchema.safeParse("credit").success).toBe(false);
  });

  test("carries the two ways money crosses a counter and no third", () => {
    expect(paymentMethodSchema.options).toEqual(["cash", "card"]);
  });
});

describe("customerSchema", () => {
  test("takes the fiche with the balance the core summed", () => {
    expect(customerSchema.parse(customer)).toEqual(customer);
  });

  test("refuses a balance JSON.parse had to round", () => {
    expect(customerSchema.safeParse({ ...customer, balance_centimes: 2_500.5 }).success).toBe(
      false,
    );
  });
});

describe("debtEntrySchema", () => {
  test("takes a movement and the balance as of itself", () => {
    expect(debtEntrySchema.parse(entry)).toEqual(entry);
  });

  test("refuses a movement whose kind the ledger does not write", () => {
    expect(debtEntrySchema.safeParse({ ...entry, kind: "refund" }).success).toBe(false);
  });

  test("refuses a fraction in any of the three columns", () => {
    // One case per column. A single case would leave the other two free to
    // be loosened to `z.number()` with the suite still green, and a debit
    // that came back as 26 180,5 is a line a shop would read to a customer.
    for (const column of ["debit_centimes", "credit_centimes", "balance_after_centimes"]) {
      expect(debtEntrySchema.safeParse({ ...entry, [column]: 0.5 }).success).toBe(false);
    }
  });
});

describe("customerLedgerSchema", () => {
  test("takes the movements and the balance they sum to", () => {
    const ledger = { customer_id: 4, balance_centimes: 250_000, entries: [entry] };
    expect(customerLedgerSchema.parse(ledger)).toEqual(ledger);
  });

  test("refuses a balance that came back with a fraction on it", () => {
    const ledger = { customer_id: 4, balance_centimes: 2_500.5, entries: [entry] };
    expect(customerLedgerSchema.safeParse(ledger).success).toBe(false);
  });

  test("refuses a ledger carrying a movement of another shape", () => {
    const ledger = { customer_id: 4, balance_centimes: 250_000, entries: [{ id: 9 }] };
    expect(customerLedgerSchema.safeParse(ledger).success).toBe(false);
  });
});

describe("paymentAllocationSchema", () => {
  test("takes the document a payment settled and by how much", () => {
    const allocation = { document_id: 12, amount_centimes: 26_180 };
    expect(paymentAllocationSchema.parse(allocation)).toEqual(allocation);
  });

  test("refuses an allocation with no amount", () => {
    expect(paymentAllocationSchema.safeParse({ document_id: 12 }).success).toBe(false);
  });

  test("refuses an amount that came back with a fraction on it", () => {
    const allocation = { document_id: 12, amount_centimes: 261.8 };
    expect(paymentAllocationSchema.safeParse(allocation).success).toBe(false);
  });
});

describe("paymentSchema", () => {
  test("takes a payment with the documents it settled", () => {
    expect(paymentSchema.parse(payment)).toEqual(payment);
  });

  test("takes an opening movement, whose mode is null", () => {
    expect(paymentSchema.parse({ ...payment, payment_mode: null })).toMatchObject({
      payment_mode: null,
    });
  });

  test("refuses a fraction in the amount and in the balance it left", () => {
    for (const column of ["amount_centimes", "balance_after_centimes"]) {
      expect(paymentSchema.safeParse({ ...payment, [column]: 500.5 }).success).toBe(false);
    }
  });

  test("refuses a payment with no allocations list at all", () => {
    const { allocations: _allocations, ...withoutAllocations } = payment;
    expect(paymentSchema.safeParse(withoutAllocations).success).toBe(false);
  });
});

describe("customerPaymentsSchema", () => {
  test("takes the payments and the whole ledger's balance", () => {
    const payments = { customer_id: 4, balance_centimes: 200_000, payments: [payment] };
    expect(customerPaymentsSchema.parse(payments)).toEqual(payments);
  });

  test("refuses an envelope whose balance came back rounded", () => {
    const payments = { customer_id: 4, balance_centimes: 2_000.5, payments: [payment] };
    expect(customerPaymentsSchema.safeParse(payments).success).toBe(false);
  });
});
