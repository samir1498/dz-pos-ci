// The stock recount block on the settings screen (features.md §1). The
// quantity on hand is derived from the movement ledger and cached on the
// product; the server compares the two every night, writes the ledger back
// over a cache that disagrees, and logs each correction. This panel says
// when that last happened and what it put right, and lets the owner ask for
// one now.
//
// Nothing here decides anything. The comparison, the repair and the
// once-a-day rule all live in the core; this file shows and translates.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { ApiError, formatQty } from "@dzpos/shared";
import type { StockDriftDto } from "@dzpos/shared";
import { api, productsQueryKey, stockRecountQueryKey } from "@/api";
import { useTranslation, type Key } from "@/i18n";

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
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

/** A difference reads with its sign: a correction that took stock off the
 *  fiche and one that put it back are the two things an owner is looking
 *  for, and an unsigned number would hide which happened. */
function signedQty(milli: number): string {
  return milli > 0 ? `+${formatQty(milli)}` : formatQty(milli);
}

export function StockRecountPanel() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const last = useQuery({
    queryKey: stockRecountQueryKey,
    queryFn: () => api.lastStockRecount(),
  });
  const [checked, setChecked] = useState<number | null>(null);
  const [done, setDone] = useState(false);
  const [serverError, setServerError] = useState<Key | null>(null);

  const recount = useMutation({
    mutationFn: () => api.recountStock(),
    onSuccess: async (report) => {
      setServerError(null);
      setDone(true);
      setChecked(report.products_checked);
      // The cached quantities were just put right, so the products screen
      // and every list that carries one are answering from a cache the
      // server no longer agrees with.
      await queryClient.invalidateQueries({ queryKey: productsQueryKey });
      await queryClient.invalidateQueries({ queryKey: stockRecountQueryKey });
    },
    onError: (error: unknown) => {
      setDone(false);
      setChecked(null);
      setServerError(errorKey(error));
    },
  });

  const drifts: StockDriftDto[] = last.data?.drifts ?? [];
  const day = last.data?.last_run_day ?? null;

  return (
    <section aria-labelledby="settings-stock-recount" className="flex flex-col gap-3 rounded border p-4">
      <h2 id="settings-stock-recount" className="font-semibold">
        {t("settings_stock_recount")}
      </h2>
      <p className="text-sm opacity-80">{t("settings_stock_recount_hint")}</p>

      {last.isPending ? <p>{t("products_loading")}</p> : null}
      {last.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(last.error))}
        </p>
      ) : null}

      {last.isSuccess ? (
        <>
          <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
            <dt>{t("stock_recount_last_run_label")}</dt>
            <dd data-testid="stock-recount-day">{day ?? t("stock_recount_never")}</dd>
            {checked === null ? null : (
              <>
                <dt>{t("stock_recount_checked_label")}</dt>
                <dd data-testid="stock-recount-checked">{checked}</dd>
              </>
            )}
          </dl>

          {day !== null && drifts.length === 0 ? (
            <p data-testid="stock-recount-clean">{t("stock_recount_none")}</p>
          ) : null}

          {drifts.length === 0 ? null : (
            <>
              {/* Said once above the list rather than on every row: the
                  correction is the same fact about all of them, and a shop
                  owner reading a column of quantities should not have to work
                  out which number is now on the fiche. */}
              <p>{t("stock_recount_corrected")}</p>
              <ul className="flex flex-col divide-y rounded border">
                {/* Keyed by position as well as by product: a day with two
                    runs reads as one list, and a product that drifted again
                    after the first run put it right is two rows carrying the
                    same id. The id alone made React drop the second one. The
                    list is replaced whole on every answer, so nothing is
                    carried across a render for the position to be wrong
                    about. */}
                {drifts.map((drift, position) => (
                  <li
                    key={`${drift.product_id}-${position}`}
                    data-testid="stock-drift-row"
                    className="flex items-center justify-between gap-4 px-3 py-2"
                  >
                    <span>{drift.name}</span>
                    {/* Quantities read left to right with Western digits
                        whatever the screen's language, the same decision the
                        products screen takes for a stock figure. */}
                    <span className="text-sm opacity-80" dir="ltr">
                      {t("stock_recount_cached")} {formatQty(drift.cached_milli)}
                    </span>
                    <span className="text-sm opacity-80" dir="ltr">
                      {t("stock_recount_ledger")} {formatQty(drift.ledger_milli)}
                    </span>
                    <span data-testid="stock-drift-difference" dir="ltr">
                      {t("stock_recount_difference")} {signedQty(drift.difference_milli)}
                    </span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </>
      ) : null}

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}
      {done && serverError === null ? <p role="status">{t("stock_recount_done")}</p> : null}

      {/* Said beside the button rather than in the panel's opening line: it
          is about the press, not about the nightly run, and the owner reads
          it while deciding whether to press. */}
      <p className="text-sm opacity-80">{t("stock_recount_marks_today")}</p>

      <div>
        <button
          type="button"
          className="rounded border px-3 py-1.5 disabled:opacity-50"
          disabled={recount.isPending}
          onClick={() => {
            setDone(false);
            setServerError(null);
            recount.mutate();
          }}
        >
          {recount.isPending ? t("action_saving") : t("action_recount_now")}
        </button>
      </div>
    </section>
  );
}
