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

// ---- amount in words: fr and en follow crates/core/src/money/words.rs
// rule for rule, and design/money.test.js pins both to fixtures/money/
// words_{fr,en}_golden.json. Arabic stays a placeholder in the mockup until
// the native review (R6). ----

/** The largest amount in dinars the three scales cover: 999 999 999 999. */
const MAX_WORDS_DINARS = 999_999_999_999;

function groups(n) {
  return [
    Math.floor(n / 1_000_000_000),
    Math.floor(n / 1_000_000) % 1000,
    Math.floor(n / 1000) % 1000,
    n % 1000,
  ];
}

const FR_UNIT = ["", "un", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf"];
const FR_TEN_PLUS = [
  "dix", "onze", "douze", "treize", "quatorze", "quinze", "seize", "dix-sept", "dix-huit", "dix-neuf",
];
const FR_TENS = { 2: "vingt", 3: "trente", 4: "quarante", 5: "cinquante", 6: "soixante" };

// `pluralS` is false when a number word follows, which is what stops the
// `s` of `quatre-vingts` before `mille`.
function frUnder100(n, pluralS) {
  const tens = Math.floor(n / 10);
  const unit = n % 10;
  if (tens === 0) return FR_UNIT[unit];
  if (tens === 1) return FR_TEN_PLUS[unit];
  if (tens <= 6) {
    const base = FR_TENS[tens];
    if (unit === 0) return base;
    if (unit === 1) return `${base}-et-un`;
    return `${base}-${FR_UNIT[unit]}`;
  }
  if (tens === 7) {
    if (unit === 0) return "soixante-dix";
    if (unit === 1) return "soixante-et-onze";
    return `soixante-${FR_TEN_PLUS[unit]}`;
  }
  if (tens === 8) {
    if (unit === 0) return pluralS ? "quatre-vingts" : "quatre-vingt";
    return `quatre-vingt-${FR_UNIT[unit]}`;
  }
  return `quatre-vingt-${FR_TEN_PLUS[unit]}`;
}

function frGroup(g, pluralS) {
  const hundreds = Math.floor(g / 100);
  const rest = g % 100;
  let head = "";
  if (hundreds === 1) head = "cent";
  // `cent` takes the s only when it is multiplied and final.
  else if (hundreds > 1) head = `${FR_UNIT[hundreds]}-cent${rest === 0 && pluralS ? "s" : ""}`;
  if (rest === 0) return head;
  const tail = frUnder100(rest, pluralS);
  return hundreds === 0 ? tail : `${head}-${tail}`;
}

// The number in words, and whether it ends on `million` or `milliard`.
// Those two are nouns, so what they count takes `de`: `deux millions de
// dinars`, against `deux millions deux-cents dinars`.
function frNumber(n) {
  if (n === 0) return ["zéro", false];
  const [milliards, millions, thousands, units] = groups(n);
  const parts = [];
  if (milliards > 0) parts.push(`${frGroup(milliards, true)} milliard${milliards > 1 ? "s" : ""}`);
  if (millions > 0) parts.push(`${frGroup(millions, true)} million${millions > 1 ? "s" : ""}`);
  // `mille` is invariable and hyphenates onto the units group.
  const tail = [];
  if (thousands === 1) tail.push("mille");
  else if (thousands > 1) tail.push(`${frGroup(thousands, false)}-mille`);
  if (units > 0) tail.push(frGroup(units, true));
  const joined = tail.join("-");
  if (joined) parts.push(joined);
  return [parts.join(" "), joined === ""];
}

function fr(dinars, sub) {
  const [number, endsOnANoun] = frNumber(dinars);
  const unit = dinars <= 1 ? "dinar" : "dinars";
  let out = `${number} ${endsOnANoun ? "de " : ""}${unit}`;
  if (sub > 0) out += ` et ${frUnder100(sub, true)} ${sub === 1 ? "centime" : "centimes"}`;
  return out;
}

const EN_UNDER_20 = [
  "", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
  "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
];
const EN_TENS = { 2: "twenty", 3: "thirty", 4: "forty", 5: "fifty", 6: "sixty", 7: "seventy", 8: "eighty", 9: "ninety" };

function enUnder100(n) {
  if (n < 20) return EN_UNDER_20[n];
  const base = EN_TENS[Math.floor(n / 10)];
  const unit = n % 10;
  return unit === 0 ? base : `${base}-${EN_UNDER_20[unit]}`;
}

function enGroup(g) {
  const hundreds = Math.floor(g / 100);
  const rest = g % 100;
  if (hundreds === 0) return enUnder100(rest);
  if (rest === 0) return `${EN_UNDER_20[hundreds]} hundred`;
  return `${EN_UNDER_20[hundreds]} hundred and ${enUnder100(rest)}`;
}

function enNumber(n) {
  if (n === 0) return "zero";
  const [billions, millions, thousands, units] = groups(n);
  const parts = [];
  if (billions > 0) parts.push(`${enGroup(billions)} billion`);
  if (millions > 0) parts.push(`${enGroup(millions)} million`);
  if (thousands > 0) parts.push(`${enGroup(thousands)} thousand`);
  if (units > 0) {
    // British usage: "one thousand and one", "one thousand two hundred".
    if (parts.length > 0 && units < 100) parts.push(`and ${enUnder100(units)}`);
    else parts.push(enGroup(units));
  }
  return parts.join(" ");
}

function en(dinars, sub) {
  let out = `${enNumber(dinars)} ${dinars === 1 ? "dinar" : "dinars"}`;
  if (sub > 0) out += ` and ${enUnder100(sub)} ${sub === 1 ? "centime" : "centimes"}`;
  return out;
}

/** `net_to_pay` written out in `lang`, dinars and centimes. Throws the
 *  same two refusals as the core: a negative amount, and one past the
 *  scales it knows. */
export function amountInWords(centimes, lang) {
  if (!Number.isSafeInteger(centimes) || centimes < 0) throw new MoneyError("Negative");
  const dinars = Math.floor(centimes / 100);
  const sub = centimes % 100;
  if (dinars > MAX_WORDS_DINARS) throw new MoneyError("Overflow");
  if (lang === "fr") return fr(dinars, sub);
  if (lang === "en") return en(dinars, sub);
  return "— المبلغ بالحروف (ثابت في النموذج) —";
}
