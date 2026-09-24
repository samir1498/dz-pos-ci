import * as React from "react"
import { cn } from "@/lib/utils"

// Note: this carries `w-full`. Hiding an Input with `sr-only` does not
// work: `w-full` beats `sr-only`'s 1px width in the cascade and leaves a
// full-width invisible box that forces a page-level horizontal scrollbar
// (settings import picker, 2026-09-15). Hide with `hidden` instead.
//
// `::-ms-reveal`/`::-ms-clear` (T8, manual test 2026-09-24): Edge draws its
// own eye and its own clear button on a password field, and no other
// browser does, so a shop moving between machines saw two different fields.
// `PasswordInput` (components/PasswordInput.tsx) draws the one eye every
// browser shows now; this hides Edge's so the two never sit side by side.
function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        "h-9 w-full min-w-0 rounded-md border border-input bg-transparent px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none selection:bg-primary selection:text-primary-foreground file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
        "focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
        "aria-invalid:border-destructive aria-invalid:ring-destructive/20",
        "[&::-ms-reveal]:hidden [&::-ms-clear]:hidden",
        className
      )}
      {...props}
    />
  )
}

export { Input }
