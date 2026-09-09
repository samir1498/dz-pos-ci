// The documents screen: every numbered paper the shop has issued, and what
// can still be done to one.
//
// The till is where a document is made; this is where it is found again. A
// row opens the document itself, with the lines it was issued with, the
// totals a comptable reads and the balance block when it carries one, and
// the sheet the core renders in the sandboxed panel beside them.
//
// Two things are done from here and nowhere else, because both are about a
// paper that already exists: writing an avoir against a facture, and
// annulling a document. Neither decides anything. What a line has left to
// credit, what a cancellation puts back and whether an avoir is issued with
// it are all the core's answers; this screen collects a quantity and a
// reason and shows what came back.

import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { QueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { ApiError, formatCentimes } from "@dzpos/shared";
import type {
  AvoirLineDto,
  DocumentKindDto,
  PrintPaper,
  SaleCancelEffectDto,
  SaleDto,
  SaleKindDto,
} from "@dzpos/shared";
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
  salesQueryKey,
  salesQueryPrefix,
} from "@/api";
import { useTranslation, type Key } from "@/i18n";

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  not_found: "error_not_found",
  money: "error_money",
  storage: "error_storage",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unreachable: "error_unreachable",
};

/** The server sends a code, never a sentence; the UI owns the wording. */
function errorKey(error: unknown): Key {
  if (error instanceof ApiError) {
    return ERROR_KEY[error.code] ?? "error_unknown";
  }
  return "error_unknown";
}

/** The filter the list offers. `all` is no filter at all rather than a
 *  fourth value on the wire: the route reads a missing `kind` as every kind
 *  (crates/api/src/routes/sales.rs). */
type Filter = SaleKindDto | "all";

/** Every value the filter offers, and what each is called. A record keyed by
 *  the union, so a sale kind added to the API fails to compile here rather
 *  than quietly never being offered: an array of strings would have taken the
 *  new kind's absence in silence.
 *
 *  `Object.keys` types its answer as `string[]`, which loses exactly the fact
 *  this needs, so the list is narrowed back by a guard rather than asserted.
 *  Insertion order is the order they are offered in. */
const FILTER_KEY = {
  all: "documents_filter_all",
  ticket: "documents_kind_ticket",
  facture: "documents_kind_facture",
  proforma: "documents_kind_proforma",
} satisfies Record<Filter, Key>;

function isFilter(value: string): value is Filter {
  return value in FILTER_KEY;
}

const FILTERS: readonly Filter[] = Object.keys(FILTER_KEY).filter(isFilter);

const KIND_KEY: Record<DocumentKindDto, Key> = {
  ticket: "documents_kind_ticket",
  facture: "documents_kind_facture",
  proforma: "documents_kind_proforma",
  avoir: "documents_kind_avoir",
  bon_de_livraison: "documents_kind_bon_de_livraison",
  bon_de_reception: "documents_kind_bon_de_reception",
  quittance: "documents_kind_quittance",
};

/** The day a document was issued, as the list column shows it. The API
 *  answers `YYYY-MM-DD HH:MM:SS` already on the shop's calendar, so the day
 *  is the first ten characters and never a `Date` this screen builds: a
 *  parse here would drag the browser's timezone into a figure the core
 *  already decided (features.md, the shop's clock). */
function day(issuedAt: string): string {
  return issuedAt.slice(0, 10);
}

