// A labelled control with room for a hint and a refusal.
//
// The id is generated here and handed to the child through a render prop
// rather than asked of the caller. Every screen that wrote its own field
// invented its own id, and two of them on one page (a customer form beside a
// supplier form) collided, which quietly points a label at the wrong input.
// `useId` cannot collide, and the caller cannot forget to wire it because the
// id arrives as an argument.
//
// The error is `role="alert"` and the control carries `aria-invalid` and
// `aria-describedby`, so a screen reader hears the refusal without the user
// hunting for the red text. Hint and error share the description: when both
// are present the error goes first, because it is the newer fact.
//
// Nothing here is left or right. The label sits above the control and the
// text starts on the reading side, which is the same rule in the three
// languages.

import { useId, type ReactNode } from "react";

import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

/** What a control needs to be reachable from its label and its messages. */
export interface FieldParts {
  readonly id: string;
  readonly "aria-invalid": boolean;
  readonly "aria-describedby": string | undefined;
  /** The label's own id. A control that is one box ignores it, because
   *  `htmlFor` already names it. A control made of several boxes cannot be
   *  named that way: `htmlFor` reaches one box and an `aria-label` on that
   *  box would beat the field's label and hide it, so those controls put
   *  this on a group around all of them. */
  readonly "aria-labelledby": string;
}

export function FormField({
  label,
  hint,
  error,
  required = false,
  className,
  children,
}: {
  label: string;
  /** Said before the mistake: a format, a unit, what blank means. */
  hint?: string;
  /** The refusal, already translated. `undefined` is "nothing wrong". */
  error?: string;
  required?: boolean;
  className?: string;
  children: (parts: FieldParts) => ReactNode;
}) {
  const id = useId();
  const labelId = `${id}-label`;
  const hintId = `${id}-hint`;
  const errorId = `${id}-error`;
  const described = [error === undefined ? null : errorId, hint === undefined ? null : hintId]
    .filter((part): part is string => part !== null)
    .join(" ");

  return (
    <div className={cn("flex flex-col gap-1.5 text-start", className)}>
      {/* The star sits beside the label, not inside it: a label's text is its
          name to a test and to a screen reader, and "Nom*" is not "Nom". */}
      <div className="flex items-baseline gap-1">
        <Label id={labelId} htmlFor={id}>
          {label}
        </Label>
        {required ? (
          <span aria-hidden="true" className="text-fg-danger">
            *
          </span>
        ) : null}
      </div>
      {children({
        id,
        "aria-labelledby": labelId,
        "aria-invalid": error !== undefined,
        "aria-describedby": described === "" ? undefined : described,
      })}
      {error === undefined ? null : (
        <p id={errorId} role="alert" className="text-sm text-fg-danger">
          {error}
        </p>
      )}
      {hint === undefined ? null : (
        <p id={hintId} className="text-sm text-muted-foreground">
          {hint}
        </p>
      )}
    </div>
  );
}
