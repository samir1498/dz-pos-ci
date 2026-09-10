// The basket half of the till: the lines a cashier is editing, the discount
// on the whole of it, and the preview of what it comes to.
//
// The totals shown here are a preview, computed by `computeTotals` in
// @dzpos/shared, the same module the same fixtures pin the Rust core with
// (architecture.md rule 2). The route computes it; this file only lays it
// out, so no amount is worked out twice.
//
// The folder is `-till` and not `till`: TanStack Router turns every file under
// `routes/` into a route, and the `-` prefix in tsr.config.json is what says
// these three are parts of a screen rather than screens of their own.

import { formatCentimes, formatQty, lineTotal, parseAmountToCentimes, parseQtyToMilli } from "@dzpos/shared";
import type { ProductDto, Totals, UnitDto } from "@dzpos/shared";

import { useTranslation, type Key } from "@/i18n";
import { rateCellLabel } from "@/lib/rate";

/** One unit, in thousandths. What a tile adds and what the + button adds. */
export const ONE_UNIT_MILLI = 1_000;

/** Units a shop cannot sell a fraction of. A half box is not a thing a
 * receipt can say, and the core would take the 500 without a word. */
const WHOLE_UNITS: readonly UnitDto[] = ["piece", "box"];

/** A cart line as the cashier is editing it: the quantity and the discount
 * stay the text that was typed, so "1," on the way to "1,5" is not thrown
 * away by a parse and written back as "1". */
export interface CartLine {
  readonly product: ProductDto;
  readonly qtyText: string;
  readonly discountText: string;
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
  const discount =
    line.discountText.trim() === "" ? 0 : (parseAmountToCentimes(line.discountText) ?? -1);
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
      <button
        type="button"
        className="rounded border px-2 py-1 text-sm"
        disabled={count === 0}
        onClick={onClear}
      >
        {t("till_clear")}
      </button>
    </header>
  );
}

/** The lines, the discount on the whole basket, and the preview. `totals` is
 * null while any of the three problems stands, and the table is left out
 * rather than shown stale. */
export function Cart({
  lines,
  read,
  discountText,
  onDiscountText,
  discountProblem,
  totalsProblem,
  totals,
  onQty,
  onDiscount,
  onStep,
  onRemove,
}: {
  lines: readonly CartLine[];
  read: readonly ReadLine[];
  discountText: string;
  onDiscountText: (value: string) => void;
  discountProblem: Key | null;
  totalsProblem: Key | null;
  totals: Totals | null;
  onQty: (id: number, value: string) => void;
  onDiscount: (id: number, value: string) => void;
  onStep: (id: number, by: number) => void;
  onRemove: (id: number) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <div data-testid="cart" className="flex flex-col gap-3">
        {lines.length === 0 ? <p>{t("till_cart_empty")}</p> : null}
        {lines.map((line, i) => (
          <CartRow
            key={line.product.id}
            line={line}
            read={read[i]}
            onQty={(value) => onQty(line.product.id, value)}
            onDiscount={(value) => onDiscount(line.product.id, value)}
            onStep={(by) => onStep(line.product.id, by)}
            onRemove={() => onRemove(line.product.id)}
          />
        ))}
      </div>

      <label className="flex flex-col gap-1">
        <span>{t("field_global_discount")}</span>
        <input
          dir="ltr"
          inputMode="decimal"
          className="rounded border px-2 py-1 text-end font-mono"
          value={discountText}
          onChange={(e) => onDiscountText(e.target.value)}
        />
      </label>
      {discountProblem !== null || totalsProblem !== null ? (
        <p role="alert" className="text-sm text-red-700">
          {t(discountProblem ?? totalsProblem ?? "error_unknown")}
        </p>
      ) : null}

      {totals !== null ? <TotalsTable totals={totals} /> : null}
    </>
  );
}

function CartRow({
  line,
  read,
  onQty,
  onDiscount,
  onStep,
  onRemove,
}: {
  line: CartLine;
  read: ReadLine;
  onQty: (value: string) => void;
  onDiscount: (value: string) => void;
  onStep: (by: number) => void;
  onRemove: () => void;
}) {
  const { t } = useTranslation();
  const net = read.problem === null ? read.gross - read.discount : 0;
  return (
    <div className="flex flex-col gap-1 border-t pt-2">
      <div className="flex items-start justify-between gap-2">
        <span className="font-medium">{line.product.name}</span>
        <span className="font-mono" dir="ltr">
          {read.problem === null ? formatCentimes(net) : ""}
        </span>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          className="rounded border px-2"
          aria-label={`${t("till_qty_decrease")} ${line.product.name}`}
          onClick={() => onStep(-ONE_UNIT_MILLI)}
        >
          −
        </button>
        <input
          dir="ltr"
          inputMode="decimal"
          size={5}
          className="w-16 rounded border px-2 py-1 text-end font-mono"
          aria-label={`${t("field_qty")} ${line.product.name}`}
          value={line.qtyText}
          onChange={(e) => onQty(e.target.value)}
        />
        <button
          type="button"
          className="rounded border px-2"
          aria-label={`${t("till_qty_increase")} ${line.product.name}`}
          onClick={() => onStep(ONE_UNIT_MILLI)}
        >
          +
        </button>
        <span className="font-mono text-sm" dir="ltr">
          {`× ${formatCentimes(line.product.selling_centimes)}`}
        </span>
        <input
          dir="ltr"
          inputMode="decimal"
          size={6}
          className="w-20 rounded border px-2 py-1 text-end font-mono"
          aria-label={`${t("field_line_discount")} ${line.product.name}`}
          value={line.discountText}
          onChange={(e) => onDiscount(e.target.value)}
        />
        <button
          type="button"
          className="ms-auto rounded border px-2"
          aria-label={`${t("till_line_remove")} ${line.product.name}`}
          onClick={onRemove}
        >
          ✕
        </button>
      </div>
      {read.problem !== null ? (
        <span role="alert" className="text-sm text-red-700">
          {t(read.problem)}
        </span>
      ) : null}
    </div>
  );
}

/** The preview, column for column with the totals table of features.md §3.
 * A row worth nothing is not printed, the way the ticket does not print it. */
function TotalsTable({ totals }: { totals: Totals }) {
  const { t } = useTranslation();
  return (
    <table className="w-full" aria-label={t("total_net_to_pay")}>
      <tbody>
        <TotalsRow label={t("total_ht")} centimes={totals.totalHt} />
        {totals.discount > 0 ? (
          <TotalsRow label={t("total_discount")} centimes={totals.discount} />
        ) : null}
        {totals.tvaByRate.map((g) => (
          <TotalsRow
            key={g.rateBps}
            label={`${t("total_tva")} ${rateCellLabel(g.rateBps, t)}`}
            centimes={g.amount}
          />
        ))}
        {totals.stamp > 0 ? <TotalsRow label={t("total_stamp")} centimes={totals.stamp} /> : null}
        <TotalsRow
          label={t("total_net_to_pay")}
          centimes={totals.netToPay}
          testId="total-net-to-pay"
          strong
        />
      </tbody>
    </table>
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
    <tr className={strong ? "font-semibold" : undefined}>
      <th scope="row" className="py-0.5 text-start font-normal">
        {label}
      </th>
      <td data-testid={testId} className="py-0.5 text-end font-mono" dir="ltr">
        {formatCentimes(centimes)}
      </td>
    </tr>
  );
}
