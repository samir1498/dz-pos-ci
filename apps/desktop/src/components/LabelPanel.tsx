// The shelf label, previewed and printed. The core renders the page and
// this shows it in the sandboxed frame the ticket and the relevé use: same
// door, same empty sandbox, so a product name that ever slipped past the
// template's escaping still cannot reach this window.
//
// One component for the one label and for the A4 sheet, because they are the
// same page at two sizes and the caller is what says which.

import { useQuery } from "@tanstack/react-query";
import { ApiError } from "@dzpos/shared";
import { api } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";


/** The two ways a label is refused, told apart by the field the server
 * names. Both arrive as `validation`, and the shop does two different
 * things about them: a product with no code at all gets one typed or
 * generated, and a product carrying a supplier's own reference is not
 * going to get an EAN-13 label whatever anybody types. Saying "no
 * barcode" about a fiche that visibly has one sends the shop looking for
 * a field that is already filled. */
const FIELD_KEY: Record<string, Key> = {
  barcode: "error_label_no_barcode",
  barcode_digits: "error_label_not_ean13",
  ids: "error_label_too_many",
};


/**
 * Which sentence a refused label gets. Not a copy of the shared table: the
 * two ways a label is refused both arrive as `validation` and are told
 * apart by the field the server named, which no other screen does. Only the
 * fallback is shared, so a code added to `lib/fields.tsx` reaches here too,
 * and `validation` with no field it recognises means the one thing this
 * panel is ever asked about, a product with no barcode to print.
 */
function labelErrorKey(error: unknown): Key {
  if (error instanceof ApiError && error.code === "validation" && error.field !== undefined) {
    const named = FIELD_KEY[error.field];
    if (named !== undefined) return named;
  }
  return errorKey(error, { validation: "error_label_no_barcode" });
}

/** The label of one product, or a sheet of the products named. */
export type LabelAsk = { kind: "one"; id: number } | { kind: "sheet"; ids: readonly number[] };

/** The query key. The ids are part of it: two selections are two pages, and
 * a sheet cached for one must never be shown for the other. */
export function labelQueryKey(ask: LabelAsk, lang: string): readonly (string | number)[] {
  return ask.kind === "one"
    ? ["label", lang, ask.id]
    : ["label-sheet", lang, ...ask.ids.map(String)];
}

export function LabelPanel({ ask }: { ask: LabelAsk }) {
  const { t, lang } = useTranslation();
  const page = useQuery({
    queryKey: labelQueryKey(ask, lang),
    queryFn: () =>
      ask.kind === "one" ? api.getProductLabel(ask.id, lang) : api.getLabelSheet(ask.ids, lang),
  });

  return (
    <>
      {page.isPending ? <p className="text-muted-foreground">{t("products_loading")}</p> : null}
      {page.isError ? (
        <p role="alert" className="text-fg-danger">
          {t(labelErrorKey(page.error))}
        </p>
      ) : null}
      {page.isSuccess ? (
        <iframe
          title={t("labels_title")}
          srcDoc={page.data}
          // An empty sandbox: the page carries no script and needs no
          // origin, so it cannot reach this one.
          sandbox=""
          className="h-96 w-full rounded-md border border-border bg-card"
          data-testid="product-label"
        />
      ) : null}
    </>
  );
}
