// An amount typed in and read out as integer centimes. No float ever exists
// here: the text goes to `parseAmountToCentimes` and the number comes back
// through `formatCentimes`, both from packages/shared, which are the same
// two functions the ticket and the core's fixtures agree with. This file
// computes nothing.
//
// It keeps its own text while the field has focus, because the typed string
// and the stored integer are not the same thing on the way through. "1 0"
// and "1," are both halfway to a number, and reformatting on every keystroke
// would move the caret out from under the shop's fingers. So the text is
// what the person typed until they leave, and on blur it is replaced by the
// canonical spelling of whatever integer was understood. Leaving the field
// with something unreadable in it puts the last understood value back rather
// than silently keeping a number the field no longer shows.
//
// `dir="ltr"` for the same reason `Money` carries it: an amount reads left to
// right with Western digits on the Arabic screen too, and without it a minus
// sign jumps to the far end of the number.
//
// The text itself starts on the reading side, like every other field: a
// money box that alone typed from the far edge read as disorienting rather
// than as a currency convention (T14). A caller inside a table or a totals
// block, where the digits of several rows have to stack in one column, asks
// for `text-end` itself through `className`; this file does not choose that
// for every screen.
//
// Blank is `null`, not zero. "No amount given" and "zero dinars" are
// different answers to "how much was the discount", and a field that turned
// the first into the second would write a zero the shop never typed.

import { formatCentimes, parseAmountToCentimes } from "@dzpos/shared";
import { useEffect, useRef, useState } from "react";

import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

export function MoneyInput({
  value,
  onChange,
  id,
  className,
  disabled,
  "data-testid": testId,
  "aria-invalid": invalid,
  "aria-describedby": describedBy,
  "aria-label": label,
  placeholder,
  onKeyDown,
}: {
  /** Centimes, or `null` for an empty field. */
  value: number | null;
  onChange: (centimes: number | null) => void;
  id?: string;
  className?: string;
  disabled?: boolean;
  "data-testid"?: string;
  "aria-invalid"?: boolean;
  "aria-describedby"?: string;
  "aria-label"?: string;
  placeholder?: string;
  /** For a screen that wants to know about Enter. The box itself does
   * nothing with it: the value is already committed on every keystroke. */
  onKeyDown?: (event: React.KeyboardEvent<HTMLInputElement>) => void;
}) {
  const shown = (centimes: number | null): string =>
    centimes === null ? "" : formatCentimes(centimes);
  const [text, setText] = useState<string>(() => shown(value));
  const editing = useRef<boolean>(false);

  // A value the field did not type (a form reset, a row loaded, a discount
  // recomputed elsewhere) has to reach the box. While the caret is in it,
  // it does not: overwriting what someone is halfway through typing is
  // worse than showing it a moment late, and the blur below reconciles.
  useEffect(() => {
    if (!editing.current) setText(shown(value));
    // The value is the only thing that moves: `shown` closes over nothing
    // but `formatCentimes`, which is a module-level pure function.
  }, [value]);

  return (
    <Input
      id={id}
      dir="ltr"
      type="text"
      inputMode="decimal"
      autoComplete="off"
      disabled={disabled}
      placeholder={placeholder}
      aria-label={label}
      aria-invalid={invalid}
      aria-describedby={describedBy}
      data-testid={testId}
      className={cn("font-numeric tabular-nums", className)}
      value={text}
      onKeyDown={onKeyDown}
      onFocus={() => {
        editing.current = true;
      }}
      onChange={(event) => {
        const next = event.target.value;
        setText(next);
        if (next.trim() === "") {
          onChange(null);
          return;
        }
        const centimes = parseAmountToCentimes(next);
        // Unreadable so far ("1 " on the way to "1 000"): the field keeps
        // the text and the last understood value stands. The blur decides.
        if (centimes !== null) onChange(centimes);
      }}
      onBlur={() => {
        editing.current = false;
        const trimmed = text.trim();
        if (trimmed === "") {
          onChange(null);
          setText("");
          return;
        }
        const centimes = parseAmountToCentimes(trimmed);
        if (centimes === null) {
          setText(shown(value));
          return;
        }
        onChange(centimes);
        setText(formatCentimes(centimes));
      }}
    />
  );
}
