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

const ERROR_KEY: Record<string, Key> = {
  validation: "error_label_no_barcode",
  not_found: "error_not_found",
  storage: "error_storage",
  restart_needed: "error_restart_needed",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unauthorized: "error_unauthorized",
  unreachable: "error_unreachable",
};

function errorKey(error: unknown): Key {
  if (error instanceof ApiError) return ERROR_KEY[error.code] ?? "error_unknown";
  return "error_unknown";
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
      {page.isPending ? <p>{t("products_loading")}</p> : null}
      {page.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(page.error))}
        </p>
      ) : null}
      {page.isSuccess ? (
        <iframe
          title={t("labels_title")}
          srcDoc={page.data}
          // An empty sandbox: the page carries no script and needs no
          // origin, so it cannot reach this one.
          sandbox=""
          className="h-96 w-full border-0"
          data-testid="product-label"
        />
      ) : null}
    </>
  );
}
