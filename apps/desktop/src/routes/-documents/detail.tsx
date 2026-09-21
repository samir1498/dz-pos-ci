// One open document: the lines it was issued with, the totals a comptable
// reads, the balance block when it carries one, and the sheet the core
// rendered beside them.
//
// Two things are done from here and nowhere else, because both are about a
// paper that already exists: writing an avoir against a facture, and
// annulling a document. Neither decides anything. What a line has left to
// credit, what a cancellation puts back and whether an avoir is issued with
// it are all the core's answers; this file collects a quantity and a reason
// and shows what came back.
//
// Both of those are dialogs rather than panels that grow inside the page.
// They are the two irreversible acts the app has, they each need a reason
// typed in, and a form that pushes the document's own figures off the screen
// while it is filled in is a form somebody confirms without reading what it
// is about.

import type { QueryClient } from "@tanstack/react-query";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ApiError, formatCentimes, formatQty } from "@dzpos/shared";
import type {
  AvoirLineDto,
  DocumentKindDto,
  PrintPaper,
  SaleCancelEffectDto,
  SaleDto,
} from "@dzpos/shared";
import { Ban, Undo2, X } from "lucide-react";
import { useState } from "react";

import {
  api,
  customerLedgerQueryKey,
  customerPaymentsQueryKey,
  customersQueryKey,
  productsQueryKey,
  saleAvoirsQueryKey,
  saleFactureQueryKey,
  saleSheetPrefixes,
  saleTicketQueryKey,
  saleQueryKey,
  salesQueryPrefix,
} from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { StatusPill } from "@/components/StatusPill";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useTranslation, type Key } from "@/i18n";

import { RefundChoice, Refusal, day, refundField, type Settlement } from "./parts";

export function DocumentDetail({ id, onClose }: { id: number; onClose: () => void }) {
  const { t } = useTranslation();
  const [paper, setPaper] = useState<PrintPaper>("a4");
  const document = useQuery({ queryKey: saleQueryKey(id), queryFn: () => api.getSale(id) });

  if (document.isPending) return <Skeleton className="h-64 w-full" />;
  if (document.isError) return <Refusal error={document.error} />;

  const doc = document.data;
  const lines: readonly Column<SaleDto["lines"][number]>[] = [
    { id: "name", header: t("documents_line"), cell: (l) => l.name },
    {
      id: "qty",
      header: t("documents_qty"),
      numeric: true,
      cell: (l) => <span dir="ltr">{formatQty(l.qty_milli)}</span>,
    },
    {
      id: "unit",
      header: t("documents_unit_price"),
      money: true,
      cell: (l) => <Money centimes={l.unit_price_centimes} />,
    },
    {
      id: "total",
      header: t("documents_line_total"),
      money: true,
      cell: (l) => <Money centimes={l.line_total_centimes} />,
    },
  ];

  // A section rather than the Card's own div: the region role is what the
  // flows find the open document by, and a div carries none.
  return (
    <section aria-label={t("documents_detail")}>
      <Card>
        <CardHeader>
          <CardTitle className="flex flex-wrap items-center gap-2">
            <span className="font-numeric tabular-nums">{doc.printed_number}</span>
            <StatusPill status={doc.status === "cancelled" ? "cancelled" : "issued"} />
          </CardTitle>
          <CardAction>
            <Button variant="ghost" size="sm" onClick={onClose}>
              <Icon as={X} size={18} />
              {t("documents_close")}
            </Button>
          </CardAction>
        </CardHeader>

        <CardContent className="flex flex-col gap-4">
          {doc.cancellation === null ? null : (
            <p className="rounded-md bg-danger-soft px-3 py-2 text-sm text-fg-danger">
              {t("documents_cancelled_on")} {day(doc.cancellation.cancelled_at)} :{" "}
              {doc.cancellation.reason}
            </p>
          )}

          <DataTable
            columns={lines}
            rows={doc.lines}
            rowKey={(l) => l.id}
            caption={t("documents_lines")}
          />

          {/* At the end of the row, so the figures land under the table's own
              money column, which is the last one in both directions. */}
          <dl
            aria-label={t("documents_totals")}
            className="grid w-full grid-cols-2 gap-1 text-sm sm:ms-auto sm:w-80"
          >
            <dt className="text-muted-foreground">{t("documents_total_ht")}</dt>
            <dd className="text-end">
              <Money centimes={doc.totals.total_ht_centimes} />
            </dd>
            <dt className="text-muted-foreground">{t("documents_tva")}</dt>
            <dd className="text-end">
              <Money centimes={doc.totals.tva_centimes} />
            </dd>
            <dt className="text-muted-foreground">{t("documents_stamp")}</dt>
            <dd className="text-end">
              <Money centimes={doc.totals.stamp_centimes} />
            </dd>
            <dt className="font-medium text-foreground">{t("documents_net")}</dt>
            <dd className="text-end">
              <Money centimes={doc.totals.net_to_pay_centimes} />
            </dd>
          </dl>

          {doc.balance === null ? null : (
            <dl
              aria-label={t("documents_balance")}
              className="grid w-full grid-cols-2 gap-1 text-sm sm:ms-auto sm:w-80"
            >
              <dt className="text-muted-foreground">{t("documents_old_balance")}</dt>
              <dd className="text-end">
                <Money centimes={doc.balance.old_balance_centimes} />
              </dd>
              <dt className="text-muted-foreground">{t("documents_remaining_debt")}</dt>
              <dd className="text-end">
                <Money centimes={doc.balance.remaining_debt_centimes} />
              </dd>
              <dt className="text-muted-foreground">{t("documents_total_debt")}</dt>
              <dd className="text-end">
                <Money centimes={doc.balance.total_debt_centimes} />
              </dd>
            </dl>
          )}

          {doc.kind === "facture" ? <AvoirPanel facture={doc} /> : null}
          {doc.status === "issued" && doc.kind !== "avoir" && doc.kind !== "proforma" ? (
            <CancelPanel document={doc} />
          ) : null}

          <PrintPanel id={doc.id} kind={doc.kind} paper={paper} onPaper={setPaper} />
        </CardContent>
      </Card>
    </section>
  );
}

