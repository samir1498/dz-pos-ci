// Contract: what a recount looks like on the wire. Quantities are
// thousandths of a unit and every one of them is an integer, because a
// fractional figure on this path is a server the client was not generated
// against and the panel would print it as a correction that never happened.

import { describe, expect, test } from "vitest";

import type { LastStockRecountDto } from "../generated/LastStockRecountDto";
import type { StockDriftDto } from "../generated/StockDriftDto";
import type { StockRecountDto } from "../generated/StockRecountDto";
import { lastStockRecountSchema, stockDriftSchema, stockRecountSchema } from "./stock";

const drift: StockDriftDto = {
  product_id: 7,
  name: "Sucre 1kg",
  cached_milli: 99_000,
  ledger_milli: 24_000,
  difference_milli: -75_000,
};

const run: StockRecountDto = {
  day: "2026-09-10",
  products_checked: 42,
  drifts: [drift],
};

const last: LastStockRecountDto = {
  last_run_day: "2026-09-10",
  drifts: [drift],
};

describe("a stock drift", () => {
  test("a row from the API parses whole", () => {
    expect(stockDriftSchema.parse(drift)).toEqual(drift);
  });

  test("a fractional quantity is refused rather than shown as a correction", () => {
    expect(stockDriftSchema.safeParse({ ...drift, ledger_milli: 24_000.5 }).success).toBe(false);
    expect(stockDriftSchema.safeParse({ ...drift, difference_milli: -75_000.5 }).success).toBe(
      false,
    );
  });

  test("a drift without the product's name is refused", () => {
    // The panel names the product a shop owner has to go and look at, and a
    // row that arrived without one would leave them an id to hunt for.
    const { name: _name, ...unnamed } = drift;
    expect(stockDriftSchema.safeParse(unnamed).success).toBe(false);
  });
});

describe("a recount", () => {
  test("a run from the API parses whole", () => {
    expect(stockRecountSchema.parse(run)).toEqual(run);
  });

  test("a run that corrected nothing still carries its day and its count", () => {
    const quiet: StockRecountDto = { day: "2026-09-10", products_checked: 42, drifts: [] };
    expect(stockRecountSchema.parse(quiet)).toEqual(quiet);
  });

  test("a day that is not a calendar day is refused", () => {
    expect(stockRecountSchema.safeParse({ ...run, day: "10/09/2026" }).success).toBe(false);
  });
});

describe("the last recount", () => {
  test("an answer from the API parses whole", () => {
    expect(lastStockRecountSchema.parse(last)).toEqual(last);
  });

  test("a shop that has never recounted has a null day and no drift", () => {
    const never: LastStockRecountDto = { last_run_day: null, drifts: [] };
    expect(lastStockRecountSchema.parse(never)).toEqual(never);
  });

  test("a day that is not a calendar day is refused", () => {
    expect(lastStockRecountSchema.safeParse({ ...last, last_run_day: "hier" }).success).toBe(false);
  });
});
