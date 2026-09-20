// What the shop is owed for a basket, on the phone.
//
// The phone used to send `Σ selling_centimes × qty` as the amount the
// customer handed over. Under the réel régime that sum is HT: it carries
// neither the TVA nor the droit de timbre, so the server — which owes
// `total_ttc + stamp` — refused every cash sale a normal shop rings
// (`crates/core/src/services/sales.rs`, `settle`). Adversarial review of
// M6+M7, 2026-09-16.
//
// The desktop till never had that bug because it prices its basket through
// `@dzpos/shared`'s `computeTotals`, the TypeScript mirror of
// `crates/core/src/money` pinned to the same files under `fixtures/money/`.
// This module is the phone doing the same thing, so neither screen can
// drift from the core without a red test. What a screen computes is still
// only a preview: the amounts a document carries are the API's answer
// (rule 2), and the ticket prints what the core rang.

import {
  computeTotals,
  formatCentimes,
  parseAmountToCentimes,
  type RegimeDto,
  type Totals,
  type TotalsLine,
} from "@dzpos/shared";

/** Showing an amount is the shared package's job, and now the phone's too.
 *
 *  This file carried its own copy: the same split into dinars and centimes,
 *  without the thousands grouping and without the safe-integer check. Two
 *  spellings of how the product shows money, one of which nobody was
 *  watching. Re-exported rather than removed, because the screens import it
 *  from here and where it lives is not their business. */
export { formatCentimes };

/** The phone shows one price per product and sells whole units. */
export type Product = {
  id: number;
  name: string;
  selling_centimes: number;
  rate_bps: number;
};

export type CartLine = { product: Product; qty: number };

/** Thousandths in one unit, as `@dzpos/shared` counts them. */
const MILLI_PER_UNIT = 1_000;

/** Whether the droit de timbre applies at all, mirroring `STAMP_ENABLED`
 * in `crates/core/src/services/sales.rs`. The desktop till keeps the same
 * constant; both follow the core if it ever becomes a setting. */
const STAMP_ENABLED = true;

/**
 * Prices the basket the way the core will. Cash only: the phone takes no
 * other payment mode yet, and the stamp is owed on cash above 300,00 DA.
 *
 * Throws `MoneyError` exactly where the core would answer one — an amount
 * past what a Number holds exactly. The caller shows the basket as
 * unpriced rather than posting a total nobody can stand behind.
 */
export function priceBasket(cart: readonly CartLine[], regime: RegimeDto): Totals {
  const lines: TotalsLine[] = cart.map((line) => ({
    qtyMilli: line.qty * MILLI_PER_UNIT,
    unitPrice: line.product.selling_centimes,
    lineDiscount: 0,
    rateBps: line.product.rate_bps,
  }));
  return computeTotals(lines, {
    globalDiscount: 0,
    paymentMode: "cash",
    stampEnabled: STAMP_ENABLED,
    regime,
  });
}

/**
 * Reads what the cashier typed into the tendered box. Blank is not zero:
 * it is a cashier who has not counted the notes yet, so it answers null
 * and the screen waits rather than posting an amount nobody handed over.
 * Accepts a comma as the decimal mark, which is what an Algerian keyboard
 * and an Algerian price tag both use.
 *
 * The digits are read by `parseAmountToCentimes`, digit by digit. This
 * function used to do `Math.round(Number(trimmed) * 100)`, which is a float
 * on the path to an amount and the first non-negotiable in CLAUDE.md. At
 * Algerian prices it landed on the right integer every time, so it was a
 * banned pattern and a second parser rather than a wrong total, and the
 * architecture review of 2026-09-17 named it as both.
 *
 * The negative is this function's own rule and not the parser's. A price
 * can be negative, an avoir is made of them; an amount handed across a
 * counter cannot be.
 */
/** The change the server worked out, or null if it did not answer one.
 *
 *  Checked because the phone's own `call<T>` hands a body straight back
 *  without a schema, unlike `@dzpos/shared`'s client, and
 *  `formatCentimes` throws on anything that is not a safe integer. A
 *  `change_centimes` that arrived as a string or a float would take the
 *  till screen down mid-sale, and the phone carries no error boundary, so
 *  the whole app would go with it. Null instead: the change line is not
 *  drawn, the sale is still rung, and the cashier reads the paper. */
export function readChange(value: unknown): number | null {
  return Number.isSafeInteger(value) && typeof value === "number" ? value : null;
}

export function readTendered(typed: string): number | null {
  const centimes = parseAmountToCentimes(typed);
  if (centimes === null || centimes < 0) return null;
  return centimes;
}
