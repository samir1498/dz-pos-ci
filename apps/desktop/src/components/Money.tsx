// An amount, everywhere one is shown. Three things it does that a bare
// `{formatCentimes(n)}` does not:
//
// - The figure font, which is JetBrains Mono through `--font-numeric`, with
//   tabular figures on. A column of totals then lines up on the digit rather
//   than on the glyph width, which is the whole reason the design carries a
//   second face at all.
// - `dir="ltr"`. An amount reads left to right with Western digits on an
//   Arabic screen too, the same decision the fiscal identifiers and the
//   barcodes already take; without it the minus sign of a negative balance
//   swaps to the far end of the number.
// - The centimes never leave the integer. It takes centimes and hands them to
//   the shared formatter; no arithmetic happens here and no float is made.

import { formatCentimes } from "@dzpos/shared";

import { cn } from "@/lib/utils";

export function Money({
  centimes,
  className,
  "data-testid": testId,
}: {
  centimes: number;
  className?: string;
  "data-testid"?: string;
}) {
  return (
    <span dir="ltr" data-testid={testId} className={cn("font-numeric tabular-nums", className)}>
      {formatCentimes(centimes)}
    </span>
  );
}
