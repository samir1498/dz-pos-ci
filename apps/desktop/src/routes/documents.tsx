// The documents screen: every numbered paper the shop has issued, and what
// can still be done to one.
//
// The till is where a document is made; this is where it is found again. A
// row opens the document itself, with the lines it was issued with, the
// totals a comptable reads and the balance block when it carries one, and
// the sheet the core renders in the sandboxed panel beside them. That detail
// card, and the two irreversible acts it offers, live in
// `-documents/detail.tsx`; this file is the list and the filter above it.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import type { DocumentKindDto, SaleDto, SaleKindDto } from "@dzpos/shared";
import { FileText } from "lucide-react";
import { useState } from "react";

import { api, salesQueryKey } from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { Money } from "@/components/Money";
import { PageHeader } from "@/components/PageHeader";
import { StatusPill } from "@/components/StatusPill";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useTranslation, type Key } from "@/i18n";

import { DocumentDetail } from "./-documents/detail";
import { Refusal, day } from "./-documents/parts";

/** The filter the list offers. `all` is no filter at all rather than a
 *  fourth value on the wire: the route reads a missing `kind` as every kind
 *  (crates/api/src/routes/sales.rs). */
type Filter = SaleKindDto | "all";

/** Every value the filter offers, and what each is called. A record keyed by
 *  the union, so a sale kind added to the API fails to compile here rather
 *  than quietly never being offered: an array of strings would have taken the
 *  new kind's absence in silence.
 *
 *  `Object.keys` types its answer as `string[]`, which loses exactly the fact
 *  this needs, so the list is narrowed back by a guard rather than asserted.
 *  Insertion order is the order they are offered in. */
const FILTER_KEY = {
  all: "documents_filter_all",
  ticket: "documents_kind_ticket",
  facture: "documents_kind_facture",
  proforma: "documents_kind_proforma",
} satisfies Record<Filter, Key>;

function isFilter(value: string): value is Filter {
  return value in FILTER_KEY;
}

const FILTERS: readonly Filter[] = Object.keys(FILTER_KEY).filter(isFilter);

const KIND_KEY: Record<DocumentKindDto, Key> = {
  ticket: "documents_kind_ticket",
  facture: "documents_kind_facture",
  proforma: "documents_kind_proforma",
  avoir: "documents_kind_avoir",
  bon_de_livraison: "documents_kind_bon_de_livraison",
  bon_de_reception: "documents_kind_bon_de_reception",
  quittance: "documents_kind_quittance",
};

export function DocumentsScreen() {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<Filter>("all");
  const [openId, setOpenId] = useState<number | null>(null);
  const kind = filter === "all" ? undefined : filter;
  const documents = useQuery({
    queryKey: salesQueryKey(kind),
    queryFn: () => api.listSales(kind),
  });

  const columns: readonly Column<SaleDto>[] = [
    {
      id: "number",
      header: t("documents_number"),
      numeric: true,
      cell: (d) => (
        <Button
          variant="link"
          size="sm"
          className="px-0 font-numeric tabular-nums"
          onClick={() => setOpenId(d.id === openId ? null : d.id)}
        >
          {d.printed_number}
        </Button>
      ),
    },
    {
      id: "date",
      header: t("documents_date"),
      numeric: true,
      cell: (d) => <span dir="ltr">{day(d.issued_at)}</span>,
    },
    {
      id: "kind",
      header: t("documents_kind"),
      cell: (d) => <Badge variant="outline">{t(KIND_KEY[d.kind])}</Badge>,
    },
    {
      id: "customer",
      header: t("documents_customer"),
      // The buyer's name is on the document, snapshotted at issue: a reprint
      // has to show the block the customer was handed, so the fiche is never
      // read live for it. The link beside it goes to the fiche as it stands
      // today, which is where a shop goes next from a document: to what the
      // customer still owes. A ticket sold to whoever walked in names nobody
      // and gets no link.
      cell: (d) =>
        d.customer_id === null ? (
          (d.buyer_name ?? "")
        ) : (
          <Link
            to="/customers/$id"
            params={{ id: String(d.customer_id) }}
            className="text-primary underline-offset-4 hover:underline"
          >
            {d.buyer_name ?? ""}
          </Link>
        ),
    },
    {
      id: "net",
      header: t("documents_net"),
      money: true,
      cell: (d) => <Money centimes={d.totals.net_to_pay_centimes} />,
    },
    {
      id: "status",
      header: t("documents_status"),
      cell: (d) => <StatusPill status={d.status === "cancelled" ? "cancelled" : "issued"} />,
    },
  ];

  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t("documents_title")} description={t("documents_subtitle")} />

      {/* The kind filter is a segmented control rather than a row of radios:
          four values that are always all offered, one of which is always on,
          and the tab list takes the arrow keys in the reading direction on
          its own. */}
      <Tabs
        value={filter}
        onValueChange={(value) => {
          if (!isFilter(value)) return;
          setFilter(value);
          // The open document may not be in the narrowed list any more, and a
          // detail panel showing a row the list no longer has is a screen
          // disagreeing with itself.
          setOpenId(null);
        }}
      >
        <TabsList aria-label={t("documents_filter")}>
          {FILTERS.map((value) => (
            <TabsTrigger key={value} value={value}>
              {t(FILTER_KEY[value])}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>

      {documents.isPending ? <Skeleton className="h-40 w-full" /> : null}
      {documents.isError ? <Refusal error={documents.error} /> : null}

      {documents.isSuccess ? (
        <DataTable
          columns={columns}
          rows={documents.data}
          rowKey={(d) => d.id}
          caption={t("documents_list")}
          empty={
            <EmptyState
              icon={FileText}
              title={t("documents_empty")}
              description={t("documents_empty_hint")}
              data-testid="documents-empty"
            />
          }
        />
      ) : null}

      {openId === null ? null : <DocumentDetail id={openId} onClose={() => setOpenId(null)} />}
    </section>
  );
}

export const Route = createFileRoute("/documents")({ component: DocumentsScreen });
