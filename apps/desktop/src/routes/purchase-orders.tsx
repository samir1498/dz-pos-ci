import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listPurchaseOrders, createPurchaseOrder, receivePurchaseOrder, deletePurchaseOrder, listProducts } from "@/lib/api";
import { fmtDA } from "@/lib/utils";
import { PageHeader, Card, Button, Tag, DataTable, Pagination, Modal, FormField, Input } from "@/components/ui";
import type { Column } from "@/components/ui";
import { useTranslation } from "@/i18n";
import { useToast } from "@/components/toast";
import type { PurchaseOrder } from "@/lib/api";

export const Route = createFileRoute("/purchase-orders")({
  component: PurchaseOrdersPage,
});

function PurchaseOrdersPage() {
  const { t } = useTranslation();
  const { toast } = useToast();
  const qc = useQueryClient();
  const [page, setPage] = useState(1);
  const [showModal, setShowModal] = useState(false);
  const [formNotes, setFormNotes] = useState("");
  const [items, setItems] = useState([{ product_name: "", quantity: "1", unit_cost: "0" }]);

  const { data, isLoading } = useQuery({
    queryKey: ["purchase-orders", page],
    queryFn: () => listPurchaseOrders(page, 20),
  });

  const receiveMutation = useMutation({
    mutationFn: (id: number) => receivePurchaseOrder(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["purchase-orders"] });
      toast(t("po_received"), "success");
    },
    onError: (err) => toast(String(err), "error"),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => deletePurchaseOrder(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["purchase-orders"] });
      toast(t("po_deleted"), "success");
    },
    onError: (err) => toast(String(err), "error"),
  });

  const createMutation = useMutation({
    mutationFn: () => createPurchaseOrder({
      purchase_order_number: `PO-${Date.now()}`,
      supplier_id: null,
      notes: formNotes || null,
      items: items.map(i => ({
        product_name: i.product_name,
        quantity: parseInt(i.quantity) || 1,
        unit_cost: parseFloat(i.unit_cost) || 0,
      })),
    }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["purchase-orders"] });
      setShowModal(false);
      toast(t("po_created"), "success");
    },
    onError: (err) => toast(String(err), "error"),
  });

  const columns: Column<PurchaseOrder>[] = [
    {
      key: "number", header: "PO #", sortable: true,
      render: (po) => <span className="font-mono text-[12px]" style={{ color: "var(--color-text)" }}>{po.purchase_order_number}</span>,
    },
    {
      key: "status", header: t("status"),
      render: (po) => (
        <Tag color={po.status === "Received" ? "good" : "warn"}>
          {po.status === "Received" ? t("received") : t("draft")}
        </Tag>
      ),
    },
    {
      key: "total", header: t("total"), align: "right",
      render: (po) => <span className="font-mono text-[12px]">{fmtDA(po.total)}</span>,
    },
    {
      key: "date", header: t("date"),
      render: (po) => <span className="font-mono text-[11.5px]" style={{ color: "var(--color-text-sec)" }}>{po.created_at}</span>,
    },
    {
      key: "actions", header: "", width: "120px",
      render: (po) => (
        <div className="flex items-center gap-1">
          {po.status === "Draft" && (
            <Button size="sm" variant="default" onClick={() => receiveMutation.mutate(po.id)}>
              {t("received")}
            </Button>
          )}
          <Button variant="ghost" size="sm" onClick={() => deleteMutation.mutate(po.id)}>
            ✕
          </Button>
        </div>
      ),
    },
  ];

  return (
    <div className="animate-in">
      <PageHeader title={t("purchase_orders")} />

      <Card>
        <div className="flex justify-end mb-4">
          <Button variant="primary" onClick={() => { setShowModal(true); setFormNotes(""); setItems([{ product_name: "", quantity: "1", unit_cost: "0" }]); }}>
            + {t("add")}
          </Button>
        </div>

        <DataTable columns={columns} data={data?.items ?? []} isLoading={isLoading} />
        {data && (
          <Pagination page={page} totalPages={Math.ceil(data.total / 20)} total={data.total} perPage={20} onPageChange={setPage} />
        )}
      </Card>

      <Modal open={showModal} onClose={() => setShowModal(false)} title={`${t("add")} ${t("purchase_orders")}`}>
        <FormField label={t("notes")}>
          <Input value={formNotes} onChange={e => setFormNotes(e.target.value)} placeholder={t("notes")} />
        </FormField>

        <div className="mt-3 mb-2 text-[12px] font-medium" style={{ color: "var(--color-text-dim)" }}>
          {t("entered_items")}
        </div>
        {items.map((item, idx) => (
          <div key={idx} className="flex items-center gap-2 mb-2">
            <Input
              value={item.product_name}
              onChange={e => {
                const next = [...items];
                next[idx].product_name = e.target.value;
                setItems(next);
              }}
              placeholder={t("product")}
              className="flex-1"
            />
            <Input
              type="number" value={item.quantity}
              onChange={e => {
                const next = [...items];
                next[idx].quantity = e.target.value;
                setItems(next);
              }}
              className="w-[60px]"
              placeholder="Qty"
            />
            <Input
              type="number" value={item.unit_cost}
              onChange={e => {
                const next = [...items];
                next[idx].unit_cost = e.target.value;
                setItems(next);
              }}
              className="w-[80px]"
              placeholder="Cost"
            />
            <Button variant="ghost" size="sm" onClick={() => setItems(items.filter((_, i) => i !== idx))}>
              ✕
            </Button>
          </div>
        ))}
        <Button variant="default" size="sm" onClick={() => setItems([...items, { product_name: "", quantity: "1", unit_cost: "0" }])}>
          + {t("add")}
        </Button>

        <div className="flex justify-end gap-2 mt-4">
          <Button variant="default" onClick={() => setShowModal(false)}>{t("cancel")}</Button>
          <Button variant="primary" onClick={() => createMutation.mutate()} disabled={items.every(i => !i.product_name.trim())}>
            {t("create")}
          </Button>
        </div>
      </Modal>
    </div>
  );
}
