import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { listExpenses, createExpense, listExpenseCategories, deleteExpense } from "@/lib/api";
import { fmtDA, today } from "@/lib/utils";
import { PageHeader, Card, Button, Tag, DataTable, Pagination, Modal, FormField, Input } from "@/components/ui";
import type { Column } from "@/components/ui";
import { useTranslation } from "@/i18n";
import { useToast } from "@/components/toast";
import type { Expense } from "@/lib/api";

export const Route = createFileRoute("/expenses")({
  component: ExpensesPage,
});

function ExpensesPage() {
  const { t } = useTranslation();
  const { toast } = useToast();
  const qc = useQueryClient();
  const [startDate, setStartDate] = useState(today().slice(0, 7) + "-01");
  const [endDate, setEndDate] = useState(today());
  const [page, setPage] = useState(1);
  const [showModal, setShowModal] = useState(false);
  const [formDate, setFormDate] = useState(today());
  const [formCategory, setFormCategory] = useState("");
  const [formDescription, setFormDescription] = useState("");
  const [formAmount, setFormAmount] = useState("");

  const { data, isLoading } = useQuery({
    queryKey: ["expenses", startDate, endDate, page],
    queryFn: () => listExpenses(startDate, endDate, page, 20),
  });

  const { data: categories } = useQuery({
    queryKey: ["expense-categories"],
    queryFn: () => listExpenseCategories(),
  });

  const createMutation = useMutation({
    mutationFn: () => createExpense({
      date: formDate,
      category: formCategory,
      description: formDescription,
      amount: parseFloat(formAmount),
    }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["expenses"] });
      setShowModal(false);
      resetForm();
      toast(t("expense_created"), "success");
    },
    onError: (err) => toast(String(err), "error"),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => deleteExpense(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["expenses"] });
      toast(t("expense_deleted"), "success");
    },
    onError: (err) => toast(String(err), "error"),
  });

  const resetForm = () => {
    setFormDate(today());
    setFormCategory(categories?.[0]?.name ?? "");
    setFormDescription("");
    setFormAmount("");
  };

  const openAdd = () => {
    resetForm();
    setFormCategory(categories?.[0]?.name ?? "");
    setShowModal(true);
  };

  const columns: Column<Expense>[] = [
    {
      key: "date", header: t("date"), sortable: true,
      render: (e) => <span className="font-mono text-[12px]" style={{ color: "var(--color-text-sec)" }}>{e.date}</span>,
    },
    {
      key: "category", header: t("category"), sortable: true,
      render: (e) => <Tag color="accent">{e.description}</Tag>,
    },
    {
      key: "description", header: t("notes"),
      render: (e) => <span style={{ color: "var(--color-text-sec)" }}>{e.description || "—"}</span>,
    },
    {
      key: "amount", header: t("amount"), align: "right",
      render: (e) => <span className="font-mono text-[12px]" style={{ color: "var(--color-bad)" }}>{fmtDA(e.amount)}</span>,
    },
    {
      key: "actions", header: "", width: "40px",
      render: (e) => (
        <Button variant="ghost" size="sm" onClick={() => deleteMutation.mutate(e.id)}>
          ✕
        </Button>
      ),
    },
  ];

  return (
    <div className="animate-in">
      <PageHeader title={t("expenses")} />

      <Card>
        <div className="flex items-center gap-3 mb-4">
          <Input type="date" value={startDate} onChange={e => { setStartDate(e.target.value); setPage(1); }} className="w-[140px]" />
          <span style={{ color: "var(--color-text-dim)" }}>→</span>
          <Input type="date" value={endDate} onChange={e => { setEndDate(e.target.value); setPage(1); }} className="w-[140px]" />
          <div className="ml-auto">
            <Button variant="primary" onClick={openAdd}>+ {t("add")}</Button>
          </div>
        </div>

        <DataTable columns={columns} data={data?.items ?? []} isLoading={isLoading} />
        {data && (
          <Pagination
            page={page}
            totalPages={Math.ceil((data.total) / 20)}
            total={data.total}
            perPage={20}
            onPageChange={setPage}
          />
        )}
      </Card>

      <Modal open={showModal} onClose={() => setShowModal(false)} title={`${t("add")} ${t("expenses")}`}>
        <FormField label={t("date")}>
          <Input type="date" value={formDate} onChange={e => setFormDate(e.target.value)} />
        </FormField>
        <FormField label={t("category")}>
          <select
            className="w-full h-[32px] px-3 rounded-[6px] text-[13px] outline-none cursor-pointer"
            style={{ background: "var(--color-raised)", border: "1px solid var(--color-border)", color: "var(--color-text)" }}
            value={formCategory}
            onChange={e => setFormCategory(e.target.value)}
          >
            {categories?.map(c => (
              <option key={c.id} value={c.name}>{c.name}</option>
            ))}
          </select>
        </FormField>
        <FormField label={t("notes")}>
          <Input value={formDescription} onChange={e => setFormDescription(e.target.value)} placeholder={t("notes")} />
        </FormField>
        <FormField label={t("amount")}>
          <Input
            type="number"
            value={formAmount}
            onChange={e => setFormAmount(e.target.value)}
            className="w-[120px]"
            placeholder="0"
          />
        </FormField>
        <div className="flex justify-end gap-2 mt-4">
          <Button variant="default" onClick={() => setShowModal(false)}>{t("cancel")}</Button>
          <Button
            variant="primary"
            onClick={() => createMutation.mutate()}
            disabled={!formAmount || parseFloat(formAmount) <= 0}
          >
            {t("save")}
          </Button>
        </div>
      </Modal>
    </div>
  );
}
