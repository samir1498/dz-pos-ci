// The totals table of docs/features.md §3, in integer centimes, so a till
// screen can show a live preview of what it is about to post. The Rust core
// (crates/core/src/money) is the reference and stays the only thing a ticket
// prints: what a screen computes here is display, and the amounts the
// document carries are the API's answer.
//
// Promoted from `design/shared/money.js`, whose own header calls itself the
// first draft of this file. Both are pinned by the same files under
// `fixtures/money/`, so neither can drift from the core without a red test.
//
// No float touches an amount: every product that could pass
// Number.MAX_SAFE_INTEGER goes through BigInt and comes back an integer.

import type { PaymentModeDto } from "./generated/PaymentModeDto";
import type { RegimeDto } from "./generated/RegimeDto";

/** Basis points in one whole: 10 000 bps = 100 %. */
export const BPS_PER_WHOLE = 10_000;

/** Thousandths in one unit: a line sells 1,5 kg as 1500. */
export const MILLI_PER_UNIT = 1_000;

// ---- droit de timbre: stamp_progressive_tranches ----
/** Nothing is due at or below 300,00 DA. */
export const STAMP_FLOOR = 30_000;
/** One tranche, 100,00 DA; the count is rounded up. */
export const STAMP_TRANCHE = 10_000;
/** Highest amount charged at 1,00 DA per tranche: 30 000,00 DA. */
export const STAMP_BAND_LOW = 3_000_000;
/** Highest amount charged at 1,50 DA per tranche: 100 000,00 DA. */
export const STAMP_BAND_MID = 10_000_000;
export const STAMP_RATE_LOW = 100;
export const STAMP_RATE_MID = 150;
export const STAMP_RATE_HIGH = 200;
/** Nothing due is charged below 5,00 DA. */
export const STAMP_MIN = 500;

/** The refusals, named after the Rust `MoneyError` variants so a fixture
 * case reads the same in both runners. */
export type MoneyErrorVariant =
  | "Overflow"
  | "RateOutOfRange"
  | "NegativeQuantity"
  | "NegativeUnitPrice"
  | "NegativeDiscount"
  | "LineDiscountAboveLine"
  | "GlobalDiscountAboveTotal";

/** A basket the core would refuse, refused here first so the screen can say
 * why without a round trip. */
export class MoneyError extends Error {
  readonly variant: MoneyErrorVariant;

  constructor(variant: MoneyErrorVariant) {
    super(variant);
    this.name = "MoneyError";
    this.variant = variant;
  }
}

/** One basket line, priced. `rateBps` is the product's rate, not a percent. */
export interface TotalsLine {
  /** Thousandths of the unit: 1,5 kg is 1500. */
  readonly qtyMilli: number;
  readonly unitPrice: number;
  readonly lineDiscount: number;
  readonly rateBps: number;
}

export interface TotalsOptions {
  readonly globalDiscount: number;
  readonly paymentMode: PaymentModeDto;
  readonly stampEnabled: boolean;
  readonly regime: RegimeDto;
}

/** One row of the TVA recap the document prints. */
export interface TvaGroup {
  readonly rateBps: number;
  readonly base: number;
  readonly amount: number;
}

export interface Totals {
  readonly totalHt: number;
  readonly discount: number;
  readonly subtotalHt: number;
  readonly tvaByRate: TvaGroup[];
  readonly tva: number;
  readonly totalTtc: number;
  readonly stamp: number;
  readonly netToPay: number;
}

/**
 * A whole number of centimes (or thousandths) as a BigInt. The Rust core
 * works in i64 and answers `Overflow` when a value leaves it; JS has no
 * i64, so the edge that matters here is `Number.MAX_SAFE_INTEGER`: past it
 * a Number has already lost digits and no BigInt can get them back. Same
 * variant, one step earlier.
 */
function exact(value: number): bigint {
  if (!Number.isSafeInteger(value)) throw new MoneyError("Overflow");
  return BigInt(value);
}

/** The other end: a product kept exact in BigInt is only an amount again if
 * it fits back into a Number without rounding. */
function safe(value: bigint): number {
  if (value > BigInt(Number.MAX_SAFE_INTEGER) || value < -BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new MoneyError("Overflow");
  }
  return Number(value);
}

/**
 * amount x rate, rounded once to the centime, half away from zero.
 * tva_rounding_once_per_rate
 */
export function pct(amount: number, rateBps: number): number {
  if (rateBps < 0 || rateBps > BPS_PER_WHOLE) throw new MoneyError("RateOutOfRange");
  const per = BigInt(BPS_PER_WHOLE);
  // The rate goes through `exact` too: a fractional or NaN rate slips past
  // the range check above (every comparison with NaN is false) and BigInt
  // would answer a RangeError, which is not a refusal a screen can read.
  const raw = exact(amount) * exact(rateBps);
  const sign = raw < 0n ? -1n : 1n;
  return safe(sign * ((raw * sign + per / 2n) / per));
}

/**
 * Droit de timbre on total_ttc. Cash only; zero at or under 300,00 DA;
 * ceil(amount / 100,00 DA) tranches at the band rate of the whole amount;
 * minimum 5,00 DA; no cap. stamp_progressive_tranches
 */