export function DocumentsScreen() {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<Filter>("all");
  const [openId, setOpenId] = useState<number | null>(null);
  const kind = filter === "all" ? undefined : filter;
  const documents = useQuery({
    queryKey: salesQueryKey(kind),
    queryFn: () => api.listSales(kind),
  });

  return (
    <section className="flex flex-col gap-4">
      <h1 className="text-xl font-semibold">{t("documents_title")}</h1>

      <fieldset className="flex flex-wrap gap-3 border-0 p-0">
        <legend className="mb-1">{t("documents_filter")}</legend>
        {FILTERS.map((value) => (
          <label key={value} className="flex items-center gap-1">
            <input
              type="radio"
              name="documents-filter"
              checked={filter === value}
              onChange={() => {
                setFilter(value);
                // The open document may not be in the narrowed list any
                // more, and a detail panel showing a row the list no longer
                // has is a screen disagreeing with itself.
                setOpenId(null);
              }}
            />
            {t(FILTER_KEY[value])}
          </label>
        ))}
      </fieldset>

      {documents.isPending ? <p>{t("products_loading")}</p> : null}
      {documents.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(documents.error))}
        </p>
      ) : null}

      {documents.isSuccess ? (
        documents.data.length === 0 ? (
          <p>{t("documents_empty")}</p>
        ) : (
          <table className="w-full text-start">
            <thead>
              <tr>
                <th className="text-start">{t("documents_number")}</th>
                <th className="text-start">{t("documents_date")}</th>
                <th className="text-start">{t("documents_kind")}</th>
                <th className="text-start">{t("documents_customer")}</th>
                <th className="text-start">{t("documents_net")}</th>
                <th className="text-start">{t("documents_status")}</th>
              </tr>
            </thead>
            <tbody>
              {documents.data.map((d) => (
                <tr key={d.id}>
                  <td>
                    <button
                      type="button"
                      className="underline"
                      onClick={() => setOpenId(d.id === openId ? null : d.id)}
                    >
                      {d.printed_number}
                    </button>
                  </td>
                  <td>{day(d.issued_at)}</td>
                  <td>{t(KIND_KEY[d.kind])}</td>
                  {/* The buyer's name is on the document, snapshotted at
                      issue: a reprint has to show the block the customer was
                      handed, so the fiche is never read live for it. */}
                  <td>{d.buyer_name ?? ""}</td>
                  {/* dir="ltr" on the amount: an amount reads left to right
                      in Arabic too. */}
                  <td dir="ltr">{formatCentimes(d.totals.net_to_pay_centimes)}</td>
                  <td>
                    {t(d.status === "cancelled" ? "documents_cancelled" : "documents_issued")}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )
      ) : null}

      {openId === null ? null : <DocumentDetail id={openId} onClose={() => setOpenId(null)} />}
    </section>
  );
}

function DocumentDetail({ id, onClose }: { id: number; onClose: () => void }) {
  const { t } = useTranslation();
  const [paper, setPaper] = useState<PrintPaper>("a4");
  const document = useQuery({ queryKey: ["sale", id], queryFn: () => api.getSale(id) });

  if (document.isPending) return <p>{t("products_loading")}</p>;
  if (document.isError) {
    return (
      <p role="alert" className="text-red-700">
        {t(errorKey(document.error))}
      </p>
    );
  }
  const doc = document.data;
  return (
    <section aria-label={t("documents_detail")} className="flex flex-col gap-3 rounded border p-3">
      <header className="flex items-center justify-between gap-4">
        <strong>{doc.printed_number}</strong>
        <button type="button" className="underline" onClick={onClose}>
          {t("documents_close")}
        </button>
      </header>

      {doc.cancellation === null ? null : (
        <p role="status" className="text-red-700">
          {t("documents_cancelled_on")} {day(doc.cancellation.cancelled_at)} :{" "}
          {doc.cancellation.reason}
        </p>
      )}

      <table className="w-full text-start">
        <thead>
          <tr>
            <th className="text-start">{t("documents_line")}</th>
            <th className="text-start">{t("documents_qty")}</th>
            <th className="text-start">{t("documents_unit_price")}</th>
            <th className="text-start">{t("documents_line_total")}</th>
          </tr>
        </thead>
        <tbody>
          {doc.lines.map((l) => (
            <tr key={l.id}>
              <td>{l.name}</td>
              <td dir="ltr">{l.qty_milli / 1000}</td>
              <td dir="ltr">{formatCentimes(l.unit_price_centimes)}</td>
              <td dir="ltr">{formatCentimes(l.line_total_centimes)}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <dl className="grid grid-cols-2 gap-1">
        <dt>{t("documents_total_ht")}</dt>
        <dd dir="ltr">{formatCentimes(doc.totals.total_ht_centimes)}</dd>
        <dt>{t("documents_tva")}</dt>
        <dd dir="ltr">{formatCentimes(doc.totals.tva_centimes)}</dd>
        <dt>{t("documents_stamp")}</dt>
        <dd dir="ltr">{formatCentimes(doc.totals.stamp_centimes)}</dd>
        <dt>{t("documents_net")}</dt>
        <dd dir="ltr">{formatCentimes(doc.totals.net_to_pay_centimes)}</dd>
      </dl>

      {doc.balance === null ? null : (
        <dl className="grid grid-cols-2 gap-1" aria-label={t("documents_balance")}>
          <dt>{t("documents_old_balance")}</dt>
          <dd dir="ltr">{formatCentimes(doc.balance.old_balance_centimes)}</dd>
          <dt>{t("documents_remaining_debt")}</dt>
          <dd dir="ltr">{formatCentimes(doc.balance.remaining_debt_centimes)}</dd>
          <dt>{t("documents_total_debt")}</dt>
          <dd dir="ltr">{formatCentimes(doc.balance.total_debt_centimes)}</dd>
        </dl>
      )}

      {doc.kind === "facture" ? <AvoirPanel facture={doc} /> : null}
      {doc.status === "issued" && doc.kind !== "avoir" && doc.kind !== "proforma" ? (
        <CancelPanel document={doc} />
      ) : null}

      <PrintPanel id={doc.id} kind={doc.kind} paper={paper} onPaper={setPaper} />
    </section>
  );
}

/** The credit notes already written against a facture, and the form that
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
  const avoirs = useQuery({
    queryKey: saleAvoirsQueryKey(facture.id),
    queryFn: () => api.listAvoirs(facture.id),
  });

  const write = useMutation({
    mutationFn: (lines: AvoirLineDto[] | null) =>
      api.createAvoir(facture.id, {
        lines,
        reason: reason.trim() === "" ? null : reason.trim(),
      }),
    onSuccess: async () => {
      setOpen(false);
      setQty({});
      setReason("");
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
    <section aria-label={t("documents_avoirs")} className="flex flex-col gap-2 rounded border p-3">
      <strong>{t("documents_avoirs")}</strong>
      {avoirs.isSuccess && avoirs.data.length > 0 ? (
        <ul>
          {avoirs.data.map((a) => (
            <li key={a.id}>
              {a.printed_number}{" : "}<span dir="ltr">{formatCentimes(a.totals.net_to_pay_centimes)}</span>
            </li>
          ))}
        </ul>
      ) : null}

      {open ? (
        <form
          className="flex flex-col gap-2"
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
            <label key={l.id} className="flex items-center gap-2">
              <span className="grow">
                {l.name} ({t("documents_left")} <span dir="ltr">{left(l.id, l.qty_milli) / 1000}</span>)
              </span>
              <input
                type="number"
                min={0}
                max={left(l.id, l.qty_milli) / 1000}
                step="0.001"
                aria-label={`${t("documents_avoir_qty")} ${l.name}`}
                value={qty[l.id] ?? ""}
                onChange={(e) => setQty({ ...qty, [l.id]: e.target.value })}
                className="w-24 rounded border px-2 py-1"
              />
            </label>
          ))}
          <label className="flex flex-col gap-1">
            {t("documents_reason")}
            <input
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              className="rounded border px-2 py-1"
            />
          </label>
          <div className="flex gap-2">
            <button type="submit" className="rounded border px-3 py-1.5">
              {t("documents_avoir_write")}
            </button>
            {/* The whole of what is left, which is the button a shop reaches
                for when the customer brought everything back. `null` lines
                is what says so on the wire. */}
            <button
              type="button"
              className="rounded border px-3 py-1.5"
              onClick={() => write.mutate(null)}
            >
              {t("documents_avoir_whole")}
            </button>
            <button type="button" className="underline" onClick={() => setOpen(false)}>
              {t("documents_cancel_action")}
            </button>
          </div>
        </form>
      ) : (
        <button
          type="button"
          className="rounded border px-3 py-1.5 self-start"
          onClick={() => setOpen(true)}
        >
          {t("documents_avoir_new")}
        </button>
      )}

      {write.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(write.error))}
        </p>
      ) : null}
    </section>
  );
}