/** The credit notes already written against a facture, and the dialog that
 *  writes another. The quantity a line offers is what the core says is left
 *  on it: what the shop has already taken back is subtracted here from the
 *  avoirs the API answered with, so the box cannot be typed past it and the
 *  refusal, when it comes, is the core's and not a second rule. */
function AvoirPanel({ facture }: { facture: SaleDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const [qty, setQty] = useState<Record<number, string>>({});
  // "No notes move" until somebody says otherwise, which is what the wire
  // carried before the field existed.
  const [settlement, setSettlement] = useState<Settlement>("ledger");
  const avoirs = useQuery({
    queryKey: saleAvoirsQueryKey(facture.id),
    queryFn: () => api.listAvoirs(facture.id),
  });

  const write = useMutation({
    mutationFn: (lines: AvoirLineDto[] | null) =>
      api.createAvoir(facture.id, {
        lines,
        reason: reason.trim() === "" ? null : reason.trim(),
        ...refundField(settlement),
      }),
    onSuccess: async () => {
      setOpen(false);
      setQty({});
      setReason("");
      setSettlement("ledger");
      await everythingItTouched(queryClient, facture.id, facture.customer_id);
    },
  });

  // What each line has left, from the avoirs already written against this
  // facture. One pass over their lines, matched by the facture line each
  // names: the two are matched by id and never by product (features.md §3).
  const credited = new Map<number, number>();
  for (const avoir of avoirs.data ?? []) {
    for (const line of avoir.lines) {
      if (line.ref_line_id === null) continue;
      credited.set(line.ref_line_id, (credited.get(line.ref_line_id) ?? 0) + line.qty_milli);
    }
  }
  const left = (lineId: number, sold: number): number => sold - (credited.get(lineId) ?? 0);

  return (
    <section
      aria-label={t("documents_avoirs")}
      className="flex flex-col gap-3 rounded-lg border border-border p-3"
    >
      <h3 className="text-sm font-semibold text-foreground">{t("documents_avoirs")}</h3>

      {avoirs.isSuccess && avoirs.data.length > 0 ? (
        <ul className="flex flex-col gap-1 text-sm">
          {avoirs.data.map((a) => (
            <li key={a.id} className="flex items-center justify-between gap-3">
              <span className="font-numeric tabular-nums">{a.printed_number}</span>
              <Money centimes={a.totals.net_to_pay_centimes} />
            </li>
          ))}
        </ul>
      ) : null}

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <Button variant="outline" size="sm" className="self-start">
            <Icon as={Undo2} size={18} flip />
            {t("documents_avoir_new")}
          </Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("documents_avoir_new")}</DialogTitle>
            <DialogDescription>{t("documents_avoir_hint")}</DialogDescription>
          </DialogHeader>
          <form
            className="flex flex-col gap-3"
            onSubmit={(e) => {
              e.preventDefault();
              // Only the lines somebody typed a quantity into travel; an
              // untouched line is not a line of zero, it is a line the shop is
              // not crediting.
              const lines: AvoirLineDto[] = facture.lines
                .map((l) => ({
                  document_line_id: l.id,
                  qty_milli: Math.round(Number(qty[l.id] ?? "") * 1000),
                }))
                .filter((l) => Number.isFinite(l.qty_milli) && l.qty_milli > 0);
              write.mutate(lines);
            }}
          >
            {facture.lines.map((l) => (
              <FormField
                key={l.id}
                label={`${t("documents_avoir_qty")} ${l.name}`}
                hint={`${t("documents_left")} ${formatQty(left(l.id, l.qty_milli))}`}
              >
                {(parts) => (
                  <Input
                    {...parts}
                    dir="ltr"
                    type="number"
                    min={0}
                    max={left(l.id, l.qty_milli) / 1000}
                    step="0.001"
                    className="font-numeric tabular-nums text-end"
                    value={qty[l.id] ?? ""}
                    onChange={(e) => setQty({ ...qty, [l.id]: e.target.value })}
                  />
                )}
              </FormField>
            ))}

            {/* Not required: an avoir with no reason is what the API takes
                when the shop has nothing to add, and the wire carries a null
                for it. */}
            <FormField label={t("documents_reason")}>
              {(parts) => (
                <Input {...parts} value={reason} onChange={(e) => setReason(e.target.value)} />
              )}
            </FormField>

            {/* The facture decides what is on offer, not the avoir being
                written: a credit facture's money is on the account however
                much of it comes back. No `figures`: what leaves the drawer is
                the avoir's own `net_to_pay`, which carries no stamp and
                depends on the quantities above, so the facture's totals would
                be the wrong number on the first avoir already. */}
            <RefundChoice document={facture} value={settlement} onChange={setSettlement} />

            {write.isError ? <Refusal error={write.error} /> : null}

            <DialogFooter>
              {/* The whole of what is left, which is the button a shop reaches
                  for when the customer brought everything back. `null` lines
                  is what says so on the wire. */}
              <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
                {t("documents_cancel_action")}
              </Button>
              <Button type="button" variant="outline" onClick={() => write.mutate(null)}>
                {t("documents_avoir_whole")}
              </Button>
              <Button type="submit">{t("documents_avoir_write")}</Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </section>
  );
}

