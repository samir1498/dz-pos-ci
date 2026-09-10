// The one brass button in the app.
//
// Brass is the money colour (`--color-money`, the same role the amounts wear)
// and it is spent on exactly one control: the button that takes the payment.
// A second brass button anywhere else is what makes the first one ordinary,
// so this is a component rather than a `variant="money"` on `Button` that
// any screen could reach for.
//
// It is the tall control (`--control-h-lg`, 52 px) because a cashier hits it
// with a thumb on a counter, and it is `type="button"` by default: the till's
// payment panel is not a form and a stray submit there would reload the till.

import type { ComponentProps } from "react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export function PayButton({
  className,
  type = "button",
  "data-testid": testId = "pay-button",
  ...props
}: ComponentProps<typeof Button> & { "data-testid"?: string }) {
  return (
    <Button
      data-testid={testId}
      type={type}
      size="lg"
      className={cn(
        "h-(--control-h-lg) bg-money px-6 text-md font-semibold text-money-foreground shadow-sm hover:bg-money-hover focus-visible:ring-money/40",
        className,
      )}
      {...props}
    />
  );
}
