import { describe, expect, test } from "vitest";
import formatFixture from "../../../fixtures/money/format_centimes.json";
import {
  formatCentimes,
  formatQty,
  parseAmountToCentimes,
  parseQtyToMilli,
} from "../src/money";

// Thousands are grouped with a narrow no-break space, French and Algerian
// typography. Written as an escape so it stays visible in a diff.
const NBSP = "\u202f";

describe("formatCentimes", () => {
  test("splits dinars from centimes without a float", () => {
    expect(formatCentimes(0)).toBe("0,00");
    expect(formatCentimes(5)).toBe("0,05");
    expect(formatCentimes(920)).toBe("9,20");
    expect(formatCentimes(1234)).toBe("12,34");
    expect(formatCentimes(100_000)).toBe(`1${NBSP}000,00`);
  });

  test("keeps the sign on a negative amount", () => {
    expect(formatCentimes(-1234)).toBe("-12,34");
    expect(formatCentimes(-5)).toBe("-0,05");
  });

  test("holds an amount a float would round", () => {
    // 0.1 + 0.2 territory: these must survive exactly.
    expect(formatCentimes(1_000_000_000_001)).toBe(
      `10${NBSP}000${NBSP}000${NBSP}000,01`,
    );
  });

  test("refuses anything that is not a safe integer", () => {
    expect(() => formatCentimes(12.5)).toThrow(RangeError);
    expect(() => formatCentimes(Number.MAX_SAFE_INTEGER + 2)).toThrow(RangeError);
  });
});

describe("formatQty", () => {
  test("drops trailing zeros in the thousandths", () => {
    expect(formatQty(24_000)).toBe("24");
    expect(formatQty(1_500)).toBe("1,5");
    expect(formatQty(1_250)).toBe("1,25");
    expect(formatQty(1)).toBe("0,001");
    expect(formatQty(0)).toBe("0");
    expect(formatQty(-1_500)).toBe("-1,5");
  });
});

// The ticket the core prints reads the same cases through
// `dzpos_core::money::format` (crates/core/tests/money_fixtures.rs). A
// customer sees the total on the screen and then on paper, so the two
// formatters are one rule and this file is half of the proof.
describe("fixtures/money/format_centimes.json", () => {
  // Imported rather than read: tsc types every case from the file itself,
  // so a fixture that loses a field fails the type check before a test runs.
  const amounts = formatFixture.cases.map((c) => ({
    name: c.name,
    value: c.centimes,
    expected: c.expected,
  }));
  const quantities = formatFixture.qty_cases.map((c) => ({
    name: c.name,
    value: c.milli,
    expected: c.expected,
  }));

  test("keeps its cases", () => {
    expect(formatFixture.name).toBe("format_centimes");
    expect(amounts.length).toBeGreaterThanOrEqual(10);
    expect(quantities.length).toBeGreaterThan(0);
  });

  test.each(amounts)("$name", ({ value, expected }) => {
    expect(formatCentimes(value)).toBe(expected);
  });

  test.each(quantities)("$name", ({ value, expected }) => {
    expect(formatQty(value)).toBe(expected);
  });
});

describe("parseAmountToCentimes", () => {
  test("reads a comma or a dot", () => {
    expect(parseAmountToCentimes("12,34")).toBe(1234);
    expect(parseAmountToCentimes("12.34")).toBe(1234);
    expect(parseAmountToCentimes("12")).toBe(1200);
    expect(parseAmountToCentimes("0,05")).toBe(5);
    expect(parseAmountToCentimes(",5")).toBe(50);
  });

  test("pads a single decimal to centimes", () => {
    expect(parseAmountToCentimes("12,3")).toBe(1230);
  });

  test("ignores spaces used as a thousands separator", () => {
    expect(parseAmountToCentimes("1 000,50")).toBe(100_050);
    expect(parseAmountToCentimes(`1${NBSP}000,50`)).toBe(100_050);
  });

  test("round trips through formatCentimes", () => {
    for (const centimes of [0, 5, 920, 1234, 100_000, 999_999_99]) {
      expect(parseAmountToCentimes(formatCentimes(centimes))).toBe(centimes);
    }
  });

  test("refuses text that is not an amount", () => {
    expect(parseAmountToCentimes("")).toBeNull();
    expect(parseAmountToCentimes("abc")).toBeNull();
    expect(parseAmountToCentimes("12,345")).toBeNull();
    expect(parseAmountToCentimes("1,2,3")).toBeNull();
  });
});

describe("parseQtyToMilli", () => {
  test("reads up to three decimals into thousandths", () => {
    expect(parseQtyToMilli("24")).toBe(24_000);
    expect(parseQtyToMilli("1,5")).toBe(1_500);
    expect(parseQtyToMilli("1,25")).toBe(1_250);
    expect(parseQtyToMilli("0,001")).toBe(1);
  });

  test("round trips through formatQty", () => {
    for (const milli of [0, 1, 1_250, 24_000, 1_000_000]) {
      expect(parseQtyToMilli(formatQty(milli))).toBe(milli);
    }
  });

  test("refuses more precision than the column holds", () => {
    expect(parseQtyToMilli("1,2345")).toBeNull();
    expect(parseQtyToMilli("kg")).toBeNull();
  });
});
