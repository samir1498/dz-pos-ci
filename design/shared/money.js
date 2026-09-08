// Money in integer centimes. First draft of packages/shared; the Rust core
// in crates/core/src/money is the reference and both are pinned by the same
// files under fixtures/money/, loaded here by money.test.js.
// Rules mirror docs/features.md "Fiscal rules"; fixture names in comments.

/** Basis points in one whole: 10 000 bps = 100 %. */
export const BPS_PER_WHOLE = 10_000;

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

/** Refused input, named like the Rust MoneyError variant. */
export class MoneyError extends Error {
  constructor(variant) {
    super(variant);
    this.name = "MoneyError";
    this.variant = variant;
  }
}

/** amount × rate, rounded once to the centime, half away from zero.
 *  tva_rounding_once_per_rate */
export function pct(amount, rateBps) {
  if (rateBps < 0 || rateBps > BPS_PER_WHOLE) throw new MoneyError("RateOutOfRange");
  const raw = amount * rateBps;
  const sign = raw < 0 ? -1 : 1;
  return sign * Math.floor((Math.abs(raw) + BPS_PER_WHOLE / 2) / BPS_PER_WHOLE);
}

/** Droit de timbre on total_ttc. Cash only; zero at or under 300,00 DA;
 *  ceil(amount / 100,00 DA) tranches at the band rate of the whole amount;
 *  minimum 5,00 DA; no cap. stamp_progressive_tranches */
export function stamp(totalTtc, mode) {
  if (mode !== "cash" || totalTtc <= STAMP_FLOOR) return 0;
  const tranches = Math.ceil(totalTtc / STAMP_TRANCHE);
  const rate =
    totalTtc <= STAMP_BAND_LOW
      ? STAMP_RATE_LOW
      : totalTtc <= STAMP_BAND_MID
        ? STAMP_RATE_MID
        : STAMP_RATE_HIGH;
  return Math.max(tranches * rate, STAMP_MIN);
}

/** HT per rate group, by rising rate, and the sum of the groups. */
function groupByRate(lines) {
  const groups = [];
  let totalHt = 0;
  for (const l of lines) {
    const lineDiscount = l.lineDiscount || 0;
    if (l.qty < 0) throw new MoneyError("NegativeQuantity");
    if (l.unitPrice < 0) throw new MoneyError("NegativeUnitPrice");
    if (lineDiscount < 0) throw new MoneyError("NegativeDiscount");
    const gross = l.qty * l.unitPrice;
    if (lineDiscount > gross) throw new MoneyError("LineDiscountAboveLine");
    const net = gross - lineDiscount;
    totalHt += net;
    const g = groups.find((x) => x.rateBps === l.rateBps);
    if (g) g.ht += net;
    else groups.push({ rateBps: l.rateBps, ht: net });
  }
  groups.sort((a, b) => a.rateBps - b.rateBps);
  return { groups, totalHt };
}

/** The global discount each group carries: its proportional share rounded
 *  down, the leftover centimes all going to the group with the largest HT
 *  subtotal, the lower rate winning a tie.
 *  discount_spread_largest_remainder */
function spreadDiscount(groups, totalHt, discount) {
  const shares = groups.map(() => 0);
  if (discount === 0 || groups.length === 0) return shares;
  // BigInt keeps discount × ht exact past Number.MAX_SAFE_INTEGER.
  const total = BigInt(totalHt);
  let allocated = 0;
  groups.forEach((g, i) => {
    shares[i] = Number((BigInt(discount) * BigInt(g.ht)) / total);
    allocated += shares[i];
  });
  const remainder = discount - allocated;
  if (remainder !== 0) {
    // Groups rise by rate, so the first strict maximum is the largest HT at
    // the lowest rate.
    let largest = 0;
    groups.forEach((g, i) => {
      if (g.ht > groups[largest].ht) largest = i;
    });
    shares[largest] += remainder;
  }
  return shares;
}

/**
 * Every column of the totals table in docs/features.md §3.
 * lines: [{ qty, unitPrice, lineDiscount, rateBps }]
 * opts: { globalDiscount, paymentMode, stampEnabled, regime }
 */
