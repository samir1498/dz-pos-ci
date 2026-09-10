// A TVA rate as a person reads it. Two screens print one (the products
// table and the till's totals block), so the wording lives here rather than
// twice: the fixed rates keep the word the dictionary carries, which is the
// only way the Arabic percent sign ("٪") ever reaches the screen, and a rate
// the fixed list does not carry is built from `percent_sign` and
// `decimal_separator` so it matches.

import type { Key } from "@/i18n";

/** features.md, TVA rates row: 19 % standard, 9 % reduced, 0 % exempt. */
export const RATES: readonly { bps: number; key: Key }[] = [
  { bps: 1900, key: "rate_1900" },
  { bps: 900, key: "rate_900" },
  { bps: 0, key: "rate_0" },
];

/**
 * "7 %" for 700 bps, "7,50 %" for 750: the label of a rate the fixed list
 * does not carry. Both the sign and the decimal separator come from i18n
 * rather than a literal "%" and a hardcoded French comma.
 *
 * Ruling (coordinator review, 2026-09-09): numbers follow the shop's
 * own format, not the UI language. Amounts, quantities and rates all read
 * comma-decimal in fr, en and ar today, so `decimal_separator` is "," in
 * every dictionary; it stays a per-language key rather than a bare constant
 * because it is the one place a locale that reads differently would land.
 */
export function rateLabel(bps: number, percentSign: string, decimalSeparator: string): string {
  const percent = bps / 100;
  const numeral = Number.isInteger(percent)
    ? String(percent)
    : percent.toFixed(2).replace(".", decimalSeparator);
  return `${numeral} ${percentSign}`;
}

/**
 * The fixed word when the rate is one of `RATES`, the computed label
 * otherwise. Calling `rateLabel` for every rate always renders a Latin "%",
 * which is how the Arabic products table once showed "9 %" in a row while
 * its own dropdown showed "9 ٪" for the same value.
 */
export function rateCellLabel(bps: number, t: (key: Key) => string): string {
  const fixed = RATES.find((r) => r.bps === bps);
  return fixed !== undefined
    ? t(fixed.key)
    : rateLabel(bps, t("percent_sign"), t("decimal_separator"));
}
