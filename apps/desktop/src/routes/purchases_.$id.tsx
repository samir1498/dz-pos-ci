// One order: what was ordered, what has arrived, what went back, and the bons
// de réception each delivery was written on.
//
// Everything that changes an order is here, because everything that changes
// an order changes what the shop owes: a delivery raises the debt at the cost
// the goods landed at, a return lowers it, and the two ways of closing an
// order each ask for a reason the server writes into the audit log.
//
// The figures are the server's. The lines carry the running totals the file
// holds, so a screen adding the receipts up itself would be a second answer.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { formatCentimes, formatQty, parseQtyToMilli } from "@dzpos/shared";
import type { NewReceiptDto, PurchaseDetailDto, PurchaseLineDto } from "@dzpos/shared";

import {
  api,
  productsQueryKey,
  purchaseQueryKey,
  purchasesQueryKey,
  suppliersQueryKey,
} from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";
import { PURCHASE_STATUS_KEY } from "./purchases";

export const Route = createFileRoute("/purchases_/$id")({ component: OnePurchaseRoute });

function OnePurchaseRoute() {
  const { id } = Route.useParams();
  // A path is text, and `/purchases/abc` is a link somebody mistyped rather
  // than an order this shop does not have.
  const parsed = Number(id);
  if (!Number.isInteger(parsed)) return <NotAPurchase />;
  return <OnePurchase id={parsed} />;
}

function NotAPurchase() {
  const { t } = useTranslation();
  return (
    <section className="flex flex-col gap-4">
      <p role="alert" className="text-red-700">
        {t("error_not_found")}
      </p>
      <Link to="/purchases" className="underline">
        {t("action_back_to_purchases")}
      </Link>
    </section>
  );
}

export function OnePurchase({ id }: { id: number }) {
  const { t } = useTranslation();
  const order = useQuery({
    queryKey: purchaseQueryKey(id),
    queryFn: () => api.getPurchase(id),
  });
  const products = useQuery({ queryKey: productsQueryKey, queryFn: () => api.listProducts() });

  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("purchases_one")}</h1>
        <Link to="/purchases" className="underline">
          {t("action_back_to_purchases")}
        </Link>
      </header>

      {order.isPending ? <p>{t("purchases_loading")}</p> : null}
      {order.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(order.error))}
        </p>
      ) : null}
      {order.isSuccess ? (
        <PurchaseDetail
          detail={order.data}
          nameOf={(productId) =>
            products.data?.find((p) => p.id === productId)?.name ?? String(productId)
          }
        />
      ) : null}
    </section>
  );
}

function PurchaseDetail({
  detail,
  nameOf,
}: {
  detail: PurchaseDetailDto;
  nameOf: (productId: number) => string;
}) {
  const { t } = useTranslation();
  const { purchase, lines } = detail;
  const open = purchase.status === "ordered" || purchase.status === "partially_received";
  const anythingArrived = lines.some((l) => l.qty_received_milli > 0);

  return (
    <div className="flex flex-col gap-6">
      <dl className="flex flex-wrap gap-x-8 gap-y-2">
        <div>
          <dt className="text-sm opacity-70">{t("col_date")}</dt>
          <dd className="font-mono" dir="ltr">
            {purchase.purchase_date}
          </dd>
        </div>
        <div>
          <dt className="text-sm opacity-70">{t("col_supplier")}</dt>
          <dd>
            <Link
              to="/suppliers/$id"
              params={{ id: String(purchase.supplier_id) }}
              className="underline"
            >
              {t("purchases_supplier_fiche")}
            </Link>
          </dd>
        </div>
        <div>
          <dt className="text-sm opacity-70">{t("col_status")}</dt>
          {/* A test id, because the state's own word and a column header of
              the lines table read the same in English ("Received"). */}
          <dd data-testid="purchase-status">{t(PURCHASE_STATUS_KEY[purchase.status])}</dd>
        </div>
        <div>
          <dt className="text-sm opacity-70">{t("col_extra_costs")}</dt>
          <dd className="font-mono" dir="ltr">
            {formatCentimes(purchase.transport_centimes + purchase.extra_costs_centimes)}
          </dd>
        </div>
      </dl>

      <LinesTable lines={lines} nameOf={nameOf} />

      <ReceiptsList detail={detail} nameOf={nameOf} />

      {open ? (
        <MovementForm
          detail={detail}
          nameOf={nameOf}
          kind="receive"
          title="purchases_receive"
          hint="purchases_receive_hint"
          action="action_receive"
          outstanding={(line) => line.qty_ordered_milli - line.qty_received_milli}
        />
      ) : null}

      {anythingArrived ? (
        <MovementForm
          detail={detail}
          nameOf={nameOf}
          kind="return"
          title="purchases_return"
          hint="purchases_return_hint"
          action="action_return"
          outstanding={(line) => line.qty_received_milli - line.qty_returned_milli}
        />
      ) : null}

      {purchase.status === "ordered" ? (
        <ReasonForm
          detail={detail}
          kind="cancel"
          title="purchases_cancel"
          hint="purchases_cancel_hint"
          action="action_cancel_order"
        />
      ) : null}
      {purchase.status === "partially_received" ? (
        <ReasonForm
          detail={detail}
          kind="close_short"
          title="purchases_close_short"
          hint="purchases_close_short_hint"
          action="action_close_short"
        />
      ) : null}
    </div>
  );
}

