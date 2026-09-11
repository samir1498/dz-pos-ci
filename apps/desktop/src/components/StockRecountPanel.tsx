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
import { ScanLine } from "lucide-react";
import { useState } from "react";
import { ApiError, formatQty } from "@dzpos/shared";
import type { StockDriftDto } from "@dzpos/shared";
import { api, productsQueryKey, stockRecountQueryKey } from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader } from "@/components/ui/card";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";



/** A difference reads with its sign: a correction that took stock off the
 *  fiche and one that put it back are the two things an owner is looking
 *  for, and an unsigned number would hide which happened. */
function signedQty(milli: number): string {
  return milli > 0 ? `+${formatQty(milli)}` : formatQty(milli);
}

/** A quantity reads left to right with Western digits whatever the screen's
 *  language, the same decision the products screen takes for a stock figure.
 *  The table already aligns the column on the reading end. */
function Qty({ children, testId }: { children: string; testId?: string }) {
  return (
    <span dir="ltr" data-testid={testId} className="inline-block">
      {children}
    </span>
  );
}

/**
 * A drift with its place in the list. A day with two runs reads as one list,
 * and a product that drifted again after the first run put it right is two
 * rows carrying the same product id, so the position is part of what makes a
 * row itself.
 */
type NumberedDrift = StockDriftDto & { readonly position: number };

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

  const columns: readonly Column<NumberedDrift>[] = [
    { id: "name", header: t("stock_recount_product_header"), cell: (drift) => drift.name },
    {
      id: "cached",
      header: t("stock_recount_cached"),
      numeric: true,
      cell: (drift) => <Qty>{formatQty(drift.cached_milli)}</Qty>,
    },
    {
      id: "ledger",
      header: t("stock_recount_ledger"),
      numeric: true,
      cell: (drift) => <Qty>{formatQty(drift.ledger_milli)}</Qty>,
    },
    {
      id: "difference",
      header: t("stock_recount_difference"),
      numeric: true,
      cell: (drift) => <Qty testId="stock-drift-difference">{signedQty(drift.difference_milli)}</Qty>,
    },
  ];

  return (
    <section aria-labelledby="settings-stock-recount">
      <Card>
        <CardHeader>
          <h3 id="settings-stock-recount" className="font-semibold text-foreground">
            {t("settings_stock_recount")}
          </h3>
          <CardDescription>{t("settings_stock_recount_hint")}</CardDescription>
        </CardHeader>

        <CardContent className="flex flex-col gap-4">
          {last.isPending ? (
            <p className="text-sm text-muted-foreground">{t("settings_loading")}</p>
          ) : null}
          {last.isError ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(errorKey(last.error))}
            </p>
          ) : null}

          {last.isSuccess ? (
            <>
              <dl className="grid grid-cols-[auto_1fr] items-baseline gap-x-4 gap-y-1">
                <dt className="text-sm text-muted-foreground">
                  {t("stock_recount_last_run_label")}
                </dt>
                <dd data-testid="stock-recount-day" className="font-medium text-foreground">
                  {day === null ? (
                    t("stock_recount_never")
                  ) : (
                    <span dir="ltr" className="font-numeric tabular-nums">
                      {day}
                    </span>
                  )}
                </dd>
                {checked === null ? null : (
                  <>
                    <dt className="text-sm text-muted-foreground">
                      {t("stock_recount_checked_label")}
                    </dt>
                    <dd
                      data-testid="stock-recount-checked"
                      dir="ltr"
                      className="font-medium tabular-nums text-foreground"
                    >
                      {checked}
                    </dd>
                  </>
                )}
              </dl>

              {day !== null && drifts.length === 0 ? (
                <EmptyState
                  data-testid="stock-recount-clean"
                  icon={ScanLine}
                  title={t("stock_recount_none")}
                />
              ) : null}

              {drifts.length === 0 ? null : (
                <>
                  {/* Said once above the list rather than on every row: the
                      correction is the same fact about all of them, and a shop
                      owner reading a column of quantities should not have to work
                      out which number is now on the fiche. */}
                  <p className="text-sm text-muted-foreground">{t("stock_recount_corrected")}</p>
                  {/* Keyed by position as well as by product: a day with two
                      runs reads as one list, and a product that drifted again
                      after the first run put it right is two rows carrying the
                      same id. The id alone made React drop the second one. The
                      list is replaced whole on every answer, so nothing is
                      carried across a render for the position to be wrong
                      about. */}
                  <DataTable
                    data-testid="stock-drift-table"
                    caption={t("settings_stock_recount")}
                    columns={columns}
                    rows={drifts.map((drift, position) => ({ ...drift, position }))}
                    rowKey={(drift) => `${drift.product_id}-${drift.position}`}
                  />
                </>
              )}
            </>
          ) : null}

          {/* Said beside the button rather than in the panel's opening line: it
              is about the press, not about the nightly run, and the owner reads
              it while deciding whether to press. */}
          <p className="text-sm text-muted-foreground">{t("stock_recount_marks_today")}</p>
        </CardContent>

        <CardFooter className="flex flex-wrap items-center gap-3">
          <Button
            type="button"
            disabled={recount.isPending}
            onClick={() => {
              setDone(false);
              setServerError(null);
              recount.mutate();
            }}
          >
            {recount.isPending ? t("action_saving") : t("action_recount_now")}
          </Button>
          {serverError !== null ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(serverError)}
            </p>
          ) : null}
          {done && serverError === null ? (
            <p role="status" className="text-sm text-fg-success">
              {t("stock_recount_done")}
            </p>
          ) : null}
        </CardFooter>
      </Card>
    </section>
  );
}
