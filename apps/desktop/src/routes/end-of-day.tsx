import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { getDashboardData, listSales } from "@/lib/api";
import { fmtDA, today } from "@/lib/utils";
import { PageHeader, Card, DataTable, Pagination } from "@/components/ui";
import type { Column } from "@/components/ui";
import { useTranslation } from "@/i18n";
import type { Transaction } from "@/lib/api";

export const Route = createFileRoute("/end-of-day")({
  component: EndOfDayPage,
});

function EndOfDayPage() {
  const { t } = useTranslation();
  const date = today();
  const [page, setPage] = useState(1);

  const { data: dash } = useQuery({
    queryKey: ["dashboard", date],
    queryFn: () => getDashboardData(date),
  });

  const { data: salesData, isLoading } = useQuery({
    queryKey: ["end-of-day-sales", date, page],
    queryFn: () => listSales(date, page, 20),
  });

  const columns: Column<Transaction>[] = [
    {
      key: "id", header: "ID", width: "60px",
      render: (tx) => <span className="font-mono text-[12px]" style={{ color: "var(--color-text-sec)" }}>#{tx.id}</span>,
    },
    {
      key: "timestamp", header: t("date"), sortable: true,
      render: (tx) => {
        const time = tx.timestamp.includes("T") ? tx.timestamp.split("T")[1]?.slice(0, 5) : tx.timestamp;
        return <span className="font-mono text-[12px]" style={{ color: "var(--color-text-sec)" }}>{time}</span>;
      },
    },
    {
      key: "total", header: t("total"), align: "right",
      render: (tx) => <span className="font-mono text-[12px]">{fmtDA(tx.total)}</span>,
    },
  ];

  return (
    <div className="animate-in">
      <PageHeader title={t("end_of_day")} subtitle={date} />

      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-5">
        <StatCard label={t("sales_today_display")} value={dash ? fmtDA(dash.sales_total) : "—"} />
        <StatCard label={t("expenses_today_display")} value={dash ? fmtDA(dash.expenses_total) : "—"} />
        <StatCard
          label={t("net_profit")}
          value={dash ? fmtDA(dash.profit) : "—"}
          color={dash && dash.profit >= 0 ? "good" : "bad"}
        />
        <StatCard label={t("transactions")} value={dash ? String(dash.transaction_count) : "—"} />
      </div>

      <Card>
        <div className="text-[12px] font-medium mb-3" style={{ color: "var(--color-text-dim)" }}>
          {t("sales")}
        </div>
        <DataTable columns={columns} data={salesData?.items ?? []} isLoading={isLoading} />
        {salesData && (
          <Pagination page={page} totalPages={Math.ceil(salesData.total / 20)} total={salesData.total} perPage={20} onPageChange={setPage} />
        )}
      </Card>
    </div>
  );
}

function StatCard({ label, value, color }: { label: string; value: string; color?: "good" | "bad" }) {
  const textColor = color === "good" ? "var(--color-good)" : color === "bad" ? "var(--color-bad)" : "var(--color-text)";
  return (
    <Card>
      <div className="text-[11px] uppercase tracking-wider font-medium mb-2" style={{ color: "var(--color-text-dim)" }}>
        {label}
      </div>
      <div className="text-[22px] font-semibold tracking-tight" style={{ color: textColor }}>
        {value}
      </div>
    </Card>
  );
}