export function stamp(totalTtc: number, mode: PaymentModeDto): number {
  if (mode !== "cash" || totalTtc <= STAMP_FLOOR) return 0;
  // Cut in BigInt, the way `money/stamp.rs` cuts them in i64 and the way
  // `pct` above works: the tranche count divided in floating point, which
  // put a tax on an amount that had already been rounded to get there. The
  // ceiling is the integer form, `(amount + tranche - 1) / tranche`, and a
  // total a Number can no longer hold is an `Overflow` rather than an answer
  // built out of digits that are gone.
  const tranche = BigInt(STAMP_TRANCHE);
  const tranches = (exact(totalTtc) + tranche - 1n) / tranche;
  const rate =
    totalTtc <= STAMP_BAND_LOW
      ? STAMP_RATE_LOW
      : totalTtc <= STAMP_BAND_MID
        ? STAMP_RATE_MID
        : STAMP_RATE_HIGH;
  return Math.max(safe(tranches * BigInt(rate)), STAMP_MIN);
}

/**
 * A line's gross amount: unitPrice x qtyMilli / 1000, rounded once to the
 * centime, half away from zero, before the line discount comes off. BigInt
 * keeps the product exact past Number.MAX_SAFE_INTEGER.
 * line_total_fractional_qty
 */
export function lineTotal(unitPrice: number, qtyMilli: number): number {
  const per = BigInt(MILLI_PER_UNIT);
  const raw = exact(unitPrice) * exact(qtyMilli);
  const sign = raw < 0n ? -1n : 1n;
  return safe(sign * ((raw * sign + per / 2n) / per));
}

interface Group {
  rateBps: number;
  ht: number;
}

/** HT per rate group, by rising rate, and the sum of the groups. */
function groupByRate(lines: readonly TotalsLine[]): { groups: Group[]; totalHt: number } {
  const groups: Group[] = [];
  let totalHt = 0;
  for (const l of lines) {
    if (l.qtyMilli < 0) throw new MoneyError("NegativeQuantity");
    if (l.unitPrice < 0) throw new MoneyError("NegativeUnitPrice");
    if (l.lineDiscount < 0) throw new MoneyError("NegativeDiscount");
    const gross = lineTotal(l.unitPrice, l.qtyMilli);
    if (l.lineDiscount > gross) throw new MoneyError("LineDiscountAboveLine");
    const net = gross - l.lineDiscount;
    totalHt += net;
    const g = groups.find((x) => x.rateBps === l.rateBps);
    if (g !== undefined) g.ht += net;
    else groups.push({ rateBps: l.rateBps, ht: net });
  }
  groups.sort((a, b) => a.rateBps - b.rateBps);
  return { groups, totalHt };
}

/**
 * The global discount each group carries: its proportional share rounded
 * down, the leftover centimes going to the group with the largest HT
 * subtotal, the lower rate winning a tie, never past what the group has
 * left. discount_spread_largest_remainder
 */
function spreadDiscount(groups: readonly Group[], totalHt: number, discount: number): number[] {
  const shares = groups.map(() => 0);
  if (discount === 0 || groups.length === 0) return shares;
  // BigInt keeps discount x ht exact past Number.MAX_SAFE_INTEGER.
  const total = exact(totalHt);
  let allocated = 0;
  groups.forEach((g, i) => {
    shares[i] = safe((exact(discount) * exact(g.ht)) / total);
    allocated += shares[i];
  });
  let remainder = discount - allocated;
  // A share never exceeds its group's HT: when the discount leaves fewer
  // centimes than there are groups the largest one may have no room, and
  // what it cannot take rolls to the next largest. Groups rise by rate and
  // the sort is stable, so the lower rate wins a tie.
  const bySize = groups.map((_, i) => i).sort((a, b) => groups[b].ht - groups[a].ht);
  for (const i of bySize) {
    if (remainder === 0) break;
    const taken = Math.min(groups[i].ht - shares[i], remainder);
    shares[i] += taken;
    remainder -= taken;
  }
  return shares;
}

/**
 * Every column of the totals table in docs/features.md §3. The input is not
 * modified: the screen keeps its cart and gets a fresh answer.
 */
export function computeTotals(lines: readonly TotalsLine[], opts: TotalsOptions): Totals {
  const { groups, totalHt } = groupByRate(lines);
  const discount = opts.globalDiscount;
  if (discount < 0) throw new MoneyError("NegativeDiscount");
  if (discount > totalHt) throw new MoneyError("GlobalDiscountAboveTotal");
  const subtotalHt = totalHt - discount;

  // Under the IFU the unit price is the single price: no TVA row, no TVA.
  // regime_ifu_prints_no_tva
  const tvaByRate: TvaGroup[] = [];
  let tva = 0;
  if (opts.regime === "reel") {
    const shares = spreadDiscount(groups, totalHt, discount);
    groups.forEach((g, i) => {
      const base = g.ht - shares[i];
      const amount = pct(base, g.rateBps);
      tva += amount;
      tvaByRate.push({ rateBps: g.rateBps, base, amount });
    });
  }

  const totalTtc = subtotalHt + tva;
  const due = opts.stampEnabled ? stamp(totalTtc, opts.paymentMode) : 0;
  return {
    totalHt,
    discount,
    subtotalHt,
    tvaByRate,
    tva,
    totalTtc,
    stamp: due,
    netToPay: totalTtc + due,
  };
}
