import { useState } from "react";

export interface Column<T> {
  key: string;
  header: string;
  sortable?: boolean;
  width?: string;
  align?: "left" | "right";
  render: (item: T) => React.ReactNode;
}

export function DataTable<T extends { id: number }>({
  columns, data, isLoading, error, onRowClick,
}: {
  columns: Column<T>[];
  data: T[];
  isLoading?: boolean;
  error?: string | null;
  onRowClick?: (item: T) => void;
}) {
  const [sortCol, setSortCol] = useState<string | null>(null);
  const [sortAsc, setSortAsc] = useState(true);

  const sorted = [...data].sort((a, b) => {
    if (!sortCol) return 0;
    const col = columns.find(c => c.key === sortCol);
    if (!col || !col.sortable) return 0;
    const va = String(col.render(a) ?? "");
    const vb = String(col.render(b) ?? "");
    return sortAsc ? va.localeCompare(vb) : vb.localeCompare(va);
  });

  const handleSort = (key: string) => {
    if (sortCol === key) {
      setSortAsc(!sortAsc);
    } else {
      setSortCol(key);
      setSortAsc(true);
    }
  };

  if (error) {
    return (
      <div className="flex items-center gap-2 px-4 py-3 rounded-lg" style={{ background: "var(--color-bad)0f", border: "1px solid var(--color-bad)33", color: "var(--color-bad)" }}>
        <span className="text-[12px]">{error}</span>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="animate-pulse flex items-center gap-3 py-8 justify-center" style={{ color: "var(--color-text-sec)" }}>
        <div className="w-4 h-4 rounded-full border-2 border-border-strong border-t-text-sec animate-spin" />
        Loading...
      </div>
    );
  }

  if (sorted.length === 0) {
    return (
      <div className="flex items-center justify-center py-8 text-[12.5px]" style={{ color: "var(--color-text-dim)" }}>
        No data
      </div>
    );
  }

  return (
    <div className="rounded-lg overflow-hidden" style={{ background: "var(--color-surface)", border: "1px solid var(--color-border)" }}>
      <div className="overflow-x-auto">
        <table className="w-full text-[12.5px]">
          <thead>
            <tr>
              {columns.map((col) => (
                <th
                  key={col.key}
                  className={`px-4 py-2.5 text-[10.5px] uppercase tracking-wider font-medium whitespace-nowrap
                    ${col.align === "right" ? "text-right" : "text-left"}
                    ${col.sortable ? "cursor-pointer hover:text-text select-none" : ""}`}
                  style={{ color: sortCol === col.key ? "var(--color-text)" : "var(--color-text-dim)", borderBottom: "1px solid var(--color-border)" }}
                  onClick={() => col.sortable && handleSort(col.key)}
                  {...(col.width ? { style: { width: col.width, color: sortCol === col.key ? "var(--color-text)" : "var(--color-text-dim)", borderBottom: "1px solid var(--color-border)" } } : {})}
                >
                  {col.header}
                  {col.sortable && sortCol === col.key && (
                    <span className="ml-1 text-[10px]">{sortAsc ? "▲" : "▼"}</span>
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {sorted.map((item, idx) => (
              <tr
                key={item.id}
                className={`transition-colors ${onRowClick ? "cursor-pointer" : ""}`}
                style={{
                  background: idx % 2 === 1 ? "var(--color-raised)" : undefined,
                  borderBottom: "1px solid var(--color-border)",
                }}
                onClick={() => onRowClick?.(item)}
              >
                {columns.map((col) => (
                  <td
                    key={col.key}
                    className={`px-4 py-3 ${col.align === "right" ? "text-right" : "text-left"}`}
                  >
                    {col.render(item)}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
