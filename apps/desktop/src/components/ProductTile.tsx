// The tile the till's grid is made of, and the one the products screen shows
// in the kit. A name, a price, and one chip saying what the stock is.
//
// It is a `Button` rather than a card with a click handler: the whole tile is
// the target a cashier hits, so it has to be focusable, reachable by the
// keyboard and disable-able as one thing. `h-auto` and `whitespace-normal`
// undo the two things the button variant assumes about its content, because a
// product name is two lines on a narrow grid and the tile grows for it.
//
// The chip is three states and not a number with a colour on it. A quantity
// alone ("3") answers nothing on a counter: what the cashier needs to know
// before touching the tile is whether the shelf has it, and the shelf has
// three answers. So a product at or under its own threshold wears the kit's
// low pill, a product at zero wears the refusal, and everything else shows
// the quantity plainly.
//
// The threshold is the product's own (`low_stock_at_milli`), never a number
// this file picks: a shop that reorders flour by the sack and sugar by the
// kilo has two different ideas of "running out", and both are already stored.
// A threshold of zero means the shop set none, and then only an empty shelf
// says anything.

import { formatQty } from "@dzpos/shared";

import { Money } from "@/components/Money";
import { StatusPill } from "@/components/StatusPill";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useTranslation } from "@/i18n";
import { cn } from "@/lib/utils";

/** What the shelf says about one product. */
export type StockState = "out" | "low" | "ok";

/**
 * Milli-units in, one word out. Exported because the till sorts and filters
 * on the same three answers and must not recompute them differently.
 *
 * Zero and below is "out" whatever the threshold says, including the negative
 * quantity a recount can leave behind. A threshold of zero is "no threshold
 * set", not "warn me at nothing", which is why it is tested before the
 * comparison rather than folded into it.
 */
export function stockState(qtyMilli: number, lowStockAtMilli: number): StockState {
  if (qtyMilli <= 0) return "out";
  if (lowStockAtMilli > 0 && qtyMilli <= lowStockAtMilli) return "low";
  return "ok";
}

export function ProductTile({
  name,
  priceCentimes,
  qtyMilli,
  lowStockAtMilli,
  onSelect,
  disabled = false,
  className,
  "data-testid": testId,
}: {
  name: string;
  /** The selling price, in centimes. */
  priceCentimes: number;
  qtyMilli: number;
  /** The product's own low-stock threshold; zero means the shop set none. */
  lowStockAtMilli: number;
  onSelect?: () => void;
  /**
   * Refused rather than empty. Whether an empty shelf refuses a sale is the
   * till's rule and not the tile's, so the caller says so.
   */
  disabled?: boolean;
  className?: string;
  "data-testid"?: string;
}) {
  const { t } = useTranslation();
  const state = stockState(qtyMilli, lowStockAtMilli);

  return (
    <Button
      type="button"
      variant="outline"
      data-testid={testId}
      data-stock={state}
      disabled={disabled}
      onClick={onSelect}
      className={cn(
        "h-auto min-h-26 w-full flex-col items-stretch justify-between gap-2 rounded-lg p-3 text-start whitespace-normal",
        className,
      )}
    >
      <span className="line-clamp-2 text-sm leading-tight font-medium text-foreground">{name}</span>
      <span className="flex items-center justify-between gap-2">
        <Money centimes={priceCentimes} className="text-md" data-testid="tile-price" />
        {state === "low" ? (
          <StatusPill status="low" data-testid="tile-stock" />
        ) : state === "out" ? (
          <Badge variant="destructive" data-testid="tile-stock">
            {t("till_out_of_stock")}
          </Badge>
        ) : (
          <Badge variant="secondary" data-testid="tile-stock" className="font-numeric tabular-nums">
            <span dir="ltr">{formatQty(qtyMilli)}</span>
          </Badge>
        )}
      </span>
    </Button>
  );
}
