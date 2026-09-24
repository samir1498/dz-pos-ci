// The suppliers screen: who the shop buys from, what it owes each of them,
// and the movements behind that figure. Everything it shows comes from the
// API over HTTP and the balance is the core's, never added up here.
//
// The list is the whole screen. A row's name opens that supplier's statement
// (`/suppliers/{id}`), which lives in `-suppliers/fiche.tsx` beside the panel
// that slides in from either page, so the fiche cannot read one way on the
// list and another on the statement.

import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { FilePlus2, SquarePen, Truck } from "lucide-react";
import { useState } from "react";
import type { SupplierDto } from "@dzpos/shared";
import { api, suppliersQueryKey } from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { PageHeader } from "@/components/PageHeader";
import { StatusPill } from "@/components/StatusPill";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useTranslation } from "@/i18n";
import { errorKey, positive } from "@/lib/fields";

import { SupplierSheet } from "./-suppliers/fiche";
import { balanceStatus } from "./-suppliers/parts";

export const Route = createFileRoute("/suppliers")({ component: SuppliersScreen });

export function SuppliersScreen() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  // One panel, two jobs: "new" is the blank fiche, a supplier is that
  // supplier's own.
  const [open, setOpen] = useState<"new" | SupplierDto | null>(null);
  const suppliers = useQuery({
    queryKey: [...suppliersQueryKey, search.trim()],
    queryFn: () => api.listSuppliers(search),
  });

  // The panel reads the row from the list, so it shows the balance the last
  // answer carried rather than the one it was opened with.
  const opened =
    open === null || open === "new"
      ? open
      : (suppliers.data?.find((s) => s.id === open.id) ?? open);

  const columns: readonly Column<SupplierDto>[] = [
    {
      id: "name",
      header: t("col_name"),
      cell: (s) => (
        <Link
          to="/suppliers/$id"
          params={{ id: String(s.id) }}
          className="font-medium underline-offset-4 hover:underline"
        >
          {s.name}
        </Link>
      ),
    },
    {
      id: "phone",
      header: t("col_phone"),
      // dir="ltr" on the number itself, not on the cell: a phone is read left
      // to right with Western digits whatever the screen's language, and
      // without it the bidi algorithm is free to reorder the groups.
      cell: (s) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {s.phone ?? ""}
        </span>
      ),
    },
    {
      id: "owed",
      header: t("col_owed"),
      money: true,
      cell: (s) => <Money centimes={positive(s.balance_centimes)} />,
    },
    {
      id: "state",
      header: t("col_status"),
      cell: (s) => (
        <div className="flex flex-wrap items-center gap-1.5">
          <StatusPill status={balanceStatus(s.balance_centimes)} />
          {s.balance_centimes < 0 ? (
            <Badge variant="secondary">{t("suppliers_advance")}</Badge>
          ) : null}
          {s.active ? null : <Badge variant="outline">{t("suppliers_inactive")}</Badge>}
        </div>
      ),
    },
  ];

  const addButton = (
    <Button onClick={() => setOpen("new")}>
      <Icon as={FilePlus2} size={18} />
      {t("suppliers_add")}
    </Button>
  );

  return (
    <div className="flex flex-col gap-4">
      <PageHeader title={t("suppliers_title")} actions={addButton} />

      <FormField label={t("suppliers_search")} className="max-w-sm">
        {(parts) => (
          <Input
            {...parts}
            type="search"
            data-testid="suppliers-search"
            placeholder={t("suppliers_search_hint")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        )}
      </FormField>

      {suppliers.isPending ? (
        <p className="text-sm text-muted-foreground">{t("suppliers_loading")}</p>
      ) : null}
      {suppliers.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(suppliers.error))}
        </p>
      ) : null}
      {suppliers.isSuccess ? (
        <DataTable
          data-testid="suppliers-table"
          caption={t("suppliers_title")}
          columns={columns}
          rows={suppliers.data}
          rowKey={(s) => s.id}
          // The whole row opens the account, as on the customers list (T32).
          onRowOpen={(s) => void navigate({ to: "/suppliers/$id", params: { id: String(s.id) } })}
          empty={
            <EmptyState
              icon={Truck}
              title={t("suppliers_empty")}
              description={t("suppliers_empty_hint")}
              action={addButton}
            />
          }
          actions={(s) => (
            <Button
              variant="ghost"
              size="icon"
              aria-label={`${t("suppliers_edit")} ${s.name}`}
              onClick={() => setOpen(s)}
            >
              <Icon as={SquarePen} size={18} />
            </Button>
          )}
        />
      ) : null}

      <SupplierSheet
        supplier={opened === "new" ? null : opened}
        open={opened !== null}
        onOpenChange={(next) => {
          if (!next) setOpen(null);
        }}
      />
    </div>
  );
}
