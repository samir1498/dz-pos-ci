// One choice among a few, three times on this screen: how the sale is paid,
// which document it becomes, and which sheet a facture is printed on.
//
// It was three groups of `<input type="radio">` and it is now a row of keys,
// because the kit's controls are the app's controls and a browser radio wears
// the platform's colour and focus ring on a counter screen. The semantics are
// kept rather than swapped: a `radiogroup` of `radio`s with `aria-checked` is
// the same thing a native group is, so a screen reader announces "2 of 3" and
// a test still asks for a radio and whether it is checked.
//
// What it does not carry is arrow-key roving. A native group is one tab stop
// and the arrows move inside it; here each key is its own tab stop. That is
// the honest trade of the swap and it is written down rather than hidden
// (the report says so too).

import { useId, type ReactNode } from "react";

import { Button } from "@/components/ui/button";

/** The label above the row, and the row. */
export function ChoiceGroup({
  label,
  children,
  className,
}: {
  label: string;
  children: ReactNode;
  className?: string;
}) {
  const id = useId();
  return (
    <div className={className}>
      <span id={id} className="mb-1.5 block text-sm text-muted-foreground">
        {label}
      </span>
      <div role="radiogroup" aria-labelledby={id} className="flex flex-wrap gap-2">
        {children}
      </div>
    </div>
  );
}

/** One key of the row. `title` is the reason it is off, said on the control
 * itself so hovering the thing that refuses explains it. */
export function Choice({
  checked,
  label,
  title,
  disabled = false,
  onPick,
}: {
  checked: boolean;
  label: string;
  title?: string;
  disabled?: boolean;
  onPick: () => void;
}) {
  return (
    <Button
      type="button"
      role="radio"
      aria-checked={checked}
      disabled={disabled}
      title={title}
      variant="outline"
      className={checked ? "border-primary bg-primary-soft text-primary" : undefined}
      onClick={onPick}
    >
      {label}
    </Button>
  );
}
