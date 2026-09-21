// Small pieces the documents list and the open row both read: how a refusal
// is shown, and the day a document carries. Nothing here decides what to
// show, only how to say it.

import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

/** The refusal, wherever one is shown. One shape, so a failed list, a failed
 *  avoir and a failed sheet all read the same and all wear the danger role
 *  rather than a colour picked per call site. */
export function Refusal({ error }: { error: unknown }) {
  const { t } = useTranslation();
  return (
    <p role="alert" className="text-sm text-fg-danger">
      {t(errorKey(error))}
    </p>
  );
}

/** The day a document was issued, as the list column and the detail card
 *  both show it. The API answers `YYYY-MM-DD HH:MM:SS` already on the shop's
 *  calendar, so the day is the first ten characters and never a `Date` this
 *  screen builds: a parse here would drag the browser's timezone into a
 *  figure the core already decided (features.md, the shop's clock). */
export function day(issuedAt: string): string {
  return issuedAt.slice(0, 10);
}