/** The sentence for one effect. The amount is put into the sentence rather
 *  than after it, because the three languages do not agree on where in the
 *  line a figure belongs: Arabic reads it mid-sentence. It is also the one
 *  amount on this screen that does not go through `Money`: it is a word in a
 *  sentence there, not a figure in a column, and an element around it would
 *  cut the sentence a screen reader reads in two. */
function says(t: (k: Key) => string, effect: SaleCancelEffectDto): string {
  switch (effect.effect) {
    case "nothing_to_reverse":
      return t("documents_cancel_nothing");
    case "stock_back":
      return t("documents_cancel_stock_only");
    case "stock_back_and_avoir":
      return t("documents_cancel_with_avoir").replace(
        "{amount}",
        formatCentimes(effect.amount_centimes),
      );
  }
}

/** Everything an avoir or a cancellation makes stale, in one place because
 *  the two make the same things stale.
 *
 *  Both write in one transaction across four tables: the document's own row,
 *  the goods, the ledger and the allocations on the customer's other papers.
 *  So the document, the list it sits in, the credit notes under it, the pages
 *  the core rendered from it, the customer's own figures and the stock all
 *  read differently afterwards, and anything left in the cache is a screen
 *  showing a file that no longer exists.
 *
 *  The stock is why the products list is here: an avoir puts the goods back,
 *  and a shop looking at a count of eight while the shelf holds eleven orders
 *  more of it. */
async function everythingItTouched(
  queryClient: QueryClient,
  documentId: number,
  customerId: number | null,
): Promise<void> {
  const keys: readonly (readonly (string | number | undefined)[])[] = [
    saleQueryKey(documentId),
    saleAvoirsQueryKey(documentId),
    salesQueryPrefix(),
    ...saleSheetPrefixes(documentId),
    customersQueryKey,
    productsQueryKey,
    ...(customerId === null
      ? []
      : [customerLedgerQueryKey(customerId), customerPaymentsQueryKey(customerId)]),
  ];
  await Promise.all(keys.map((queryKey) => queryClient.invalidateQueries({ queryKey })));
}

/** Annulling a document. The confirm says what will happen, and it says a
 *  different thing when the document put money on a customer's account: the
 *  goods come back either way, and a facture carrying debt is undone by an
 *  avoir the core writes with it (features.md §3). */
