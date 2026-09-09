// The mockup's money.js against the same files the Rust core is pinned by.
// If a fixture and money.js disagree, the mockup is showing a wrong total.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { MoneyError, amountInWords, computeTotals, pct, stamp } from "./shared/money.js";

function fixture(name) {
  const path = fileURLToPath(new URL(`../fixtures/money/${name}.json`, import.meta.url));
  return JSON.parse(readFileSync(path, "utf8"));
}

const rounding = fixture("tva_rounding_once_per_rate");
const stampFixture = fixture("stamp_progressive_tranches");

// The rate column of a line and of a TVA group is basis points, not percent.
const toLine = (l) => ({
  qty: l.qty,
  unitPrice: l.unit_price,
  lineDiscount: l.line_discount,
  rateBps: l.rate_bps,
});
const toOpts = (o) => ({
  globalDiscount: o.global_discount,
  paymentMode: o.payment_mode,
  stampEnabled: o.stamp_enabled,
  regime: o.regime,
});

describe("tva_rounding_once_per_rate", () => {
  it("keeps its cases", () => {
    expect(rounding.pct_cases.length).toBeGreaterThanOrEqual(10);
    expect(rounding.totals_cases.length).toBeGreaterThanOrEqual(10);
  });

  it.each(rounding.pct_cases)("pct: $name", (c) => {
    expect(pct(c.amount, c.rate_bps)).toBe(c.expected);
  });

  // rounding.overflow_cases are skipped for the same reason as the stamp's:
  // the amount is i64::MAX, past Number.MAX_SAFE_INTEGER, so JS cannot hold
  // the input. Rust covers them; there is nothing to assert here.
  it.each(rounding.invalid_rate_cases)("refuses a rate above one whole: $name", (c) => {
    expect(() => pct(10000, c.rate_bps)).toThrowError(/RateOutOfRange/);
  });

  it.each(rounding.totals_cases)("totals: $name", (c) => {
    const got = computeTotals(c.input.lines.map(toLine), toOpts(c.input.opts));
    const e = c.expected;
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
  });

  it.each(rounding.totals_error_cases)("refuses: $name", (c) => {
    expect(() => computeTotals(c.input.lines.map(toLine), toOpts(c.input.opts))).toThrowError(
      new RegExp(c.error),
    );
  });
});

describe("stamp_progressive_tranches", () => {
  it("keeps its cases", () => {
    expect(stampFixture.cases.length).toBeGreaterThanOrEqual(20);
  });

  // overflow_cases are skipped: i64::MAX is past Number.MAX_SAFE_INTEGER,
  // so the JS side cannot represent that input at all.
  it.each(stampFixture.cases)("$name", (c) => {
    expect(stamp(c.input.total_ttc, c.input.mode)).toBe(c.expected.stamp);
  });
});

// The amount in words is a legal field of the facture (décret 05-468), so
// the mockup writes it exactly as the core does: the same golden files pin
// both. Arabic stays a placeholder in the mockup until the native review.
describe("amount in words follows the golden files", () => {
  for (const lang of ["fr", "en"]) {
    const golden = fixture(`words_${lang}_golden`);
    it.each(golden.cases)(`${lang}: $centimes`, ({ centimes, words }) => {
      expect(amountInWords(centimes, lang)).toBe(words);
    });
  }

  it("refuses a negative amount and one past the scales it knows", () => {
    expect(() => amountInWords(-1, "fr")).toThrow(MoneyError);
    expect(() => amountInWords(100_000_000_000_000, "en")).toThrow(MoneyError);
  });
});
