export function Card({ children, className = "" }: { children: React.ReactNode; className?: string }) {
  return (
    <div
      className={`rounded-lg p-[18px_18px_14px] transition-all duration-200 hover:border-strong ${className}`}
      style={{
        background: "var(--color-surface)",
        border: "1px solid var(--color-border)",
      }}
    >
      {children}
    </div>
  );
}

export function CardRow({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="flex items-center justify-between gap-3 px-[18px] py-[11px]"
      style={{ borderBottom: "1px solid var(--color-border)" }}
    >
      {children}
    </div>
  );
}
