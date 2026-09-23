// The rate labels two screens share. What is worth pinning is the thing the
// products table got wrong once: a rate rendered with a Latin "%" in a row
// while the dropdown beside it showed the Arabic "٪" for the same value. So
// every case here goes through the real dictionaries.

import { describe, expect, test } from "vitest";
import ar from "@/i18n/ar.json";
import fr from "@/i18n/fr.json";
import type { Key } from "@/i18n";
import { RATES, rateCellLabel, rateLabel } from "../../src/lib/rate";

/** `t` as a screen would pass it, reading the dictionary that ships. */
function translator(dict: Record<string, string>): (key: Key) => string {
  return (key) => dict[key] ?? key;
}

const tFr = translator(fr);
const tAr = translator(ar);

describe("rateLabel: a rate the fixed list does not carry", () => {
  test("a whole percent keeps no decimals", () => {
    expect(rateLabel(700, "%", ",")).toBe("7 %");
  });

  test("750 bps is seven and a half, comma-decimal", () => {
    expect(rateLabel(750, "%", ",")).toBe("7,50 %");
  });

  test("the separator is the one it is given, never a hardcoded comma", () => {
    expect(rateLabel(750, "%", ".")).toBe("7.50 %");
  });

  test("the sign is the one it is given, so Arabic gets its own", () => {
    expect(rateLabel(750, "٪", ",")).toBe("7,50 ٪");
    expect(rateLabel(700, "٪", ",")).toBe("7 ٪");
  });
});

describe("rateCellLabel: the dictionary word first", () => {
  test.each(RATES)("$bps bps reads the fr dictionary word", (r) => {
    expect(rateCellLabel(r.bps, tFr)).toBe(fr[r.key]);
  });

  test.each(RATES)("$bps bps reads the ar dictionary word", (r) => {
    expect(rateCellLabel(r.bps, tAr)).toBe(ar[r.key]);
    // The one thing the Latin sign would break.
    expect(rateCellLabel(r.bps, tAr)).toContain("٪");
    expect(rateCellLabel(r.bps, tAr)).not.toContain("%");
  });

  test("a rate outside the list is built from the dictionary's own sign", () => {
    expect(rateCellLabel(750, tFr)).toBe("7,50 %");
    expect(rateCellLabel(750, tAr)).toBe("7,50 ٪");
    expect(rateCellLabel(700, tAr)).toBe("7 ٪");
  });

  test("the two languages agree on the numeral and differ only in the sign", () => {
    expect(rateCellLabel(750, tFr).replace("%", "")).toBe(rateCellLabel(750, tAr).replace("٪", ""));
  });
});
