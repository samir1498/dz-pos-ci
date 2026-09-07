import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listProducts, updateProduct } from "@/lib/api";
import { PageHeader, Card, Input, Button, Tag } from "@/components/ui";
import { useTranslation } from "@/i18n";

export const Route = createFileRoute("/barcodes")({
  component: BarcodesPage,
});

function BarcodesPage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [editingId, setEditingId] = useState<number | null>(null);
  const [barcodeValue, setBarcodeValue] = useState("");

  const { data, isLoading } = useQuery({
    queryKey: ["products", 1, 100],
    queryFn: () => listProducts(1, 100),
  });

  const updateMutation = useMutation({
    mutationFn: ({ id, barcode }: { id: number; barcode: string | null }) =>
      updateProduct(id, {
        name: "", barcode, cost_price: 0, selling_price: 0, quantity_on_hand: 0,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["products"] });
      setEditingId(null);
    },
  });

  const startEdit = (id: number, barcode: string | null) => {
    setEditingId(id);
    setBarcodeValue(barcode ?? "");
  };

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
      <PageHeader title={t("barcodes")} />

      <Card>
        <div className="overflow-x-auto">
          <table className="w-full text-[12.5px]">
            <thead>
              <tr>
                <Th>{t("product")}</Th>
                <Th>{t("barcode")}</Th>
                <Th>{t("selling_price")}</Th>
                <Th>{t("status")}</Th>
                <Th width="60px"> </Th>
              </tr>
            </thead>
            <tbody>
              {data?.items.map((p) => (
                <tr key={p.id} className="transition-colors" style={{ borderBottom: "1px solid var(--color-border)" }}>
                  <td className="px-4 py-3">
                    <div className="font-medium" style={{ color: "var(--color-text)" }}>{p.name}</div>
                    <div className="text-[11px] font-mono" style={{ color: "var(--color-text-dim)" }}>
                      PRD-{String(p.id).padStart(3, "0")}
                    </div>
                  </td>
                  <td className="px-4 py-3">
                    {editingId === p.id ? (
                      <div className="flex items-center gap-1">
                        <Input
                          value={barcodeValue}
                          onChange={e => setBarcodeValue(e.target.value)}
                          className="w-[140px]"
                          placeholder="Code-barres"
                          autoFocus
                        />
                        <Button size="sm" variant="primary" onClick={() => updateMutation.mutate({ id: p.id, barcode: barcodeValue || null })}>
                          OK
                        </Button>
                        <Button size="sm" variant="ghost" onClick={() => setEditingId(null)}>
                          {t("cancel")}
                        </Button>
                      </div>
                    ) : (
                      <span
                        className="font-mono text-[12px] cursor-pointer hover:text-text"
                        style={{ color: p.barcode ? "var(--color-text)" : "var(--color-text-dim)" }}
                        onClick={() => startEdit(p.id, p.barcode)}
                      >
                        {p.barcode || "—"}
                      </span>
                    )}
                  </td>
                  <td className="px-4 py-3 font-mono text-[12px]" style={{ color: "var(--color-text)" }}>
                    {p.selling_price.toFixed(0)} DA
                  </td>
                  <td className="px-4 py-3">
                    <Tag color={p.quantity_on_hand <= 0 ? "bad" : p.quantity_on_hand <= 10 ? "warn" : "good"}>
                      {p.quantity_on_hand <= 0 ? t("out") : p.quantity_on_hand <= 10 ? t("low") : t("in_stock")}
                    </Tag>
                  </td>
                  <td className="px-4 py-3">
                    <Button size="sm" variant="ghost" onClick={() => startEdit(p.id, p.barcode)}>
                      {t("edit")}
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}

function Th({ children, width }: { children?: React.ReactNode; width?: string }) {
  return (
    <th
      className="px-4 py-2.5 text-[10.5px] uppercase tracking-wider font-medium whitespace-nowrap text-left"
      style={{ color: "var(--color-text-dim)", borderBottom: "1px solid var(--color-border)", width }}
    >
      {children}
    </th>
  );
}
