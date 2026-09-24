// The head of one order's page: its day, its supplier, its state, and where
// it stands in money (T46). Lifted out of `purchases_.$id.tsx`, which is at
// the length `scripts/file-sizes.json` pins it to.
//
// Every figure is the server's (`PurchaseDetailDto`, computed in
// `services::purchase_account`): the total at the cost the goods landed at,
// what arrived, what was paid and what is still owed. Nothing is added or
// taken away here; a negative "owed" is credit the supplier holds and is
// named as such, the way the supplier's balance card names it.

import { Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import type { PurchaseDetailDto } from "@dzpos/shared";

import { Money } from "@/components/Money";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { api, suppliersQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { positive } from "@/lib/fields";

import { PurchaseStatusBadge } from "../purchases";

/** One fact of the order's head: what it is called and what it says. */
function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd>{children}</dd>
    </div>
  );
}

export function PurchaseHead({
  detail,
  seeCostAndMargin,
}: {
  detail: PurchaseDetailDto;
  seeCostAndMargin: boolean;
}) {
  const { t } = useTranslation();
  const { purchase } = detail;
  const suppliers = useQuery({
    queryKey: [...suppliersQueryKey, ""],
    queryFn: () => api.listSuppliers(),
  });
  // The name, which is what a shop knows its supplier by (T46); the id only
  // until the list has answered.
  const supplierName =
    suppliers.data?.find((s) => s.id === purchase.supplier_id)?.name ??
    String(purchase.supplier_id);
  const creditHeld = detail.owed_centimes < 0;

  return (
    <Card>
      <CardContent className="flex flex-col gap-4">
        <dl className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <Fact label={t("col_date")}>
            <span dir="ltr" className="font-numeric tabular-nums">
              {purchase.purchase_date}
            </span>
          </Fact>
          <Fact label={t("col_supplier")}>
            <Button asChild variant="link" size="sm" className="h-auto p-0">
              <Link to="/suppliers/$id" params={{ id: String(purchase.supplier_id) }}>
                {supplierName}
              </Link>
            </Button>
          </Fact>
          {/* A test id, because the state's own word and a column header of
              the lines table read the same in English ("Received"). */}
          <Fact label={t("col_status")}>
            <PurchaseStatusBadge status={purchase.status} data-testid="purchase-status" />
          </Fact>
          <Fact label={t("col_supplier_document")}>
            <span dir="ltr" className="font-numeric tabular-nums">
              {purchase.supplier_document_number ?? ""}
            </span>
          </Fact>
        </dl>
        {seeCostAndMargin ? (
          <dl className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5" data-testid="purchase-figures">
            <Fact label={t("purchases_total")}>
              <Money centimes={detail.total_centimes} />
            </Fact>
            <Fact label={t("col_extra_costs")}>
              <Money centimes={purchase.extras_centimes} />
            </Fact>
            <Fact label={t("purchases_received_value")}>
              <Money centimes={detail.received_centimes} />
            </Fact>
            <Fact label={t("purchases_paid")}>
              <Money centimes={detail.paid_centimes} />
            </Fact>
            <Fact label={t(creditHeld ? "purchases_credit_held" : "purchases_owed")}>
              <Money centimes={positive(detail.owed_centimes)} className="font-semibold" />
            </Fact>
          </dl>
        ) : null}
        {seeCostAndMargin ? (
          <p className="text-sm text-muted-foreground">{t("purchases_owed_hint")}</p>
        ) : null}
      </CardContent>
    </Card>
  );
}
