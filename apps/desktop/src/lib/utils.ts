/** Format amount as Dinar */
export function fmtDA(amount: number): string {
  return new Intl.NumberFormat("fr-DZ", {
    style: "decimal",
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(amount) + " DA";
}

/** Format date for display */
export function fmtDate(dateStr: string): string {
  if (!dateStr) return "";
  const d = new Date(dateStr);
  return d.toLocaleDateString("fr-DZ", {
    day: "2-digit",
    month: "2-digit",
    year: "2-digit",
  });
}

/** Get today's date as YYYY-MM-DD */
export function today(): string {
  const d = new Date();
  return d.toISOString().slice(0, 10);
}

/** Clamp number between min and max */
export function clamp(val: number, min: number, max: number): number {
  return Math.min(Math.max(val, min), max);
}

/** Calculate margin percentage */
export function marginPct(cost: number, price: number): number {
  if (cost <= 0) return 0;
  return ((price - cost) / cost) * 100;
}