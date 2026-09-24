// What the order being typed comes to, shown before it is saved (T46): Samir
// typed 100 cafés for 10 and nothing on the form said the order had just
// become ten times bigger. The figure is `purchaseOrderTotal` from the shared
// money code; this file only reads the draft into the shape it takes, and no
// amount is added up here or in the JSX.
//
// It is the total before the landed cost is spread, so the saved order can
// read a few centimes under it (features.md §1); the hint says so, and the
// order's own page carries the server's figure.

import { MoneyError, parseQtyToMilli, purchaseOrderTotal } from "@dzpos/shared";
import type { PurchaseDraftLine } from "@dzpos/shared";

import { Money } from "@/components/Money";
import { useTranslation } from "@/i18n";

/** One row of the line editor, as the form holds it. */
export interface DraftLineAmounts {
  readonly qty: string;
  readonly unitCost: number | null;
}

/**
 * The draft as the shared function takes it. A row still being typed (no
 * cost yet, or a quantity that is not a number yet) counts for nothing rather
 * than stopping the total: the figure follows the typing.
 */
export function draftLines(lines: readonly DraftLineAmounts[]): PurchaseDraftLine[] {
  const read: PurchaseDraftLine[] = [];
  for (const line of lines) {
    const qtyMilli = parseQtyToMilli(line.qty);
    if (line.unitCost === null || qtyMilli === null) continue;
    read.push({ unitCost: line.unitCost, qtyMilli });
  }
  return read;
}

/** The total, or `null` when the shared code refuses the figures. */
export function runningTotal(
  lines: readonly DraftLineAmounts[],
  transport: number | null,
  extra: number | null,
): number | null {
  try {
    return purchaseOrderTotal(draftLines(lines), transport ?? 0, extra ?? 0);
  } catch (error) {
    if (error instanceof MoneyError) return null;
    throw error;
  }
}

export function RunningTotal({
  lines,
  transport,
  extra,
}: {
  lines: readonly DraftLineAmounts[];
  transport: number | null;
  extra: number | null;
}) {
  const { t } = useTranslation();
  const total = runningTotal(lines, transport, extra);
  return (
    <div className="flex flex-col gap-1" data-testid="purchase-running-total">
      <div className="flex items-center justify-between gap-4">
        <span className="text-md font-semibold">{t("purchases_total")}</span>
        {total === null ? (
          <span role="alert" className="text-sm text-fg-danger">
            {t("purchases_running_total_unreadable")}
          </span>
        ) : (
          <Money centimes={total} className="text-xl font-semibold" />
        )}
      </div>
      <p className="text-sm text-muted-foreground">{t("purchases_running_total_hint")}</p>
    </div>
  );
}
