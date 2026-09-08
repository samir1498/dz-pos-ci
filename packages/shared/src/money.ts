// Money is integer centimes everywhere (architecture.md rule 6). These are
// display and input helpers only: no arithmetic here reaches a total, and
// none of it uses a float. Totals, TVA and the stamp live in crates/core.

const CENTIMES_PER_DINAR = 100;

/** "1234" centimes becomes "12,34". Negative amounts keep the sign. */
export function formatCentimes(centimes: number): string {
  if (!Number.isSafeInteger(centimes)) {
    throw new RangeError(`centimes must be a safe integer, got ${centimes}`);
  }
  const negative = centimes < 0;
  const abs = Math.abs(centimes);
  const dinars = Math.trunc(abs / CENTIMES_PER_DINAR);
  const rest = abs % CENTIMES_PER_DINAR;
  const grouped = String(dinars).replace(/\B(?=(\d{3})+(?!\d))/g, " ");
  return `${negative ? "-" : ""}${grouped},${String(rest).padStart(2, "0")}`;
}

/** Thousandths of a unit become a readable quantity: 24000 is "24". */
export function formatQty(milli: number): string {
  if (!Number.isSafeInteger(milli)) {
    throw new RangeError(`quantity must be a safe integer, got ${milli}`);
  }
  const negative = milli < 0;
  const abs = Math.abs(milli);
  const whole = Math.trunc(abs / 1000);
  const rest = abs % 1000;
  const sign = negative ? "-" : "";
  if (rest === 0) return `${sign}${whole}`;
  return `${sign}${whole},${String(rest).padStart(3, "0").replace(/0+$/, "")}`;
}

/**
 * "12,34" or "12.34" becomes 1234 centimes. Parsed digit by digit: a
 * `parseFloat` here would put a float on the path to a stored price.
 * Returns null when the text is not an amount.
 */
export function parseAmountToCentimes(text: string): number | null {
  const cleaned = text.trim().replace(/[\s  ]/g, "");
  if (cleaned === "") return null;
  const match = /^(-?)(\d*)(?:[.,](\d{0,2}))?$/.exec(cleaned);
  if (match === null) return null;
  const [, sign, whole, fraction] = match;
  if (whole === "" && (fraction === undefined || fraction === "")) return null;
  const dinars = whole === "" ? 0 : Number(whole);
  const centimes = Number((fraction ?? "").padEnd(2, "0") || "0");
  const total = dinars * CENTIMES_PER_DINAR + centimes;
  if (!Number.isSafeInteger(total)) return null;
  return sign === "-" ? -total : total;
}
