// The new-purchase form's running total against the fixture the Rust core
// reads too (crates/kernel/tests/money_fixtures.rs). The fixture is the
// expectation: nothing here computes an expected value.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { MoneyError, purchaseOrderTotal, type PurchaseDraftLine } from "../src/totals";

interface FixtureOrder {
  name: string;
  lines: { unit_cost: number; qty_milli: number }[];
  transport: number;
  extra_costs: number;
}

interface PurchaseTotalFixture {
  cases: (FixtureOrder & { expected: number })[];
  refusals: (FixtureOrder & { error: string })[];
  js_overflow_cases: (FixtureOrder & { error: string })[];
}

const path = fileURLToPath(
  new URL("../../../fixtures/money/purchase_order_total_before_landing.json", import.meta.url),
);
// JSON.parse is `any`; the interface above is the contract the test reads the
// file through, and a fixture that stops matching it fails on the first
// assertion rather than silently.
const fixture: PurchaseTotalFixture = JSON.parse(readFileSync(path, "utf8"));

function linesOf(order: FixtureOrder): PurchaseDraftLine[] {
  return order.lines.map((l) => ({ unitCost: l.unit_cost, qtyMilli: l.qty_milli }));
}

function refusalOf(order: FixtureOrder): string {
  try {
    purchaseOrderTotal(linesOf(order), order.transport, order.extra_costs);
  } catch (error) {
    if (error instanceof MoneyError) return error.variant;
    throw error;
  }
  return "no refusal";
}

describe("purchase_order_total_before_landing", () => {
  it.each(fixture.cases)("$name", (c) => {
    expect(purchaseOrderTotal(linesOf(c), c.transport, c.extra_costs)).toBe(c.expected);
  });

  it.each([...fixture.refusals, ...fixture.js_overflow_cases])("$name", (c) => {
    expect(refusalOf(c)).toBe(c.error);
  });
});
