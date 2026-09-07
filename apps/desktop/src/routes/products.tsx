import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { listProducts } from "@/lib/api";
import { fmtDA, marginPct } from "@/lib/utils";
import { PageHeader, Tag, StockBar, DataTable } from "@/components/ui";
import type { Column } from "@/components/ui";
import { useTranslation } from "@/i18n";
import type { Product } from "@/lib/api";

export const Route = createFileRoute("/products")({
  component: ProductsPage,
});

function ProductsPage() {
  const { t } = useTranslation();
  const { data, isLoading } = useQuery({
    queryKey: ["products"],
    queryFn: () => listProducts(1, 50),
  });

  const columns: Column<Product>[] = [
    {
      key: "name", header: t("product"), sortable: true,
      render: (p) => (
        <div className="flex items-center gap-2.5">
          <div
            className="w-7 h-7 rounded-sm flex items-center justify-center text-[11px] flex-shrink-0"
            style={{ background: "var(--color-raised)", border: "1px solid var(--color-border)" }}
          >
            {p.name.slice(0, 2).toUpperCase()}
          </div>
          <div>
            <div className="font-medium text-[12.5px] truncate max-w-[200px]" style={{ color: "var(--color-text)" }}>
              {p.name}
            </div>
            <div className="text-[11px] font-mono" style={{ color: "var(--color-text-dim)" }}>
              PRD-{String(p.id).padStart(3, "0")}
            </div>
          </div>
        </div>
      ),
    },
    {
      key: "barcode", header: t("barcode"),
      render: (p) => (
        <span className="font-mono text-[11.5px]" style={{ color: "var(--color-text-sec)" }}>
          {p.barcode ?? "—"}
        </span>
      ),
    },
    {
      key: "cost", header: t("cost_price"), align: "right",
      render: (p) => <span className="font-mono text-[12px]">{fmtDA(p.cost_price)}</span>,
    },
    {
      key: "price", header: t("selling_price"), align: "right",
      render: (p) => <span className="font-mono text-[12px]">{fmtDA(p.selling_price)}</span>,
    },
    {
      key: "stock", header: t("stock"),
      render: (p) => <StockBar qty={p.quantity_on_hand} max={50} />,
    },
    {
      key: "margin", header: t("margin"), align: "right",
      render: (p) => {
        const margin = marginPct(p.cost_price, p.selling_price);
        const mc = margin > 50 ? "good" : margin > 30 ? "warn" : "bad";
        return <Tag color={mc}>{margin.toFixed(1)}%</Tag>;
      },
    },
    {
      key: "status", header: t("status"),
      render: (p) => {
        const status = p.quantity_on_hand <= 0 ? t("out") : p.quantity_on_hand <= 10 ? t("low") : t("in_stock");
        const sc = status === t("in_stock") ? "good" : status === t("low") ? "warn" : "bad";
        return <Tag color={sc}>{status}</Tag>;
      },
    },
  ];

  return (
    <div className="animate-in">
      <PageHeader title={t("inventory")} subtitle={`${data?.total ?? 0} ${t("products_registered")}`} />
      <DataTable columns={columns} data={data?.items ?? []} isLoading={isLoading} />
    </div>
  );
}
