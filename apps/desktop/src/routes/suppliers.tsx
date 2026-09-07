import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listSuppliers, createSupplier, updateSupplier, deleteSupplier } from "@/lib/api";
import { PageHeader, Card, Button, DataTable, Modal, FormField, Input } from "@/components/ui";
import type { Column } from "@/components/ui";
import { useTranslation } from "@/i18n";
import { useToast } from "@/components/toast";
import type { Supplier } from "@/lib/api";

export const Route = createFileRoute("/suppliers")({
  component: SuppliersPage,
});

function SuppliersPage() {
  const { t } = useTranslation();
  const { toast } = useToast();
  const qc = useQueryClient();
  const [showModal, setShowModal] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [formName, setFormName] = useState("");
  const [formPhone, setFormPhone] = useState("");
  const [formNotes, setFormNotes] = useState("");

  const { data, isLoading } = useQuery({
    queryKey: ["suppliers"],
    queryFn: () => listSuppliers(),
  });

  const createMutation = useMutation({
    mutationFn: () => createSupplier({ name: formName, phone: formPhone || null, notes: formNotes || null }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["suppliers"] }); closeModal(); toast(t("supplier_created"), "success"); },
    onError: (err) => toast(String(err), "error"),
  });

  const updateMutation = useMutation({
    mutationFn: () => updateSupplier(editingId!, { name: formName, phone: formPhone || null, notes: formNotes || null }),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["suppliers"] }); closeModal(); toast(t("supplier_updated"), "success"); },
    onError: (err) => toast(String(err), "error"),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => deleteSupplier(id),
    onSuccess: () => { qc.invalidateQueries({ queryKey: ["suppliers"] }); toast(t("supplier_deleted"), "success"); },
    onError: (err) => toast(String(err), "error"),
  });

  const openAdd = () => {
    setEditingId(null);
    setFormName(""); setFormPhone(""); setFormNotes("");
    setShowModal(true);
  };

  const openEdit = (s: Supplier) => {
    setEditingId(s.id);
    setFormName(s.name); setFormPhone(s.phone ?? ""); setFormNotes(s.notes ?? "");
    setShowModal(true);
  };

  const closeModal = () => { setShowModal(false); setEditingId(null); };

  const columns: Column<Supplier>[] = [
    {
      key: "name", header: t("name"), sortable: true,
      render: (s) => (
        <span className="font-medium text-[13px]" style={{ color: "var(--color-text)" }}>{s.name}</span>
      ),
    },
    {
      key: "phone", header: "Téléphone",
      render: (s) => <span className="font-mono text-[12px]" style={{ color: "var(--color-text-sec)" }}>{s.phone || "—"}</span>,
    },
    {
      key: "notes", header: t("notes"),
      render: (s) => <span style={{ color: "var(--color-text-sec)" }}>{s.notes || "—"}</span>,
    },
    {
      key: "actions", header: "", width: "100px",
      render: (s) => (
        <div className="flex items-center gap-1">
          <Button size="sm" variant="ghost" onClick={() => openEdit(s)}>{t("edit")}</Button>
          <Button variant="ghost" size="sm" onClick={() => deleteMutation.mutate(s.id)}>✕</Button>
        </div>
      ),
    },
  ];

  return (
    <div className="animate-in">
      <PageHeader title={t("suppliers")} />

      <Card>
        <div className="flex justify-end mb-4">
          <Button variant="primary" onClick={openAdd}>+ {t("add")}</Button>
        </div>
        <DataTable columns={columns} data={data ?? []} isLoading={isLoading} />
      </Card>

      <Modal open={showModal} onClose={closeModal} title={`${editingId ? t("edit") : t("add")} ${t("suppliers")}`}>
        <FormField label={t("name")}>
          <Input value={formName} onChange={e => setFormName(e.target.value)} placeholder={t("name")} />
        </FormField>
        <FormField label="Téléphone">
          <Input value={formPhone} onChange={e => setFormPhone(e.target.value)} placeholder="0550 00 00 00" />
        </FormField>
        <FormField label={t("notes")}>
          <Input value={formNotes} onChange={e => setFormNotes(e.target.value)} placeholder={t("notes")} />
        </FormField>
        <div className="flex justify-end gap-2 mt-4">
          <Button variant="default" onClick={closeModal}>{t("cancel")}</Button>
          <Button variant="primary" onClick={() => editingId ? updateMutation.mutate() : createMutation.mutate()} disabled={!formName.trim()}>
            {t("save")}
          </Button>
        </div>
      </Modal>
    </div>
  );
}