export function computeTotals(lines, opts) {
  const { groups, totalHt } = groupByRate(lines);
  const discount = opts.globalDiscount || 0;
  if (discount < 0) throw new MoneyError("NegativeDiscount");
  if (discount > totalHt) throw new MoneyError("GlobalDiscountAboveTotal");
  const subtotalHt = totalHt - discount;

  // Under the IFU the unit price is the single price: no TVA row, no TVA.
  // regime_ifu_prints_no_tva
  const tvaByRate = [];
  let tva = 0;
  if ((opts.regime || "reel") === "reel") {
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

const LOCALE = { fr: "fr-DZ", ar: "ar-DZ", en: "en-DZ" };
const UNIT = { fr: "DA", ar: "د.ج", en: "DZD" };

export function fmt(centimes, lang = "fr") {
  const n = (centimes / 100).toLocaleString(LOCALE[lang] || "fr-DZ", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
    numberingSystem: "latn",
  });
  return `${n} ${UNIT[lang] || "DA"}`;
}

export function fmtInt(centimes, lang = "fr") {
  return (centimes / 100).toLocaleString(LOCALE[lang] || "fr-DZ", {
    maximumFractionDigits: 0,
    numberingSystem: "latn",
  });
}

// ---- amount in words: fr and en implemented; ar is a static placeholder
// in the mockup (words_ar_golden pending). ----
const FR_U = [
  "zéro", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf",
  "dix", "onze", "douze", "treize", "quatorze", "quinze", "seize",
  "dix-sept", "dix-huit", "dix-neuf",
];
const FR_T = ["", "", "vingt", "trente", "quarante", "cinquante", "soixante", "soixante", "quatre-vingt", "quatre-vingt"];

function fr999(n) {
  let out = [];
  const h = Math.floor(n / 100);
  const r = n % 100;
  if (h) out.push(h === 1 ? "cent" : `${FR_U[h]} cent${r === 0 ? "s" : ""}`);
  if (r) {
    if (r < 20) out.push(FR_U[r]);
    else {
      const t = Math.floor(r / 10);
      const u = r % 10;
      if (t === 7 || t === 9) {
        out.push(`${FR_T[t]}${u === 1 && t === 7 ? " et " : "-"}${FR_U[10 + u]}`);
      } else {
        let s = FR_T[t];
        if (u === 1 && t !== 8) s += " et un";
        else if (u) s += `-${FR_U[u]}`;
        else if (t === 8) s += "s";
        out.push(s);
      }
    }
  }
  return out.join(" ");
}

function frWords(n) {
  if (n === 0) return "zéro";
  const parts = [];
  const scales = [
    [1_000_000_000, "milliard", "milliards"],
    [1_000_000, "million", "millions"],
    [1000, "mille", "mille"],
  ];
  for (const [v, s1, sN] of scales) {
    const q = Math.floor(n / v);
    if (q) {
      parts.push(q === 1 && v === 1000 ? s1 : `${fr999(q)} ${q > 1 ? sN : s1}`);
      n %= v;
    }
  }
  if (n) parts.push(fr999(n));
  return parts.join(" ");
}

const EN_U = [
  "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
  "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
  "seventeen", "eighteen", "nineteen",
];
const EN_T = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];

function en999(n) {
  const out = [];
  const h = Math.floor(n / 100);
  const r = n % 100;
  if (h) out.push(`${EN_U[h]} hundred`);
  if (r < 20 && r) out.push(EN_U[r]);
  else if (r) out.push(`${EN_T[Math.floor(r / 10)]}${r % 10 ? "-" + EN_U[r % 10] : ""}`);
  return out.join(" ");
}

function enWords(n) {
  if (n === 0) return "zero";
  const parts = [];
  for (const [v, s] of [[1_000_000_000, "billion"], [1_000_000, "million"], [1000, "thousand"]]) {
    const q = Math.floor(n / v);
    if (q) {
      parts.push(`${en999(q)} ${s}`);
      n %= v;
    }
  }
  if (n) parts.push(en999(n));
  return parts.join(" ");
}

export function amountInWords(centimes, lang) {
  const dinars = Math.floor(centimes / 100);
  const cents = centimes % 100;
  if (lang === "fr") {
    const d = `${frWords(dinars)} dinar${dinars > 1 ? "s" : ""}`;
    return cents ? `${d} et ${frWords(cents)} centime${cents > 1 ? "s" : ""}` : d;
  }
  if (lang === "en") {
    const d = `${enWords(dinars)} dinar${dinars > 1 ? "s" : ""}`;
    return cents ? `${d} and ${enWords(cents)} centime${cents > 1 ? "s" : ""}` : d;
  }
  return "— المبلغ بالحروف (ثابت في النموذج) —";
}
