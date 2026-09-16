// One heading shape for every settings panel, and the one helper that turns
// a validator's answer into words. Both were private to `routes/settings.tsx`
// until that file became a layout and its panels moved out to their own
// routes.

import { isKey, type Key } from "@/i18n";

/**
 * A panel's title. `h3` because `PageHeader` already carries the page's `h2`
 * and the shell owns the `h1`; the id is what the block's form or section
 * names itself by, so a test and a screen reader find the block the same way.
 */
export function PanelHeading({ id, children }: { id: string; children: string }) {
  return (
    <h3 id={id} className="font-semibold text-foreground">
      {children}
    </h3>
  );
}

/** Field validators return translation keys, never sentences. */
export function messageOf(messages: unknown[], t: (key: Key) => string): string | undefined {
  const key = messages.find((m): m is string => typeof m === "string");
  if (key === undefined) return undefined;
  return t(isKey(key) ? key : "error_unknown");
}
