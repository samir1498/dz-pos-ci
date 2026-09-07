export function PageHeader({ title, subtitle }: { title: string; subtitle?: string }) {
  return (
    <div className="flex items-end justify-between mb-6 pb-4" style={{ borderBottom: "1px solid var(--color-border)" }}>
      <div>
        <h1 className="text-xl font-semibold tracking-tight" style={{ color: "var(--color-text)" }}>
          {title}
        </h1>
        {subtitle && (
          <p className="text-xs mt-1" style={{ color: "var(--color-text-dim)" }}>
            {subtitle}
          </p>
        )}
      </div>
    </div>
  );
}
