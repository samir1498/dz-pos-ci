// Four boxes for a four-digit PIN, the two faces of it in one place.
//
// `PinBoxes` is the typed one (the users dialogs). input-otp keeps a real
// `<input>` under the boxes, so the field's label, the numeric keyboard and
// paste still reach it, and the browser is told it is a one-time code and
// not a password: a password field is what makes Brave and Chrome offer the
// shop's saved login over a PIN box, and `autoComplete="off"` alone is
// ignored by both. Never `type="number"`, which unmasks, grows spinner
// arrows and turns `0512` into `512`, a different PIN.
//
// `PinDots` is the drawn one (sign-in pad, lock screen). The keypad owns the
// digits there and these boxes only show how many are in, so a cashier sees
// which box is next without a caret to find.
//
// Both mask: a shoulder behind the counter reads a row of dots, never the
// digits it is made of. Both read left to right whatever the UI language,
// the way a card terminal does.

import { PIN_DIGITS } from "@dzpos/shared";
import { REGEXP_ONLY_DIGITS } from "input-otp";

import type { FieldParts } from "@/components/FormField";
import { InputOTP, InputOTPGroup, InputOTPSlot } from "@/components/ui/input-otp";
import { cn } from "@/lib/utils";

const SLOTS: readonly number[] = Array.from({ length: PIN_DIGITS }, (_, index) => index);

const SLOT_SIZE = "h-12 w-12 text-xl font-numeric";

export function PinBoxes({
  value,
  onChange,
  autoFocus = false,
  disabled = false,
  ...parts
}: Partial<FieldParts> & {
  value: string;
  onChange: (next: string) => void;
  autoFocus?: boolean;
  disabled?: boolean;
}) {
  return (
    <InputOTP
      {...parts}
      value={value}
      onChange={onChange}
      maxLength={PIN_DIGITS}
      pattern={REGEXP_ONLY_DIGITS}
      inputMode="numeric"
      autoComplete="one-time-code"
      autoFocus={autoFocus}
      disabled={disabled}
    >
      <InputOTPGroup dir="ltr" className="gap-2">
        {SLOTS.map((index) => (
          <InputOTPSlot
            key={index}
            index={index}
            mask
            className={cn(SLOT_SIZE, "rounded-md border-l")}
          />
        ))}
      </InputOTPGroup>
    </InputOTP>
  );
}

export function PinDots({ filled, testId }: { filled: number; testId: string }) {
  return (
    <div data-testid={testId} dir="ltr" className="flex items-center gap-2">
      {SLOTS.map((index) => (
        <div
          key={index}
          data-active={index === filled}
          className={cn(
            SLOT_SIZE,
            "flex items-center justify-center rounded-md border border-input bg-muted shadow-xs",
            "data-[active=true]:border-ring data-[active=true]:ring-[3px] data-[active=true]:ring-ring/50",
          )}
        >
          {index < filled ? "•" : ""}
        </div>
      ))}
    </div>
  );
}
