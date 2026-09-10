// The basket half of the till: the lines a cashier is editing, the discount
// on the whole of it, and the preview of what it comes to.
//
// The totals shown here are a preview, computed by `computeTotals` in
// @dzpos/shared, the same module the same fixtures pin the Rust core with
// (architecture.md rule 2). The route computes it; this file only lays it
// out, so no amount is worked out twice.
//
// The lines are a `DataTable` like every other list in the app, and the two
// amounts on a line are `MoneyInput` and `Money`, so the discount a cashier
// types is integer centimes from the keystroke on and the line total lines up
// on the digit with the totals under it.
//
// The totals block is not a list and is not one: it is a few named amounts,
// so it is laid out as rows of a label and a `Money` rather than forced into a
// table whose header row would say nothing. It keeps the accessible name the
// tests ask it for.
//
// The folder is `-till` and not `till`: TanStack Router turns every file under
// `routes/` into a route, and the `-` prefix in tsr.config.json is what says
// these four are parts of a screen rather than screens of their own.

import { lineTotal, formatQty, parseQtyToMilli } from "@dzpos/shared";
import type { ProductDto, Totals, UnitDto } from "@dzpos/shared";
import { Minus, Plus, ShoppingCart, X } from "lucide-react";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useTranslation, type Key } from "@/i18n";
import { rateCellLabel } from "@/lib/rate";

/** One unit, in thousandths. What a tile adds and what the + button adds. */
export const ONE_UNIT_MILLI = 1_000;

/** Units a shop cannot sell a fraction of. A half box is not a thing a
 * receipt can say, and the core would take the 500 without a word. */
const WHOLE_UNITS: readonly UnitDto[] = ["piece", "box"];

/** A cart line as the cashier is editing it. The quantity stays the text that
 * was typed, so "1," on the way to "1,5" is not thrown away by a parse and
 * written back as "1"; the discount is integer centimes, because `MoneyInput`
 * owns that halfway state itself and hands back the integer or nothing. */
export interface CartLine {
  readonly product: ProductDto;
  readonly qtyText: string;
  readonly discount: number | null;
}

/** A line read: its amounts if they are readable, the field message if not. */
export interface ReadLine {
  readonly qtyMilli: number;
  readonly discount: number;
  readonly gross: number;
  readonly problem: Key | null;
}

export function readLine(line: CartLine): ReadLine {
  const qtyMilli = parseQtyToMilli(line.qtyText);
  if (qtyMilli === null || qtyMilli <= 0) {
    return { qtyMilli: 0, discount: 0, gross: 0, problem: "error_qty_invalid" };
  }
  if (WHOLE_UNITS.includes(line.product.unit) && qtyMilli % ONE_UNIT_MILLI !== 0) {
    return { qtyMilli, discount: 0, gross: 0, problem: "error_qty_whole" };
  }
  const gross = lineTotal(line.product.selling_centimes, qtyMilli);
  const discount = line.discount ?? 0;
  if (discount < 0) {
    return { qtyMilli, discount: 0, gross, problem: "error_discount_invalid" };
  }
  if (discount > gross) {
    return { qtyMilli, discount, gross, problem: "error_discount_above_line" };
  }
  return { qtyMilli, discount, gross, problem: null };
}

/** How many lines the basket holds, and the way to empty it. */
export function CartHeader({ count, onClear }: { count: number; onClear: () => void }) {
  const { t } = useTranslation();
  return (
    <header className="flex items-center justify-between gap-2">
      <strong>{`${t("till_cart")} · ${count}`}</strong>
      <Button type="button" variant="ghost" size="sm" disabled={count === 0} onClick={onClear}>
        {t("till_clear")}
      </Button>
    </header>
  );
}

/** What one row of the table is made of, so the columns below read as a list
 * of cells rather than as a second copy of the props. */
interface Row {
  readonly line: CartLine;
  readonly read: ReadLine;
}

/** The lines, the discount on the whole basket, and the preview. `totals` is
 * null while any of the three problems stands, and the block is left out
 * rather than shown stale. */