function CancelPanel({ document }: { document: SaleDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const [settlement, setSettlement] = useState<Settlement>("ledger");
  const cancel = useMutation({
    mutationFn: () => api.cancelSale(document.id, { reason, ...refundField(settlement) }),
    onSuccess: async () => {
      setOpen(false);
      setReason("");
      setSettlement("ledger");
      await everythingItTouched(queryClient, document.id, document.customer_id);
    },
  });
  // The server's own answer, never re-derived here. A facture whose goods have
  // all come back on earlier credit notes carries debt, was sold on credit and
  // names a customer, and cancelling it does nothing at all: every field this
  // screen could read says the opposite of what will happen.
  //
  // Absent while the document is still loading, which is the one case with
  // nothing to say yet.
  const effect = document.cancel_effect;

  return (
    // No heading of its own: it would say the same words as the button under
    // it. What the row carries instead is the sentence that says what the
    // button costs, because this is the one act on the screen nobody undoes.
    <section
      aria-label={t("documents_cancel")}
      className="flex flex-wrap items-center gap-3 border-t border-border pt-4"
    >
      <p className="text-sm text-muted-foreground">{t("documents_cancel_hint")}</p>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogTrigger asChild>
          <Button variant="outline" size="sm" className="ms-auto">
            <Icon as={Ban} size={18} />
            {t("documents_cancel")}
          </Button>
        </DialogTrigger>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("documents_cancel")}</DialogTitle>
            <DialogDescription>{t("documents_cancel_hint")}</DialogDescription>
          </DialogHeader>
          <form
            className="flex flex-col gap-3"
            onSubmit={(e) => {
              e.preventDefault();
              cancel.mutate();
            }}
          >
            {/* The one live region on the screen, and it is inside the dialog
                on purpose: it is the sentence somebody has to have read
                before they confirm. */}
            <p role="status" className="rounded-md bg-warn-soft px-3 py-2 text-sm text-warn">
              {effect === null ? t("products_loading") : says(t, effect)}
            </p>

            <FormField label={t("documents_reason")}>
              {(parts) => (
                <Input {...parts} value={reason} onChange={(e) => setReason(e.target.value)} />
              )}
            </FormField>

            {/* `figures` because a cancellation hands back the paper on the
                screen, so its own stored totals are the right two figures to
                show. The avoir dialog above passes none. */}
            <RefundChoice
              document={document}
              value={settlement}
              onChange={setSettlement}
              figures
            />

            {cancel.isError ? <Refusal error={cancel.error} /> : null}

            <DialogFooter>
              <Button type="button" variant="ghost" onClick={() => setOpen(false)}>
                {t("documents_cancel_action")}
              </Button>
              <Button type="submit" variant="destructive">
                {t("documents_cancel_confirm")}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </section>
  );
}

/** The sheet the core rendered, in a sandboxed frame. A ticket has its own
 *  80 mm paper; a facture, an avoir and a proforma are the same sheet with a
 *  different title, so they all go through the facture route. */
function PrintPanel({
  id,
  kind,
  paper,
  onPaper,
}: {
  id: number;
  kind: DocumentKindDto;
  paper: PrintPaper;
  onPaper: (paper: PrintPaper) => void;
}) {
  const { t, lang } = useTranslation();
  const sheet = kind !== "ticket";
  const page = useQuery({
    queryKey: sheet ? saleFactureQueryKey(id, lang, paper) : saleTicketQueryKey(id, lang),
    queryFn: () => (sheet ? api.getSaleFacture(id, lang, paper) : api.getSaleTicket(id, lang)),
  });
  return (
    <section
      aria-label={t("documents_print")}
      className="flex flex-col gap-3 rounded-lg border border-border p-3"
    >
      <div className="flex flex-wrap items-center gap-3">
        <h3 className="text-sm font-semibold text-foreground">{t("documents_print")}</h3>
        {sheet ? (
          <Tabs
            className="ms-auto"
            value={paper}
            onValueChange={(value) => {
              if (isPaper(value)) onPaper(value);
            }}
          >
            <TabsList aria-label={t("till_paper")}>
              <TabsTrigger value="a4">{t("till_paper_a4")}</TabsTrigger>
              <TabsTrigger value="a5">{t("till_paper_a5")}</TabsTrigger>
            </TabsList>
          </Tabs>
        ) : null}
      </div>
      {page.isPending ? <Skeleton className="h-96 w-full" /> : null}
      {page.isError ? <Refusal error={page.error} /> : null}
      {page.isSuccess ? (
        <iframe
          title={t("documents_print")}
          srcDoc={page.data}
          // An empty sandbox: the page carries no script and needs no
          // origin, so what it renders cannot reach this one even if a
          // product name ever slipped past the template's escaping.
          sandbox=""
          className="h-96 w-full rounded-md border border-border bg-card"
          data-testid="documents-sheet"
        />
      ) : null}
    </section>
  );
}

/** The tab list hands back a string; the two papers are a union. A guard
 *  rather than an assertion, the same way the kind filter narrows. */
function isPaper(value: string): value is PrintPaper {
  return value === "a4" || value === "a5";
}
