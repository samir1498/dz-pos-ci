// The catalogue itself, as `DataTable` and therefore the same table as every
// other list in the app. Reading and picking rows only: the fiche that adds
// or edits one lives in `-products/fiche.tsx`.

import { formatQty } from "@dzpos/shared";
import type { ProductDto } from "@dzpos/shared";
import type { ReactNode } from "react";

import { DataTable, type Column } from "@/components/DataTable";
import { Money } from "@/components/Money";
import { stockState } from "@/components/ProductTile";
import { StatusPill } from "@/components/StatusPill";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { useTranslation } from "@/i18n";
import { rateCellLabel } from "@/lib/rate";

import { UNIT_KEY } from "./parts";

export function ProductTable({
  rows,
  empty,
  onEdit,
  picked,
  onPick,
}: {
  rows: readonly ProductDto[];
  empty: ReactNode;
  onEdit: (row: ProductDto) => void;
  picked: readonly number[];
  onPick: (id: number, on: boolean) => void;
}) {
  const { t } = useTranslation();

  // `dir="ltr"` on the four cells below: a barcode, a price, a rate and a
  // quantity are read left to right with Western digits regardless of the
  // screen's language (a decision, features.md names no rule for it).
  // Without it the Unicode bidi algorithm is free to reorder the space and
  // the sign around the digits inside an RTL row. `Money` carries its own.
  const columns: readonly Column<ProductDto>[] = [
    {
      id: "pick",
      header: t("col_pick"),
      cell: (row) => (
        <Checkbox
          aria-label={`${t("col_pick")} ${row.name}`}
          checked={picked.includes(row.id)}
          onCheckedChange={(next) => onPick(row.id, next === true)}
        />
      ),
    },
    {
      id: "name",
      header: t("col_name"),
      cell: (row) => (
        <span className="flex flex-wrap items-center gap-2">
          <span className={row.active ? "" : "text-muted-foreground"}>{row.display_name}</span>
          {row.active ? null : <Badge variant="outline">{t("products_inactive")}</Badge>}
        </span>
      ),
    },
    {
      id: "barcode",
      header: t("col_barcode"),
      cell: (row) => (
        <span dir="ltr" data-testid="cell-barcode" className="font-numeric tabular-nums">
          {row.barcode ?? ""}
        </span>
      ),
    },
    { id: "unit", header: t("col_unit"), cell: (row) => t(UNIT_KEY[row.unit]) },
    {
      id: "price",
      header: t("col_price"),
      money: true,
      cell: (row) => <Money centimes={row.selling_centimes} data-testid="cell-price" />,
    },
    {
      id: "rate",
      header: t("col_rate"),
      numeric: true,
      cell: (row) => (
        <span dir="ltr" data-testid="cell-rate">
          {rateCellLabel(row.rate_bps, t)}
        </span>
      ),
    },
    {
      id: "stock",
      header: t("col_stock"),
      numeric: true,
      cell: (row) => (
        <span className="flex items-center justify-end gap-2">
          <span dir="ltr" data-testid="cell-stock">
            {formatQty(row.qty_on_hand_milli)}
          </span>
          {stockState(row.qty_on_hand_milli, row.low_stock_at_milli) === "ok" ? null : (
            <StatusPill status="low" data-testid="cell-low" />
          )}
        </span>
      ),
    },
  ];

  return (
    <DataTable
      columns={columns}
      rows={rows}
      rowKey={(row) => row.id}
      caption={t("products_title")}
      empty={empty}
      data-testid="products-table"
      actions={(row) => (
        <Button
          variant="ghost"
          size="sm"
          aria-label={`${t("products_edit")} ${row.name}`}
          onClick={() => onEdit(row)}
        >
          {t("products_edit")}
        </Button>
      )}
    />
  );
}
