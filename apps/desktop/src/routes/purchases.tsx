// The purchases screen: what the shop has ordered from its suppliers and
// where each order stands. Everything it shows comes from the API over HTTP.
//
// Nothing here deletes an order. An order nothing arrived against is
// cancelled and one the rest of which will never come is closed short, and
// both are on the order's own page because both ask for a reason.
//
// Stock and what the shop owes move when goods arrive, never when the paper
// is written (features.md §1): the list says which orders are still waiting.
//
// The state of an order is a `Badge` and not a `StatusPill`. The kit's pill
// carries five words of its own ("payé", "ouvert", …) and an order has five
// other ones; calling a received order "payé" would say something about the
// money that the screen does not know. The kit needs either those five states
// or a label a caller can hand it, and that is a kit change rather than a
// screen one.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { ClipboardList, Plus } from "lucide-react";
import { useState } from "react";
import type { PurchaseDto, PurchaseStatusDto, SupplierDto } from "@dzpos/shared";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { PageHeader } from "@/components/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { api, purchasesQueryKey, suppliersQueryKey } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

export const Route = createFileRoute("/purchases")({ component: PurchasesScreen });

/** Exported for the screen's own test, which mounts it under a memory router
 *  rather than through the file route. */
export { PurchasesScreen };

/** The five states, in the order an order moves through them. */
export const PURCHASE_STATES: readonly PurchaseStatusDto[] = [
  "ordered",
  "partially_received",
  "received",
  "closed_short",
  "cancelled",
];

export const PURCHASE_STATUS_KEY: Record<PurchaseStatusDto, Key> = {
  ordered: "purchase_status_ordered",
  partially_received: "purchase_status_partially_received",
  received: "purchase_status_received",
  cancelled: "purchase_status_cancelled",
  closed_short: "purchase_status_closed_short",
};

/** The tone each state wears. An order that ended badly is the loud one; the
 *  finished order is the filled one; the rest are quiet. */
const PURCHASE_STATUS_TONE: Record<
  PurchaseStatusDto,
  "default" | "secondary" | "destructive" | "outline"
> = {
  ordered: "outline",
  partially_received: "secondary",
  received: "default",
  closed_short: "secondary",
  cancelled: "destructive",
};

/**
 * The word a filter uses for "no filter at all". Radix refuses an item whose
 * value is the empty string, since that is how it spells "nothing chosen", so
 * the screen carries a word for it and turns it back into the blank the query
 * key and the API have always used.
 */
const ANY = "any";

/** Blank is every state; the filter sends the empty string for it. */
function chosenState(value: string): PurchaseStatusDto | undefined {
  return PURCHASE_STATES.find((state) => state === value);
}

export function PurchaseStatusBadge({
  status,
  "data-testid": testId,
}: {
  status: PurchaseStatusDto;
  "data-testid"?: string;
}) {
  const { t } = useTranslation();
  return (
    <Badge data-testid={testId} data-status={status} variant={PURCHASE_STATUS_TONE[status]}>
      {t(PURCHASE_STATUS_KEY[status])}
    </Badge>
  );
}

function PurchasesScreen() {
  const { t } = useTranslation();
  const [status, setStatus] = useState("");
  const [supplier, setSupplier] = useState("");
  const chosenStatus = chosenState(status);
  const chosenSupplier = supplier === "" ? undefined : Number(supplier);
  const purchases = useQuery({
    queryKey: [...purchasesQueryKey, status, supplier],
    queryFn: () =>
      api.listPurchases({ status: chosenStatus, supplierId: chosenSupplier }),
  });
  // The names beside the ids. The list answers a supplier id, because an
  // order names the fiche and not a copy of its name; the screen is where
  // the two are put together.
  const suppliers = useQuery({
    queryKey: [...suppliersQueryKey, ""],
    queryFn: () => api.listSuppliers(),
  });

  return (
    <section className="flex flex-col gap-4">
      <PageHeader
        title={t("purchases_title")}
        actions={
          <Button asChild>
            <Link to="/purchases/new">
              <Icon as={Plus} size={18} />
              {t("purchases_add")}
            </Link>
          </Button>
        }
      />

      <div className="flex flex-wrap items-end gap-4">
        <FormField label={t("purchases_filter_status")} className="min-w-48">
          {(parts) => (
            <Select
              value={status === "" ? ANY : status}
              onValueChange={(next) => setStatus(next === ANY ? "" : next)}
            >
              <SelectTrigger id={parts.id} className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={ANY}>{t("purchases_filter_any")}</SelectItem>
                {PURCHASE_STATES.map((state) => (
                  <SelectItem key={state} value={state}>
                    {t(PURCHASE_STATUS_KEY[state])}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </FormField>
        <FormField label={t("purchases_filter_supplier")} className="min-w-48">
          {(parts) => (
            <Select
              value={supplier === "" ? ANY : supplier}
              onValueChange={(next) => setSupplier(next === ANY ? "" : next)}
            >
              <SelectTrigger id={parts.id} className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={ANY}>{t("purchases_filter_any")}</SelectItem>
                {(suppliers.data ?? []).map((s) => (
                  <SelectItem key={s.id} value={String(s.id)}>
                    {s.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </FormField>
      </div>

      {purchases.isPending ? (
        <div className="flex flex-col gap-2">
          <span className="sr-only">{t("purchases_loading")}</span>
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : null}
      {purchases.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(purchases.error))}
        </p>
      ) : null}
      {purchases.isSuccess ? (
        <PurchaseTable rows={purchases.data} suppliers={suppliers.data ?? []} />
      ) : null}
    </section>
  );
}

function PurchaseTable({
  rows,
  suppliers,
}: {
  rows: PurchaseDto[];
  suppliers: SupplierDto[];
}) {
  const { t } = useTranslation();
  const nameOf = (supplierId: number) =>
    suppliers.find((s) => s.id === supplierId)?.name ?? "";

  const columns: readonly Column<PurchaseDto>[] = [
    {
      id: "date",
      header: t("col_date"),
      // A day is read left to right with Western digits whatever the screen's
      // language, the same decision Money takes for an amount.
      cell: (p) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {p.purchase_date}
        </span>
      ),
    },
    { id: "supplier", header: t("col_supplier"), cell: (p) => nameOf(p.supplier_id) },
    {
      id: "document",
      header: t("col_supplier_document"),
      cell: (p) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {p.supplier_document_number ?? ""}
        </span>
      ),
    },
    {
      id: "extra",
      header: t("col_extra_costs"),
      money: true,
      cell: (p) => <Money centimes={p.transport_centimes + p.extra_costs_centimes} />,
    },
    {
      id: "status",
      header: t("col_status"),
      cell: (p) => <PurchaseStatusBadge status={p.status} />,
    },
  ];

  return (
    <DataTable
      columns={columns}
      rows={rows}
      rowKey={(p) => p.id}
      caption={t("purchases_title")}
      empty={
        <EmptyState
          icon={ClipboardList}
          title={t("purchases_empty")}
          description={t("purchases_empty_hint")}
          action={
            <Button asChild>
              <Link to="/purchases/new">
                <Icon as={Plus} size={18} />
                {t("purchases_add")}
              </Link>
            </Button>
          }
        />
      }
      actions={(p) => (
        <Button asChild variant="ghost" size="sm">
          <Link
            to="/purchases/$id"
            params={{ id: String(p.id) }}
            aria-label={`${t("purchases_open")} ${p.purchase_date}`}
          >
            {t("purchases_open")}
          </Link>
        </Button>
      )}
    />
  );
}
