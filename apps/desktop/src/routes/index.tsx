import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { getDashboardData, getLowStockProducts } from "@/lib/api";
import { fmtDA, today } from "@/lib/utils";
import { Card, PageHeader } from "@/components/ui";
import { useTranslation } from "@/i18n";

export const Route = createFileRoute("/")({
  component: DashboardPage,
});

function DashboardPage() {
  const { t } = useTranslation();
  const date = today();

  const { data: dash, isLoading } = useQuery({
    queryKey: ["dashboard", date],
    queryFn: () => getDashboardData(date),
  });

  const { data: lowStock } = useQuery({
    queryKey: ["low-stock"],
    queryFn: () => getLowStockProducts(),
  });

  if (isLoading) {
    return (
      <div className="animate-pulse flex items-center gap-3 mt-8 justify-center" style={{ color: "var(--color-text-sec)" }}>
        <div className="w-4 h-4 rounded-full border-2 border-border-strong border-t-text-sec animate-spin" />
        {t("loading")}
      </div>
    );
  }

  return (
    <div className="animate-in">
      <PageHeader
        title={t("dashboard")}
        subtitle={new Date().toLocaleDateString("fr-DZ", { weekday: "long", year: "numeric", month: "long", day: "numeric" })}
      />

      {lowStock && lowStock.length > 0 && (
        <div
          className="flex items-center gap-2.5 px-4 py-3 rounded-lg mb-5 cursor-pointer transition-all hover:border-strong"
          style={{
            background: "var(--color-raised)",
            border: "1px solid var(--color-border)",
          }}
        >
          <div
            className="w-6 h-6 rounded-sm flex items-center justify-center text-[10px] font-mono"
            style={{
              background: "var(--color-surface)",
              border: "1px solid var(--color-border)",
              color: "var(--color-warn)",
            }}
          >
            !
          </div>
          <span className="text-[12.5px] font-medium" style={{ color: "var(--color-text)" }}>
            {lowStock.length} {t("low_stock_alert")}
          </span>
          <span className="ml-auto text-[11.5px]" style={{ color: "var(--color-text-dim)" }}>
            {t("view_inventory")}
          </span>
        </div>
      )}

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

      {dash && dash.transaction_count > 0 && (
        <Card>
          <div className="flex items-center justify-between">
            <span className="text-[11px] uppercase tracking-wider font-medium" style={{ color: "var(--color-text-dim)" }}>
              {t("details")}
            </span>
            <span className="text-[11px]" style={{ color: "var(--color-text-dim)" }}>
              {t("revenue")}: {fmtDA(dash.sales_total)} / {t("cost_price")}: {fmtDA(dash.cost_total)}
            </span>
          </div>
        </Card>
      )}
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
