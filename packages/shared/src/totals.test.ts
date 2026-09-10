// The shipped TypeScript totals against the same files the Rust core is
// pinned by. `design/shared/money.js` was the first draft of this and its
// own test reads these fixtures; a disagreement between the two runners
// means a till screen would show a total the ticket does not carry.
//
// The fixture is the expectation: nothing here computes an expected value.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import type { PaymentModeDto } from "./generated/PaymentModeDto";
import type { RegimeDto } from "./generated/RegimeDto";
import {
  MoneyError,
  STAMP_RATE_HIGH,
  STAMP_RATE_MID,
  computeTotals,
  lineTotal,
  pct,
  stamp,
  type TotalsLine,
  type TotalsOptions,
} from "./totals";

interface FixtureLine {
  qty_milli: number;
  unit_price: number;
  line_discount: number;
  rate_bps: number;
}

interface FixtureOpts {
  global_discount: number;
  payment_mode: PaymentModeDto;
  stamp_enabled: boolean;
  regime: RegimeDto;
}

interface FixtureTotalsCase {
  name: string;
  input: { lines: FixtureLine[]; opts: FixtureOpts };
  expected: {
    total_ht: number;
    discount: number;
    subtotal_ht: number;
    tva_by_rate: { rate_bps: number; base: number; amount: number }[];
    tva: number;
    total_ttc: number;
    stamp: number;
    net_to_pay: number;
  };
}

interface FixtureErrorCase {
  name: string;
  input: { lines: FixtureLine[]; opts: FixtureOpts };
  error: string;
}

interface RoundingFixture {
  pct_cases: { name: string; amount: number; rate_bps: number; expected: number }[];
  invalid_rate_cases: { name: string; rate_bps: number }[];
  overflow_cases: { name: string; amount: number; rate_bps: number }[];
  totals_cases: FixtureTotalsCase[];
  totals_error_cases: FixtureErrorCase[];
}

interface LineFixture {
  line_cases: { name: string; unit_price: number; qty_milli: number; expected: number }[];
  totals_cases: FixtureTotalsCase[];
}

interface StampFixture {
  cases: {
    name: string;
    input: { total_ttc: number; mode: PaymentModeDto };
    expected: { stamp: number };
  }[];
}

function fixture<T>(name: string): T {
  const path = fileURLToPath(
    new URL(`../../../fixtures/money/${name}.json`, import.meta.url),
  );
  // JSON.parse is `any`; the interfaces above are the contract this test
  // reads the file through, and a fixture that stops matching them fails
  // on the first assertion rather than silently.
  const parsed: T = JSON.parse(readFileSync(path, "utf8"));
  return parsed;
}

const rounding = fixture<RoundingFixture>("tva_rounding_once_per_rate");
const fractional = fixture<LineFixture>("line_total_fractional_qty");
const stampFixture = fixture<StampFixture>("stamp_progressive_tranches");

/** The fixture writes a line the way the wire does; the module takes the
 * shape a screen builds. */
function toLine(l: FixtureLine): TotalsLine {
  return {
    qtyMilli: l.qty_milli,
    unitPrice: l.unit_price,
    lineDiscount: l.line_discount,
    rateBps: l.rate_bps,
  };
}

function toOpts(o: FixtureOpts): TotalsOptions {
  return {
    globalDiscount: o.global_discount,
    paymentMode: o.payment_mode,
    stampEnabled: o.stamp_enabled,
    regime: o.regime,
  };
}

function expectTotals(c: FixtureTotalsCase): void {
  const got = computeTotals(c.input.lines.map(toLine), toOpts(c.input.opts));
  const e = c.expected;
  // Every column the fixture names, in the order the document prints them.
  expect(got.totalHt).toBe(e.total_ht);
  expect(got.discount).toBe(e.discount);
  expect(got.subtotalHt).toBe(e.subtotal_ht);
  expect(got.tvaByRate).toEqual(
    e.tva_by_rate.map((g) => ({ rateBps: g.rate_bps, base: g.base, amount: g.amount })),
  );
  expect(got.tva).toBe(e.tva);
  expect(got.totalTtc).toBe(e.total_ttc);
  expect(got.stamp).toBe(e.stamp);
  expect(got.netToPay).toBe(e.net_to_pay);
}

