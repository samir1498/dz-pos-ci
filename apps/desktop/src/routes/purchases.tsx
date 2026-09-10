// The purchases screen: what the shop has ordered from its suppliers and
// where each order stands. Everything it shows comes from the API over HTTP.
//
// Nothing here deletes an order. An order nothing arrived against is
// cancelled and one the rest of which will never come is closed short, and
// both are on the order's own page because both ask for a reason.
//
// Stock and what the shop owes move when goods arrive, never when the paper
// is written (features.md §1): the list says which orders are still waiting.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { formatCentimes } from "@dzpos/shared";
import type { PurchaseDto, PurchaseStatusDto, SupplierDto } from "@dzpos/shared";

import { api, purchasesQueryKey, suppliersQueryKey } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

export const Route = createFileRoute("/purchases")({ component: PurchasesScreen });

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

/** Blank is every state; the select sends the empty string for it. */
function chosenState(value: string): PurchaseStatusDto | undefined {
  return PURCHASE_STATES.find((state) => state === value);
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
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("purchases_title")}</h1>
        <Link to="/purchases/new" className="rounded border px-3 py-1.5">
          {t("purchases_add")}
        </Link>
      </header>

      <div className="flex flex-wrap gap-4">
        {/* The label points at the select by id rather than wrapping it: a
            wrapping label's text is its whole content, options included. */}
        <label className="flex flex-col gap-1" htmlFor="purchases-filter-status">
          <span>{t("purchases_filter_status")}</span>
          <select
            id="purchases-filter-status"
            className="rounded border px-2 py-1"
            value={status}
            onChange={(e) => setStatus(e.target.value)}
          >
            <option value="">{t("purchases_filter_any")}</option>
            {PURCHASE_STATES.map((state) => (
              <option key={state} value={state}>
                {t(PURCHASE_STATUS_KEY[state])}
              </option>
            ))}
          </select>
        </label>
        <label className="flex flex-col gap-1" htmlFor="purchases-filter-supplier">
          <span>{t("purchases_filter_supplier")}</span>
          <select
            id="purchases-filter-supplier"
            className="rounded border px-2 py-1"
            value={supplier}
            onChange={(e) => setSupplier(e.target.value)}
          >
            <option value="">{t("purchases_filter_any")}</option>
            {(suppliers.data ?? []).map((s) => (
              <option key={s.id} value={String(s.id)}>
                {s.name}
              </option>
            ))}
          </select>
        </label>
      </div>

      {purchases.isPending ? <p>{t("purchases_loading")}</p> : null}
      {purchases.isError ? (
        <p role="alert" className="text-red-700">
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
  if (rows.length === 0) return <p>{t("purchases_empty")}</p>;
  return (
    <table className="w-full text-start">
      <caption className="sr-only">{t("purchases_title")}</caption>
      <thead>
        <tr>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_date")}</th>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_supplier")}</th>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_supplier_document")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_extra_costs")}</th>
          <th scope="col" className="text-start pb-2 ps-3">{t("col_status")}</th>
          <th scope="col" className="pb-2">
            <span className="sr-only">{t("purchases_open")}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {rows.map((p) => (
          <tr key={p.id} className="border-t">
            {/* dir="ltr" on the value itself: a day and an amount are read
                left to right with Western digits whatever the screen's
                language. */}
            <td className="py-1.5 pe-3 font-mono">
              <span dir="ltr">{p.purchase_date}</span>
            </td>
            <td className="py-1.5 pe-3">
              {suppliers.find((s) => s.id === p.supplier_id)?.name ?? ""}
            </td>
            <td className="py-1.5 pe-3 font-mono">
              <span dir="ltr">{p.supplier_document_number ?? ""}</span>
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">
                {formatCentimes(p.transport_centimes + p.extra_costs_centimes)}
              </span>
            </td>
            <td className="py-1.5 ps-3">{t(PURCHASE_STATUS_KEY[p.status])}</td>
            <td className="py-1.5 ps-3 text-end">
              <Link
                to="/purchases/$id"
                params={{ id: String(p.id) }}
                className="rounded border px-2 py-0.5 text-sm"
                aria-label={`${t("purchases_open")} ${p.purchase_date}`}
              >
                {t("purchases_open")}
              </Link>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
