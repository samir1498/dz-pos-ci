// The cash position of a period, as the expenses screen and the dashboard
// both show it. It lived in `routes/expenses.tsx` until the dashboard needed
// the same block; a second copy there would be the one place two screens
// could answer one question differently, which is the whole thing this panel
// exists to prevent.
//
// Every figure is the server's. The panel adds nothing up and subtracts
// nothing: `cash_in.total`, `cash_out.total` and the net between them are
// three integers the core derived from the ledgers (features.md §1, "no
// column stores a total"), and the panel puts them where a shopkeeper can
// read them.
//
// `cash_out.refunds_centimes` is money handed back over the counter on a
// reversal, on the day the notes changed hands. It was a hard-coded zero
// until ruling 5 of the 2026-09-20 loop; it is now the sum of the
// `cash_refunds` rows, and it is the figure that explains a drawer lighter
// than the day's takings.

import type { CashPositionDto } from "@dzpos/shared";

import { Money } from "@/components/Money";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { useTranslation } from "@/i18n";
import { cn } from "@/lib/utils";

/**
 * The cash position of one period. Every figure is the server's: the panel
 * shows what the core summed and adds nothing up, so the box and the
 * dashboard can never be two answers to one question.
 */
export function CashPanel({ position }: { position: CashPositionDto }) {
  const { t } = useTranslation();
  return (
    <Card data-testid="cash-position" className="h-full">
      <CardHeader>
        <CardTitle>{t("cash_title")}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1">
            <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
              {t("cash_in")}
            </h3>
            <Figure label={t("cash_sales")} centimes={position.cash_in.sales_centimes} />
            {/* Inside the line above, not beside it: the drawer took the stamp
                with the rest, and this says how much of what it took is tax
                the shop is holding for the state. */}
            <Figure
              label={t("cash_stamp")}
              centimes={position.cash_in.stamp_centimes}
              testId="cash-in-stamp"
            />
            <Figure
              label={t("cash_customer_payments")}
              centimes={position.cash_in.customer_payments_centimes}
            />
            <Figure
              label={t("cash_total")}
              centimes={position.cash_in.total_centimes}
              testId="cash-in-total"
              strong
            />
          </div>
          <div className="flex flex-col gap-1">
            <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
              {t("cash_out")}
            </h3>
            <Figure
              label={t("cash_supplier_payments")}
              centimes={position.cash_out.supplier_payments_centimes}
            />
            <Figure
              label={t("cash_expenses")}
              centimes={position.cash_out.expenses_centimes}
              testId="cash-out-expenses"
            />
            <Figure
              label={t("cash_refunds")}
              centimes={position.cash_out.refunds_centimes}
              hint={t("cash_refunds_hint")}
              testId="cash-out-refunds"
            />
            <Figure
              label={t("cash_total")}
              centimes={position.cash_out.total_centimes}
              testId="cash-out-total"
              strong
            />
          </div>
        </div>
        <Separator />
        <Figure
          label={t("cash_net")}
          centimes={position.cash_centimes}
          testId="cash-net"
          strong
        />
        <Figure
          label={t("cash_card_in")}
          centimes={position.card_in.total_centimes}
          testId="card-in-total"
        />
      </CardContent>
    </Card>
  );
}

/** One labelled amount. The figure is `Money`, which carries the figure face
 *  and the `dir="ltr"` an amount needs on the Arabic screen too. */
function Figure({
  label,
  centimes,
  testId,
  hint,
  strong = false,
}: {
  label: string;
  centimes: number;
  testId?: string;
  hint?: string;
  strong?: boolean;
}) {
  return (
    <p className={cn("flex items-baseline justify-between gap-4 text-sm", strong && "font-semibold")}>
      <span className={strong ? "text-foreground" : "text-muted-foreground"} title={hint}>
        {label}
      </span>
      <Money centimes={centimes} data-testid={testId} />
    </p>
  );
}
