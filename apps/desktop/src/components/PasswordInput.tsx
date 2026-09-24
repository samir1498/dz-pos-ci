// A password field with one eye button, ours, because the browsers
// disagree with each other about theirs: Edge draws a native reveal,
// Chrome, Brave and Firefox draw none (manual test 2026-09-24, T8). Hiding
// Edge's (`ui/input.tsx`, `::-ms-reveal`/`::-ms-clear`) and drawing this one
// everywhere means the field looks the same field wherever the shop signs
// in from.
//
// A plain wrapper around `Input`, not a new field type: everything a caller
// already passes (`id`, `aria-invalid`, `aria-describedby`, `value`,
// `onChange`, `disabled`, `autoComplete`) lands on the real input exactly as
// before, `FormField`'s render-prop included. Only `type` is this file's to
// decide, which is why it is left out of the props a caller can pass.

import { Eye, EyeOff } from "lucide-react";
import { useState, type ComponentProps } from "react";

import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useTranslation } from "@/i18n";
import { cn } from "@/lib/utils";

/** `Omit` drops the input element's `data-${string}` index signature (a
 *  known TypeScript limitation: a mapped type built from `Exclude<keyof T,
 *  K>` cannot keep a generic template-literal key), so the one attribute
 *  every caller here passes is named back in by hand. */
type PasswordInputProps = Omit<ComponentProps<typeof Input>, "type"> & {
  readonly "data-testid"?: string;
};

export function PasswordInput({
  className,
  disabled,
  "data-testid": testId,
  ...props
}: PasswordInputProps) {
  const { t } = useTranslation();
  const [shown, setShown] = useState(false);

  return (
    <div className="relative">
      <Input
        {...props}
        data-testid={testId}
        disabled={disabled}
        type={shown ? "text" : "password"}
        className={cn("pe-9", className)}
      />
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        data-testid={testId === undefined ? undefined : `${testId}-toggle`}
        aria-label={t(shown ? "password_hide" : "password_show")}
        aria-pressed={shown}
        disabled={disabled}
        // No `tabIndex={-1}`: a keyboard-only person reaches this the same
        // way a mouse does, right after the field it belongs to.
        className="absolute inset-y-0 end-0.5 my-auto text-muted-foreground hover:bg-transparent hover:text-foreground"
        onClick={() => setShown((current) => !current)}
      >
        <Icon as={shown ? EyeOff : Eye} size={18} />
      </Button>
    </div>
  );
}