function LinesTable({
  lines,
  nameOf,
}: {
  lines: PurchaseLineDto[];
  nameOf: (productId: number) => string;
}) {
  const { t } = useTranslation();
  return (
    <table className="w-full text-start">
      <caption className="sr-only">{t("purchases_lines")}</caption>
      <thead>
        <tr>
          <th scope="col" className="text-start pb-2 pe-3">{t("col_product")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_ordered")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_received")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_returned")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_unit_cost")}</th>
          <th scope="col" className="text-end pb-2 ps-3">{t("col_landed_cost")}</th>
        </tr>
      </thead>
      <tbody>
        {lines.map((line) => (
          <tr key={line.id} className="border-t">
            <td className="py-1.5 pe-3">{nameOf(line.product_id)}</td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">{formatQty(line.qty_ordered_milli)}</span>
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">{formatQty(line.qty_received_milli)}</span>
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">{formatQty(line.qty_returned_milli)}</span>
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">{formatCentimes(line.unit_cost_centimes)}</span>
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">
              <span dir="ltr">{formatCentimes(line.landed_unit_cost_centimes)}</span>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** The deliveries, newest first, each with its own number. A bon de
 *  réception is kept and listed like the paper it stands for; M3 does not
 *  print one. */
function ReceiptsList({
  detail,
  nameOf,
}: {
  detail: PurchaseDetailDto;
  nameOf: (productId: number) => string;
}) {
  const { t } = useTranslation();
  if (detail.receipts.length === 0) return <p>{t("purchases_no_receipt")}</p>;
  return (
    <section className="flex flex-col gap-2">
      <h2 className="font-semibold">{t("purchases_receipts")}</h2>
      <ul className="flex flex-col gap-2">
        {detail.receipts.map((receipt) => (
          <li key={receipt.id} className="rounded border p-2">
            <p className="font-mono" dir="ltr">
              {receipt.series} / {receipt.number} · {receipt.received_at}
            </p>
            <ul>
              {receipt.lines.map((line) => {
                const ordered = detail.lines.find((l) => l.id === line.purchase_line_id);
                return (
                  <li key={line.purchase_line_id}>
                    {ordered === undefined ? "" : nameOf(ordered.product_id)}{" "}
                    <span className="font-mono" dir="ltr">
                      {formatQty(line.qty_milli)}
                    </span>
                  </li>
                );
              })}
            </ul>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** A delivery and a return are the same form: the lines that still have
 *  something to move, and how much of each. The direction is the route the
 *  button posts to, and what "outstanding" means is the only thing that
 *  differs between them. */
function MovementForm({
  detail,
  nameOf,
  kind,
  title,
  hint,
  action,
  outstanding,
}: {
  detail: PurchaseDetailDto;
  nameOf: (productId: number) => string;
  kind: "receive" | "return";
  title: Key;
  hint: Key;
  action: Key;
  outstanding: (line: PurchaseLineDto) => number;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [typed, setTyped] = useState<Record<number, string>>({});
  const [note, setNote] = useState("");
  const [problem, setProblem] = useState<Key | null>(null);
  const id = detail.purchase.id;

  const send = useMutation({
    mutationFn: (body: NewReceiptDto) =>
      kind === "receive" ? api.receivePurchase(id, body) : api.returnPurchase(id, body),
    onSuccess: async () => {
      setProblem(null);
      setTyped({});
      setNote("");
      // The stock and what the shop owes both moved, so the product list and
      // the supplier list are stale along with this order.
      await queryClient.invalidateQueries({ queryKey: purchaseQueryKey(id) });
      await queryClient.invalidateQueries({ queryKey: purchasesQueryKey });
      await queryClient.invalidateQueries({ queryKey: productsQueryKey });
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
    },
    onError: (error: unknown) => setProblem(errorKey(error)),
  });

  const movable = detail.lines.filter((line) => outstanding(line) > 0);
  if (movable.length === 0) return null;

  const submit = async () => {
    const built: NewReceiptDto["lines"] = [];
    for (const line of movable) {
      const text = typed[line.id] ?? "";
      if (text.trim() === "") continue;
      const qty = parseQtyToMilli(text);
      if (qty === null || qty <= 0) {
        setProblem("error_amount_unreadable");
        return;
      }
      built.push({ purchase_line_id: line.id, qty_milli: qty });
    }
    if (built.length === 0) {
      setProblem("purchases_no_line");
      return;
    }
    await send.mutateAsync({ lines: built, note: note.trim() === "" ? null : note }).catch(
      () => undefined,
    );
  };

  return (
    <form
      className="flex flex-col gap-2 rounded border p-3"
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      <h2 className="font-semibold">{t(title)}</h2>
      <p className="text-sm opacity-70">{t(hint)}</p>
      {movable.map((line) => (
        <label key={line.id} className="flex items-center gap-2">
          {/* The product and not the word "quantity": an order with two lines
              would otherwise offer two identical boxes. */}
          <span className="min-w-40">{nameOf(line.product_id)}</span>
          <input
            dir="ltr"
            inputMode="decimal"
            className="w-24 rounded border px-2 py-1 font-mono text-end"
            aria-label={`${t(action)} ${String(line.id)}`}
            value={typed[line.id] ?? ""}
            onChange={(e) =>
              setTyped((current) => ({ ...current, [line.id]: e.target.value }))
            }
          />
          <span className="text-sm opacity-70 font-mono" dir="ltr">
            / {formatQty(outstanding(line))}
          </span>
        </label>
      ))}
      <label className="flex flex-col gap-1">
        <span>{t("col_note")}</span>
        <input
          className="rounded border px-2 py-1"
          value={note}
          onChange={(e) => setNote(e.target.value)}
        />
      </label>
      {problem === null ? null : (
        <p role="alert" className="text-red-700">
          {t(problem)}
        </p>
      )}
      <div>
        <button type="submit" className="rounded border px-3 py-1.5" disabled={send.isPending}>
          {t(action)}
        </button>
      </div>
    </form>
  );
}

/** Cancelling and closing short are the same form: a reason, and the server
 *  refuses one that is blank. Both are decisions, and the reason is what the
 *  audit log carries. */
function ReasonForm({
  detail,
  kind,
  title,
  hint,
  action,
}: {
  detail: PurchaseDetailDto;
  kind: "cancel" | "close_short";
  title: Key;
  hint: Key;
  action: Key;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [reason, setReason] = useState("");
  const [problem, setProblem] = useState<Key | null>(null);
  const id = detail.purchase.id;

  const send = useMutation({
    mutationFn: (text: string) =>
      kind === "cancel"
        ? api.cancelPurchase(id, { reason: text })
        : api.closeShortPurchase(id, { reason: text }),
    onSuccess: async () => {
      setProblem(null);
      setReason("");
      await queryClient.invalidateQueries({ queryKey: purchaseQueryKey(id) });
      await queryClient.invalidateQueries({ queryKey: purchasesQueryKey });
    },
    onError: (error: unknown) => setProblem(errorKey(error)),
  });

  return (
    <form
      className="flex flex-col gap-2 rounded border p-3"
      onSubmit={(e) => {
        e.preventDefault();
        if (reason.trim() === "") {
          setProblem("purchases_reason_needed");
          return;
        }
        void send.mutateAsync(reason).catch(() => undefined);
      }}
    >
      <h2 className="font-semibold">{t(title)}</h2>
      <p className="text-sm opacity-70">{t(hint)}</p>
      <label className="flex flex-col gap-1">
        <span>{t("purchases_reason")}</span>
        <input
          className="rounded border px-2 py-1"
          value={reason}
          onChange={(e) => setReason(e.target.value)}
        />
      </label>
      {problem === null ? null : (
        <p role="alert" className="text-red-700">
          {t(problem)}
        </p>
      )}
      <div>
        <button type="submit" className="rounded border px-3 py-1.5" disabled={send.isPending}>
          {t(action)}
        </button>
      </div>
    </form>
  );
}
