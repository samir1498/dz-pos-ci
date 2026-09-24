// The sheet a sale prints, pulled out of till.tsx so that file stays under
// its line cap (scripts/file-sizes.json) rather than growing past it for a
// feature this component owns end to end: the preview, the paper choice on
// a facture, and now the print itself.

import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import type { DocumentKindDto, PrintPaper } from "@dzpos/shared";

import { api, saleFactureQueryKey, saleTicketQueryKey } from "@/api";
import { Skeleton } from "@/components/ui/skeleton";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

import { Choice, ChoiceGroup } from "./choice";

/**
 * The ticket itself, not a screen that resembles it. `GET /sales/{id}/ticket`
 * hands back the 80 mm page the core rendered from the stored document, and
 * that page goes into an iframe as it came: the cashier is looking at what
 * the printer will put on paper, down to the rounding, rather than at a
 * second rendering of the same numbers that could disagree with it.
 *
 * The language named here is the till's own, but a shop with a stored print
 * language keeps every fiscal paper in it regardless of the screen a
 * cashier has open (`context/plans/20260920-a-print-language-the-shop-keeps.md`).
 *
 * `srcDoc` rather than a `src` URL: the page arrives as a string the client
 * already fetched with the launch token, and an iframe pointed at the route
 * would ask for it again without one.
 *
 * `printRequest` is what turns this from a preview into paper: it is the
 * count of "Imprimer" clicks the till has made, and the effect below opens
 * the browser's print dialog on this frame's own window the moment the
 * frame both holds the document this request is for and has caught up to
 * that count. Neither on its own is enough — a click that lands before the
 * fetch resolves must wait for the frame to load, and a frame that reloads
 * for its own reason (the a4/a5 switch below) must never print on its own.
 */
export function Receipt({
  id,
  kind,
  paper,
  onPaper,
  printRequest,
}: {
  id: number;
  kind: DocumentKindDto;
  paper: PrintPaper;
  onPaper: (paper: PrintPaper) => void;
  printRequest: number;
}) {
  const { t, lang } = useTranslation();
  const facture = kind === "facture";
  const page = useQuery({
    queryKey: facture ? saleFactureQueryKey(id, lang, paper) : saleTicketQueryKey(id, lang),
    queryFn: () =>
      facture ? api.getSaleFacture(id, lang, paper) : api.getSaleTicket(id, lang),
  });
  const iframeRef = useRef<HTMLIFrameElement>(null);
  // Whether the frame currently on screen has finished loading the page
  // `page.data` holds. Reset the moment that page changes (a fresh sale, or
  // a paper switch fetching the other sheet) so a load event left over from
  // what used to be in the frame cannot be read as "ready" for this one.
  const [loaded, setLoaded] = useState(false);
  useEffect(() => setLoaded(false), [page.data]);
  // The highest `printRequest` already sent to the printer, so a paper
  // switch (which flips `loaded` back on once the new sheet arrives) does
  // not reprint a request this frame already served.
  const printedRef = useRef(0);
  useEffect(() => {
    if (loaded && printRequest > printedRef.current) {
      printedRef.current = printRequest;
      iframeRef.current?.contentWindow?.print();
    }
  }, [loaded, printRequest]);
  return (
    <section aria-label={t("till_receipt")} className="flex flex-col gap-2">
      <strong>{t("till_receipt")}</strong>
      {/* A4 is what a facture is filed on; the A5 half sheet is the one a
          counter printer is loaded with. The same page either way: the
          sheet changes the @page size the core writes and nothing else
          (features.md §4). */}
      {facture ? (
        <ChoiceGroup label={t("till_paper")}>
          <Choice
            checked={paper === "a4"}
            label={t("till_paper_a4")}
            onPick={() => onPaper("a4")}
          />
          <Choice
            checked={paper === "a5"}
            label={t("till_paper_a5")}
            onPick={() => onPaper("a5")}
          />
        </ChoiceGroup>
      ) : null}
      {page.isPending ? <Skeleton className="h-96 w-full" /> : null}
      {page.isError ? (
        <p role="alert" className="text-fg-danger">
          {t(errorKey(page.error))}
        </p>
      ) : null}
      {page.isSuccess ? (
        <iframe
          ref={iframeRef}
          title={t("till_receipt")}
          srcDoc={page.data}
          onLoad={() => setLoaded(true)}
          // `allow-same-origin allow-modals` and nothing else: the page
          // still carries no script (`allow-scripts` stays out, so nothing
          // it renders can ever run one even if a product name slipped past
          // the template's escaping), but printing needs both grants —
          // `window.print()` on a frame's own opaque origin throws, and
          // even same-origin a sandboxed frame ignores `print()` without
          // `allow-modals` (it is one of the sandboxed-modals APIs, beside
          // `alert`/`confirm`). Neither grant is worth anything without
          // `allow-scripts` to exercise it, which this frame never has.
          sandbox="allow-same-origin allow-modals"
          className="h-96 w-full rounded-md border border-border bg-background"
          data-testid={facture ? "till-facture" : "till-ticket"}
        />
      ) : null}
    </section>
  );
}
