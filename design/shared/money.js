// Money in integer centimes. First draft of packages/shared.
// Rules mirror docs/features.md "Fiscal rules"; fixture names in comments.

export const STAMP_RATE_PCT = 1; // stamp_cash_only_clamped
export const STAMP_MIN = 500; // 5 DZD
export const STAMP_MAX = 250_000; // 2 500 DZD

// Half away from zero on an integer × percent. tva_rounding_once_per_rate
export function pct(amount, ratePct) {
  const raw = amount * ratePct;
  const sign = raw < 0 ? -1 : 1;
  return sign * Math.floor((Math.abs(raw) + 50) / 100);
}

export function clamp(v, lo, hi) {
  return Math.min(hi, Math.max(lo, v));
}

/**
 * lines: [{ qty, unitPrice, lineDiscount, tvaRate }]
 * opts: { globalDiscount, paymentMode, stampEnabled }
 */
export function computeTotals(lines, opts) {
  const byRate = new Map();
  let totalHt = 0;
  for (const l of lines) {
    const lineHt = l.qty * l.unitPrice - (l.lineDiscount || 0);
    totalHt += lineHt;
    byRate.set(l.tvaRate, (byRate.get(l.tvaRate) || 0) + lineHt);
  }
  const discount = clamp(opts.globalDiscount || 0, 0, totalHt);
  // Global discount spread across rate groups proportionally, remainder to the largest.
  let allocated = 0;
  const subtotalByRate = [];
  const groups = [...byRate.entries()].sort((a, b) => b[1] - a[1]);
  groups.forEach(([rate, ht], i) => {
    let share = totalHt === 0 ? 0 : Math.floor((discount * ht) / totalHt);
    if (i === groups.length - 1) share = discount - allocated;
    allocated += share;
    subtotalByRate.push({ rate, subtotal: ht - share });
  });
  const subtotalHt = totalHt - discount;
  let tva = 0;
  const tvaByRate = subtotalByRate.map(({ rate, subtotal }) => {
    const amount = pct(subtotal, rate);
    tva += amount;
    return { rate, base: subtotal, amount };
  });
  const totalTtc = subtotalHt + tva;
  const stamp =
    opts.stampEnabled && opts.paymentMode === "cash" && totalTtc > 0
      ? clamp(pct(totalTtc, STAMP_RATE_PCT), STAMP_MIN, STAMP_MAX)
      : 0;
  return {
    totalHt,
    discount,
    subtotalHt,
    tvaByRate,
    tva,
    totalTtc,
    stamp,
    netToPay: totalTtc + stamp,
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
