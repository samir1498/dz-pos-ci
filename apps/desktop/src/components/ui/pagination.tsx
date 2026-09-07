export function Pagination({
  page, totalPages, total, perPage, onPageChange,
}: {
  page: number;
  totalPages: number;
  total: number;
  perPage: number;
  onPageChange: (p: number) => void;
}) {
  if (total === 0 || (totalPages <= 1 && total <= perPage)) return null;

  const start = (page - 1) * perPage + 1;
  const end = Math.min(page * perPage, total);

  return (
    <div className="flex items-center justify-between mt-1 pt-3" style={{ borderTop: "1px solid var(--color-border)" }}>
      <span className="text-[13px] font-mono font-medium" style={{ color: "var(--color-text-sec)" }}>
        {start}–{end} / {total}
      </span>
      <div className="flex items-center gap-1">
        <PageButton disabled={page <= 1} onClick={() => onPageChange(page - 1)}>
          ‹
        </PageButton>
        <PageButton disabled={page >= totalPages} onClick={() => onPageChange(page + 1)}>
          ›
        </PageButton>
      </div>
    </div>
  );
}

function PageButton({ disabled, onClick, children }: { disabled: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      disabled={disabled}
      onClick={onClick}
      className="inline-flex items-center justify-center w-8 h-7 rounded-[4px] text-[16px] font-medium cursor-pointer
        hover:bg-raised transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
      style={{
        border: disabled ? "1px solid transparent" : "1px solid var(--color-border)",
        color: disabled ? "var(--color-text-dim)" : "var(--color-text-sec)",
      }}
    >
      {children}
    </button>
  );
}