/** Annulling a document. The confirm says what will happen, and it says a
 *  different thing when the document put money on a customer's account: the
 *  goods come back either way, and a facture carrying debt is undone by an
 *  avoir the core writes with it (features.md §3). */
/** The sentence for one effect. The amount is put into the sentence rather
 *  than after it, because the three languages do not agree on where in the
 *  line a figure belongs: Arabic reads it mid-sentence. */
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
    ["sale", documentId],
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

function CancelPanel({ document }: { document: SaleDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const cancel = useMutation({
    mutationFn: () => api.cancelSale(document.id, { reason }),
    onSuccess: async () => {
      setOpen(false);
      setReason("");
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
    <section aria-label={t("documents_cancel")} className="flex flex-col gap-2 rounded border p-3">
      <strong>{t("documents_cancel")}</strong>
      {open ? (
        <form
          className="flex flex-col gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            cancel.mutate();
          }}
        >
          <p role="status">{effect === null ? t("products_loading") : says(t, effect)}</p>
          <label className="flex flex-col gap-1">
            {t("documents_reason")}
            <input
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              className="rounded border px-2 py-1"
            />
          </label>
          <div className="flex gap-2">
            <button type="submit" className="rounded border px-3 py-1.5">
              {t("documents_cancel_confirm")}
            </button>
            <button type="button" className="underline" onClick={() => setOpen(false)}>
              {t("documents_cancel_action")}
            </button>
          </div>
        </form>
      ) : (
        <button
          type="button"
          className="rounded border px-3 py-1.5 self-start"
          onClick={() => setOpen(true)}
        >
          {t("documents_cancel")}
        </button>
      )}
      {cancel.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(cancel.error))}
        </p>
      ) : null}
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
    <section aria-label={t("documents_print")} className="flex flex-col gap-2 rounded border p-3">
      <strong>{t("documents_print")}</strong>
      {sheet ? (
        <fieldset className="flex flex-wrap gap-3 border-0 p-0">
          <legend className="mb-1">{t("till_paper")}</legend>
          {(["a4", "a5"] as const).map((value) => (
            <label key={value} className="flex items-center gap-1">
              <input
                type="radio"
                name="documents-paper"
                checked={paper === value}
                onChange={() => onPaper(value)}
              />
              {t(value === "a4" ? "till_paper_a4" : "till_paper_a5")}
            </label>
          ))}
        </fieldset>
      ) : null}
      {page.isPending ? <p>{t("products_loading")}</p> : null}
      {page.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(page.error))}
        </p>
      ) : null}
      {page.isSuccess ? (
        <iframe
          title={t("documents_print")}
          srcDoc={page.data}
          // An empty sandbox: the page carries no script and needs no
          // origin, so what it renders cannot reach this one even if a
          // product name ever slipped past the template's escaping.
          sandbox=""
          className="h-96 w-full border-0"
          data-testid="documents-sheet"
        />
      ) : null}
    </section>
  );
}

export const Route = createFileRoute("/documents")({ component: DocumentsScreen });
