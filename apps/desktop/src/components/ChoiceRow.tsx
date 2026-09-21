// A choice between two or three named things. It started in
// `routes/-customers/parts.tsx` for the party kind and the payment mode, and
// moved here when the documents dialogs needed the same row for how a
// reversal is settled: a screen folder is not a place another screen folder
// reaches into (frontend-conventions.md, "Two folders, one rule about which
// is which").

import { useId } from "react";

import { Button } from "@/components/ui/button";

/**
 * A choice between two or three named things: the party kind, the payment
 * mode, how the money goes back on a reversal. It is a row of kit buttons
 * wearing `role="radio"` inside a `radiogroup` rather than
 * `<input type="radio">`, which the kit's lint rule refuses and which wears
 * the browser's own dot on every theme.
 *
 * The accessible name of each option is its own label, so a test and a
 * screen reader both find "Entreprise" rather than "option 1 of 2".
 */
export function ChoiceRow<Value extends string>({
  label,
  options,
  value,
  onChange,
}: {
  label: string;
  readonly options: readonly { readonly value: Value; readonly label: string }[];
  value: Value;
  onChange: (next: Value) => void;
}) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1.5 text-start">
      <span id={id} className="text-sm font-medium text-foreground">
        {label}
      </span>
      <div role="radiogroup" aria-labelledby={id} className="flex flex-wrap gap-2">
        {options.map((option) => (
          <Button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={value === option.value}
            variant={value === option.value ? "secondary" : "outline"}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </Button>
        ))}
      </div>
    </div>
  );
}