export function Cart({
  lines,
  read,
  discount,
  onDiscount,
  discountProblem,
  totalsProblem,
  totals,
  onQty,
  onLineDiscount,
  onStep,
  onRemove,
}: {
  lines: readonly CartLine[];
  read: readonly ReadLine[];
  discount: number | null;
  onDiscount: (centimes: number | null) => void;
  discountProblem: Key | null;
  totalsProblem: Key | null;
  totals: Totals | null;
  onQty: (id: number, value: string) => void;
  onLineDiscount: (id: number, centimes: number | null) => void;
  onStep: (id: number, by: number) => void;
  onRemove: (id: number) => void;
}) {
  const { t } = useTranslation();
  const rows: Row[] = lines.map((line, i) => ({ line, read: read[i] }));

  const columns: readonly Column<Row>[] = [
    {
      id: "product",
      header: t("col_product"),
      cell: (row) => (
        <div className="flex flex-col gap-0.5">
          <span className="font-medium">{row.line.product.name}</span>
          <span className="text-xs text-muted-foreground">
            <Money centimes={row.line.product.selling_centimes} className="font-normal" />
            {` · ${t("total_tva")} ${rateCellLabel(row.line.product.rate_bps, t)}`}
          </span>
          {row.read.problem !== null ? (
            <span role="alert" className="text-sm text-fg-danger">
              {t(row.read.problem)}
            </span>
          ) : null}
        </div>
      ),
    },
    {
      id: "qty",
      header: t("field_qty"),
      numeric: true,
      cell: (row) => (
        <div className="flex items-center justify-end gap-1">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={`${t("till_qty_decrease")} ${row.line.product.name}`}
            onClick={() => onStep(row.line.product.id, -ONE_UNIT_MILLI)}
          >
            <Icon as={Minus} size={18} />
          </Button>
          <Input
            dir="ltr"
            inputMode="decimal"
            className="w-16 text-end font-numeric tabular-nums"
            aria-label={`${t("field_qty")} ${row.line.product.name}`}
            value={row.line.qtyText}
            onChange={(event) => onQty(row.line.product.id, event.target.value)}
          />
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={`${t("till_qty_increase")} ${row.line.product.name}`}
            onClick={() => onStep(row.line.product.id, ONE_UNIT_MILLI)}
          >
            <Icon as={Plus} size={18} />
          </Button>
        </div>
      ),
    },
    {
      id: "discount",
      header: t("total_discount"),
      money: true,
      cell: (row) => (
        <MoneyInput
          className="w-24"
          aria-label={`${t("field_line_discount")} ${row.line.product.name}`}
          value={row.line.discount}
          onChange={(centimes) => onLineDiscount(row.line.product.id, centimes)}
        />
      ),
    },
    {
      id: "total",
      header: t("col_total"),
      money: true,
      cell: (row) =>
        row.read.problem === null ? <Money centimes={row.read.gross - row.read.discount} /> : null,
    },
  ];

  return (
    <>
      <DataTable
        data-testid="cart"
        columns={columns}
        rows={rows}
        rowKey={(row) => row.line.product.id}
        caption={t("till_cart")}
        empty={<EmptyState icon={ShoppingCart} title={t("till_cart_empty")} />}
        actions={(row) => (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={`${t("till_line_remove")} ${row.line.product.name}`}
            onClick={() => onRemove(row.line.product.id)}
          >
            <Icon as={X} size={18} />
          </Button>
        )}
      />

      <FormField
        label={t("field_global_discount")}
        error={
          discountProblem !== null || totalsProblem !== null
            ? t(discountProblem ?? totalsProblem ?? "error_unknown")
            : undefined
        }
      >
        {(parts) => <MoneyInput {...parts} value={discount} onChange={onDiscount} />}
      </FormField>

      {totals !== null ? <TotalsBlock totals={totals} /> : null}
    </>
  );
}

/** The preview, row for row with the totals table of features.md §3. A row
 * worth nothing is not printed, the way the ticket does not print it. */
function TotalsBlock({ totals }: { totals: Totals }) {
  const { t } = useTranslation();
  return (
    <dl
      aria-label={t("till_totals")}
      data-testid="till-totals"
      className="flex flex-col gap-1 border-t border-border pt-2"
    >
      <TotalsRow label={t("total_ht")} centimes={totals.totalHt} />
      {totals.discount > 0 ? (
        <TotalsRow label={t("total_discount")} centimes={totals.discount} />
      ) : null}
      {totals.tvaByRate.map((group) => (
        <TotalsRow
          key={group.rateBps}
          label={`${t("total_tva")} ${rateCellLabel(group.rateBps, t)}`}
          centimes={group.amount}
        />
      ))}
      {totals.stamp > 0 ? <TotalsRow label={t("total_stamp")} centimes={totals.stamp} /> : null}
      <TotalsRow
        label={t("total_net_to_pay")}
        centimes={totals.netToPay}
        testId="total-net-to-pay"
        strong
      />
    </dl>
  );
}

function TotalsRow({
  label,
  centimes,
  testId,
  strong = false,
}: {
  label: string;
  centimes: number;
  testId?: string;
  strong?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <dt className={strong ? "font-semibold" : "text-muted-foreground"}>{label}</dt>
      <dd>
        <Money
          centimes={centimes}
          data-testid={testId}
          className={strong ? "text-lg font-semibold" : undefined}
        />
      </dd>
    </div>
  );
}
