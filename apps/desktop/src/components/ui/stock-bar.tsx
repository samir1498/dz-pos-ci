export function StockBar({ qty, max = 50 }: { qty: number; max: number }) {
  const pct = max > 0 ? Math.min(qty / max, 1) : 0;
  const color = qty <= 0 ? "var(--color-bad)" : qty <= 5 ? "var(--color-warn)" : "var(--color-text-sec)";
  return (
    <div className="flex items-center gap-2">
      <div
        className="w-10 h-[3px] rounded-full overflow-hidden"
        style={{ background: "var(--color-raised)" }}
      >
        <div
          className="h-full rounded-full transition-all"
          style={{ width: `${pct * 100}%`, background: color }}
        />
      </div>
      <span className="font-mono text-[12px]" style={{ color: "var(--color-text)" }}>
        {qty}
      </span>
    </div>
  );
}
