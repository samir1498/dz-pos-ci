// The five states a row can be in, drawn the same way on every screen.
//
// A pill is a colour and a word, never a colour alone: the shop prints in
// black and white, the ink theme is not the paper theme, and a red dot means
// nothing to someone who cannot tell it from the green one. So the word is
// the pill and the colour is the emphasis.
//
// The tones point at the soft surfaces and the text roles, which every theme
// redefines, so a pill on the ink theme is the ink theme's version of the
// same idea and no code here learns which theme is on.
//
// Five and no more. "issued" and "cancelled" are what a document is,
// "paid" and "open" are what a balance is, "low" is what a stock level is;
// a sixth state means a screen has an idea the kit has not been told about,
// and it is added here rather than drawn on the screen.

import { cva, type VariantProps } from "class-variance-authority";

import { useTranslation, type Key } from "@/i18n";
import { cn } from "@/lib/utils";

const pill = cva(
  "inline-flex w-fit shrink-0 items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium whitespace-nowrap",
  {
    variants: {
      status: {
        issued: "bg-info-soft text-info",
        cancelled: "bg-danger-soft text-fg-danger",
        paid: "bg-primary-soft text-fg-success",
        open: "bg-muted text-muted-foreground",
        low: "bg-warn-soft text-warn",
      },
    },
    defaultVariants: { status: "open" },
  },
);

export type Status = NonNullable<VariantProps<typeof pill>["status"]>;

/**
 * Spelled out rather than built from the status name, so a key the three
 * dictionaries do not carry fails the parity test instead of rendering the
 * key itself on a screen.
 */
const LABEL: Readonly<Record<Status, Key>> = {
  issued: "pill_issued",
  cancelled: "pill_cancelled",
  paid: "pill_paid",
  open: "pill_open",
  low: "pill_low",
};

export function StatusPill({
  status,
  className,
  "data-testid": testId,
}: {
  status: Status;
  className?: string;
  "data-testid"?: string;
}) {
  const { t } = useTranslation();
  return (
    <span data-testid={testId} data-status={status} className={cn(pill({ status }), className)}>
      {t(LABEL[status])}
    </span>
  );
}
