// The pure pieces of the products screen: the unit list, the category
// sentinels, the rate choices a category or a stored product can carry, and
// the filter the list is read through. Nothing here fetches and nothing here
// renders a form; the screen and the fiche both read from here so the two
// cannot answer the same question two different ways.

import type { CategoryDto, ProductDto, UnitDto } from "@dzpos/shared";

import { stockState } from "@/components/ProductTile";
import type { Key } from "@/i18n";
import { RATES, rateLabel } from "@/lib/rate";

export const UNITS: readonly UnitDto[] = ["piece", "kg", "litre", "box"];

export const UNIT_KEY: Record<UnitDto, Key> = {
  piece: "unit_piece",
  kg: "unit_kg",
  litre: "unit_litre",
  box: "unit_box",
};

/**
 * Two sentinels, because a `SelectItem` refuses an empty value: Radix uses
 * the empty string internally for "nothing chosen" and throws on an item
 * that claims it. They are spelled here once and turned back into `null` and
 * "no filter" at the two places that read them.
 */
export const NO_CATEGORY = "none";
export const ANY_CATEGORY = "all";

/**
 * The fixed choices plus any category default (or stored product rate) the
 * list does not carry. The migration allows any rate between 0 and 10 000
 * bps on a category; without this a 700 bps category showed "19 %" while
 * the form posted 700.
 */
export function rateOptions(
  categories: readonly CategoryDto[],
  t: (key: Key) => string,
  stored?: number,
): { value: string; label: string }[] {
  const fixed = RATES.map((rate) => ({ value: String(rate.bps), label: t(rate.key) }));
  const known = new Set(RATES.map((rate) => rate.bps));
  const candidates = categories.map((c) => c.default_rate_bps);
  // A product edited later keeps showing the rate it was stored with, even
  // one no category offers any more.
  if (stored !== undefined) candidates.push(stored);
  const percentSign = t("percent_sign");
  const decimalSeparator = t("decimal_separator");
  const extra = [...new Set(candidates)]
    .filter((bps) => !known.has(bps))
    .sort((a, b) => b - a)
    .map((bps) => ({ value: String(bps), label: rateLabel(bps, percentSign, decimalSeparator) }));
  return [...fixed, ...extra];
}

/** What the filter bar is set to. All three off is the whole catalogue. */
export interface Filters {
  readonly search: string;
  readonly category: string;
  readonly lowOnly: boolean;
}

/**
 * The filter bar applied to one product. Search reads the name and the
 * barcode, because a shop holding the box in its hand types the digits off
 * it rather than the name printed on it; both are compared case-folded so a
 * catalogue typed in capitals still answers.
 *
 * "Running out" is the product's own threshold and not a number this screen
 * picked, which is the same rule the tile draws its chip from.
 */
export function keeps(product: ProductDto, filters: Filters): boolean {
  const needle = filters.search.trim().toLocaleLowerCase();
  if (needle !== "") {
    const haystack = `${product.name} ${product.barcode ?? ""}`.toLocaleLowerCase();
    if (!haystack.includes(needle)) return false;
  }
  if (filters.category === NO_CATEGORY) {
    if (product.category_id !== null) return false;
  } else if (filters.category !== ANY_CATEGORY) {
    if (String(product.category_id) !== filters.category) return false;
  }
  if (filters.lowOnly && stockState(product.qty_on_hand_milli, product.low_stock_at_milli) === "ok") {
    return false;
  }
  return true;
}