describe("pct: one rounding per rate group, half away from zero", () => {
  it("keeps its cases", () => {
    expect(rounding.pct_cases.length).toBeGreaterThanOrEqual(10);
    expect(rounding.totals_cases.length).toBeGreaterThanOrEqual(10);
  });

  it.each(rounding.pct_cases)("$name", (c) => {
    expect(pct(c.amount, c.rate_bps)).toBe(c.expected);
  });

  // The Rust case is an i64 product that overflows i64. JS has no i64: the
  // amount itself is already past Number.MAX_SAFE_INTEGER by the time
  // JSON.parse is done with it, so the refusal comes one step earlier and
  // carries the same variant.
  it.each(rounding.overflow_cases)("refuses: $name", (c) => {
    expect(() => pct(c.amount, c.rate_bps)).toThrowError(/Overflow/);
  });

  it("refuses a product it could not keep exact, and keeps the exact ones", () => {
    // 10^13 x 19 % is 1,9 x 10^16, past 2^53: the product has to be BigInt
    // or the last centimes are invented.
    expect(pct(10_000_000_000_000, 1900)).toBe(1_900_000_000_000);
    expect(pct(9_007_199_254_740_991, 1)).toBe(900_719_925_474);
    expect(() => pct(Number.MAX_SAFE_INTEGER + 2, 1900)).toThrowError(/Overflow/);
  });

  it("refuses a rate that is not a whole number of basis points", () => {
    // 19,005 % is not a rate the core can hold, and NaN is what an empty
    // field parses to: both are a MoneyError, never a RangeError out of
    // BigInt.
    expect(() => pct(100, 1900.5)).toThrowError(MoneyError);
    expect(() => pct(100, 1900.5)).toThrowError(/Overflow/);
    expect(() => pct(100, Number.NaN)).toThrowError(MoneyError);
  });

  it.each(rounding.invalid_rate_cases)("refuses a rate above one whole: $name", (c) => {
    expect(() => pct(10_000, c.rate_bps)).toThrowError(/RateOutOfRange/);
  });
});

describe("computeTotals: the totals table of features.md", () => {
  it.each(rounding.totals_cases)("$name", expectTotals);

  it.each(rounding.totals_error_cases)("refuses: $name", (c) => {
    expect(() => computeTotals(c.input.lines.map(toLine), toOpts(c.input.opts))).toThrowError(
      new RegExp(c.error),
    );
  });

  it("refuses rather than clamps, with a variant the caller can branch on", () => {
    let caught: unknown = null;
    try {
      computeTotals([{ qtyMilli: 1000, unitPrice: 10_000, lineDiscount: 10_001, rateBps: 1900 }], {
        globalDiscount: 0,
        paymentMode: "cash",
        stampEnabled: true,
        regime: "reel",
      });
    } catch (error: unknown) {
      caught = error;
    }
    expect(caught).toBeInstanceOf(MoneyError);
    if (caught instanceof MoneyError) {
      expect(caught.variant).toBe("LineDiscountAboveLine");
    }
  });

  it("leaves the lines it was given untouched", () => {
    const lines: TotalsLine[] = [
      { qtyMilli: 1500, unitPrice: 20_000, lineDiscount: 0, rateBps: 900 },
    ];
    const before = JSON.stringify(lines);
    computeTotals(lines, {
      globalDiscount: 0,
      paymentMode: "cash",
      stampEnabled: true,
      regime: "reel",
    });
    expect(JSON.stringify(lines)).toBe(before);
  });
});

describe("lineTotal: a fractional quantity is rounded once", () => {
  it("keeps its cases", () => {
    expect(fractional.line_cases.length).toBeGreaterThanOrEqual(10);
    expect(fractional.totals_cases.length).toBeGreaterThanOrEqual(1);
  });

  it.each(fractional.line_cases)("$name", (c) => {
    expect(lineTotal(c.unit_price, c.qty_milli)).toBe(c.expected);
  });

  it.each(fractional.totals_cases)("totals: $name", expectTotals);
});

describe("stamp: the progressive tranches", () => {
  it("keeps its cases", () => {
    expect(stampFixture.cases.length).toBeGreaterThanOrEqual(20);
  });

  it.each(stampFixture.cases)("$name", (c) => {
    expect(stamp(c.input.total_ttc, c.input.mode)).toBe(c.expected.stamp);
  });

  // The tranche count divided in floating point, which is the one place in
  // this file where an amount left the integers. The core cuts the same
  // tranches with i64 and answers `Overflow` when a total will not fit, so a
  // total a Number cannot hold has to be refused here too rather than
  // answered from digits that were already lost.
  it("refuses a total a Number can no longer hold", () => {
    expect(() => stamp(Number.MAX_SAFE_INTEGER + 2, "cash")).toThrowError(MoneyError);
    expect(() => stamp(Number.MAX_SAFE_INTEGER + 2, "cash")).toThrowError(/Overflow/);
  });

  // A total at the top of the safe range still has an exact answer, and the
  // whole of it is asserted here rather than left to the fixture: 90 071
  // 992 547 409,91 DA is 900 719 925 475 tranches, rounded up, in the band
  // above 100 000,00 DA, so 2,00 DA on each of them.
  it("counts the tranches of the largest total that is still exact", () => {
    const tranches = 900_719_925_475;
    expect(stamp(Number.MAX_SAFE_INTEGER, "cash")).toBe(tranches * STAMP_RATE_HIGH);
  });

  // The band is read off the total and the tranches are counted off the same
  // total, so a total one centime over a band boundary charges every one of
  // its tranches at the higher rate: 30 000,01 DA is 301 tranches at 1,50 DA
  // and not 300 at 1,00 with one at 1,50.
  it("charges the whole amount at the band the total falls in", () => {
    expect(stamp(3_000_001, "cash")).toBe(301 * STAMP_RATE_MID);
  });
});
