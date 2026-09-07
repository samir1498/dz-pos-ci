export function FormField({
  label, children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-start gap-3 mb-3">
      <label
        className="text-[13px] font-medium shrink-0 pt-[6px]"
        style={{ color: "var(--color-text)", width: 110 }}
      >
        {label}
      </label>
      <div className="flex-1 min-w-0">
        {children}
      </div>
    </div>
  );
}

export function Input({ className = "", ...props }: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={`w-full h-[32px] px-3 rounded-[6px] text-[13px] outline-none
        placeholder:text-text-dim transition-colors
        hover:border-stronger focus:border-stronger ${className}`}
      style={{
        background: "var(--color-raised)",
        border: "1px solid var(--color-border)",
        color: "var(--color-text)",
      }}
      {...props}
    />
  );
}

export function Select({ className = "", children, ...props }: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      className={`h-[32px] px-3 rounded-[6px] text-[13px] outline-none cursor-pointer
        hover:border-stronger focus:border-stronger ${className}`}
      style={{
        background: "var(--color-raised)",
        border: "1px solid var(--color-border)",
        color: "var(--color-text)",
      }}
      {...props}
    >
      {children}
    </select>
  );
}
