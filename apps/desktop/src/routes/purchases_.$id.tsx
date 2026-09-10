// One order: what was ordered, what has arrived, what went back, and the bons
// de réception each delivery was written on.
//
// Everything that changes an order is here, because everything that changes
// an order changes what the shop owes: a delivery raises the debt at the cost
// the goods landed at, a return lowers it, and the two ways of closing an
// order each ask for a reason the server writes into the audit log.
//
// The four of them are dialogs rather than four forms stacked under the
// lines. Each is a decision taken once about the whole order, each asks for
// something (quantities, a reason), and the page underneath is what the
// person is deciding from; a dialog keeps the figures in view behind it and
// keeps the page itself readable, which four open forms did not.
//
// The figures are the server's. The lines carry the running totals the file
// holds, so a screen adding the receipts up itself would be a second answer.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowLeft, Inbox } from "lucide-react";
import { useState, type ReactNode } from "react";
import { formatQty, parseQtyToMilli } from "@dzpos/shared";
import type { NewReceiptDto, PurchaseDetailDto, PurchaseLineDto } from "@dzpos/shared";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
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
import {
  api,
  productsQueryKey,
  purchaseQueryKey,
  purchasesQueryKey,
  suppliersQueryKey,
} from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";
import { PurchaseStatusBadge } from "./purchases";

export const Route = createFileRoute("/purchases_/$id")({ component: OnePurchaseRoute });

function OnePurchaseRoute() {
  const { id } = Route.useParams();
  // A path is text, and `/purchases/abc` is a link somebody mistyped rather
  // than an order this shop does not have.
  const parsed = Number(id);
  if (!Number.isInteger(parsed)) return <NotAPurchase />;
  return <OnePurchase id={parsed} />;
}

function BackToList() {
  const { t } = useTranslation();
  return (
    <Button asChild variant="outline">
      <Link to="/purchases">
        <Icon as={ArrowLeft} size={18} flip />
        {t("action_back_to_purchases")}
      </Link>
    </Button>
  );
}

function NotAPurchase() {
  const { t } = useTranslation();
  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t("purchases_one")} actions={<BackToList />} />
      <p role="alert" className="text-sm text-fg-danger">
        {t("error_not_found")}
      </p>
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
      {order.isSuccess ? (
        <PurchaseDetail
          detail={order.data}
          nameOf={(productId) =>
            products.data?.find((p) => p.id === productId)?.name ?? String(productId)
          }
        />
      ) : (
        <PageHeader title={t("purchases_one")} actions={<BackToList />} />
      )}

      {order.isPending ? (
        <div className="flex flex-col gap-2">
          <span className="sr-only">{t("purchases_loading")}</span>
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : null}
      {order.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(order.error))}
        </p>
      ) : null}
    </section>
  );
}

/** One fact of the order's head: what it is called and what it says. */
function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd>{children}</dd>
    </div>
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
    <>
      <PageHeader
        title={t("purchases_one")}
        actions={
          <>
            {open ? (
              <MovementDialog
                detail={detail}
                nameOf={nameOf}
                kind="receive"
                trigger="purchases_receive"
                hint="purchases_receive_hint"
                action="action_receive"
                outstanding={(line) => line.qty_ordered_milli - line.qty_received_milli}
              />
            ) : null}
            {anythingArrived ? (
              <MovementDialog
                detail={detail}
                nameOf={nameOf}
                kind="return"
                trigger="purchases_return"
                hint="purchases_return_hint"
                action="action_return"
                outstanding={(line) => line.qty_received_milli - line.qty_returned_milli}
              />
            ) : null}
            {purchase.status === "ordered" ? (
              <ReasonDialog
                detail={detail}
                kind="cancel"
                title="purchases_cancel"
                hint="purchases_cancel_hint"
                action="action_cancel_order"
              />
            ) : null}
            {purchase.status === "partially_received" ? (
              <ReasonDialog
                detail={detail}
                kind="close_short"
                title="purchases_close_short"
                hint="purchases_close_short_hint"
                action="action_close_short"
              />
            ) : null}
            <BackToList />
          </>
        }
      />

      <div className="flex flex-col gap-6">
        <Card>
          <CardContent>
            <dl className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
              <Fact label={t("col_date")}>
                <span dir="ltr" className="font-numeric tabular-nums">
                  {purchase.purchase_date}
                </span>
              </Fact>
              <Fact label={t("col_supplier")}>
                <Button asChild variant="link" size="sm" className="h-auto p-0">
                  <Link to="/suppliers/$id" params={{ id: String(purchase.supplier_id) }}>
                    {t("purchases_supplier_fiche")}
                  </Link>
                </Button>
              </Fact>
              {/* A test id, because the state's own word and a column header of
                  the lines table read the same in English ("Received"). */}
              <Fact label={t("col_status")}>
                <PurchaseStatusBadge status={purchase.status} data-testid="purchase-status" />
              </Fact>
              <Fact label={t("col_extra_costs")}>
                <Money centimes={purchase.transport_centimes + purchase.extra_costs_centimes} />
              </Fact>
            </dl>
          </CardContent>
        </Card>

        <LinesTable lines={lines} nameOf={nameOf} />

        <ReceiptsList detail={detail} nameOf={nameOf} />
      </div>
    </>
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
  const columns: readonly Column<PurchaseLineDto>[] = [
    { id: "product", header: t("col_product"), cell: (line) => nameOf(line.product_id) },
    {
      id: "ordered",
      header: t("col_ordered"),
      numeric: true,
      cell: (line) => <Qty milli={line.qty_ordered_milli} />,
    },
    {
      id: "received",
      header: t("col_received"),
      numeric: true,
      cell: (line) => <Qty milli={line.qty_received_milli} />,
    },
    {
      id: "returned",
      header: t("col_returned"),
      numeric: true,
      cell: (line) => <Qty milli={line.qty_returned_milli} />,
    },
    // TODO(M4): a cashier does not see these two. What the shop pays for its
    // stock is not something a till operator has any call to read, and there
    // are no roles in the app until §5 lands.
    {
      id: "cost",
      header: t("col_unit_cost"),
      money: true,
      cell: (line) => <Money centimes={line.unit_cost_centimes} />,
    },
    {
      id: "landed",
      header: t("col_landed_cost"),
      money: true,
      cell: (line) => <Money centimes={line.landed_unit_cost_centimes} />,
    },
  ];
  return (
    <DataTable
      columns={columns}
      rows={lines}
      rowKey={(line) => line.id}
      caption={t("purchases_lines")}
    />
  );
}

/** A quantity in thousandths, read out the way an amount is: left to right,
 *  Western digits, on the figure face so a column lines up on the digit. */
function Qty({ milli }: { milli: number }) {
  return (
    <span dir="ltr" className="font-numeric tabular-nums">
      {formatQty(milli)}
    </span>
  );
}

/** The deliveries, newest first, each with its own number. A bon de
 *  réception is kept and listed like the paper it stands for; it is not
 *  printed. */
function ReceiptsList({
  detail,
  nameOf,
}: {
  detail: PurchaseDetailDto;
  nameOf: (productId: number) => string;
}) {
  const { t } = useTranslation();
  if (detail.receipts.length === 0) {
    return (
      <EmptyState
        icon={Inbox}
        title={t("purchases_no_receipt")}
        description={t("purchases_receive_hint")}
      />
    );
  }
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-md font-semibold">{t("purchases_receipts")}</h3>
      <ul className="flex flex-col gap-2">
        {detail.receipts.map((receipt) => (
          <li key={receipt.id}>
            <Card>
              <CardContent className="flex flex-col gap-1">
                <p dir="ltr" className="font-numeric tabular-nums text-sm text-muted-foreground">
                  {receipt.series} / {receipt.number} · {receipt.received_at}
                </p>
                <ul className="flex flex-col gap-1">
                  {receipt.lines.map((line) => {
                    const ordered = detail.lines.find((l) => l.id === line.purchase_line_id);
                    return (
                      <li key={line.purchase_line_id} className="flex items-center gap-2">
                        <span>{ordered === undefined ? "" : nameOf(ordered.product_id)}</span>
                        <Qty milli={line.qty_milli} />
                      </li>
                    );
                  })}
                </ul>
              </CardContent>
            </Card>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** A delivery and a return are the same dialog: the lines that still have
 *  something to move, and how much of each. The direction is the route the
 *  button posts to, and what "outstanding" means is the only thing that
 *  differs between them. */
function MovementDialog({
  detail,
  nameOf,
  kind,
  trigger,
  hint,
  action,
  outstanding,
}: {
  detail: PurchaseDetailDto;
  nameOf: (productId: number) => string;
  kind: "receive" | "return";
  trigger: Key;
  hint: Key;
  action: Key;
  outstanding: (line: PurchaseLineDto) => number;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [shown, setShown] = useState(false);
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
      setShown(false);
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
    <Dialog
      open={shown}
      onOpenChange={(next) => {
        setShown(next);
        // A dialog closed with Escape or the cross is a decision not taken:
        // what was typed and the refusal on screen both go with it.
        if (!next) {
          setTyped({});
          setNote("");
          setProblem(null);
        }
      }}
    >
      <DialogTrigger asChild>
        <Button variant={kind === "receive" ? "default" : "outline"}>{t(trigger)}</Button>
      </DialogTrigger>
      <DialogContent>
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            void submit();
          }}
        >
          <DialogHeader>
            <DialogTitle>{t(trigger)}</DialogTitle>
            <DialogDescription>{t(hint)}</DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-3">
            {movable.map((line) => (
              // The product and not the word "quantity": an order with two
              // lines would otherwise offer two identical boxes.
              <div key={line.id} className="flex items-center gap-2">
                <span className="min-w-40">{nameOf(line.product_id)}</span>
                <Input
                  dir="ltr"
                  inputMode="decimal"
                  autoComplete="off"
                  className="w-24 text-end font-numeric tabular-nums"
                  aria-label={`${t(action)} ${String(line.id)}`}
                  value={typed[line.id] ?? ""}
                  onChange={(e) =>
                    setTyped((current) => ({ ...current, [line.id]: e.target.value }))
                  }
                />
                <span dir="ltr" className="font-numeric tabular-nums text-sm text-muted-foreground">
                  / {formatQty(outstanding(line))}
                </span>
              </div>
            ))}
            <FormField label={t("col_note")}>
              {(parts) => (
                <Input {...parts} value={note} onChange={(e) => setNote(e.target.value)} />
              )}
            </FormField>
          </div>
          {problem === null ? null : (
            <p role="alert" className="text-sm text-fg-danger">
              {t(problem)}
            </p>
          )}
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => setShown(false)}>
              {t("action_cancel")}
            </Button>
            <Button type="submit" disabled={send.isPending}>
              {t(action)}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

/** Cancelling and closing short are the same dialog: a reason, and the server
 *  refuses one that is blank. Both are decisions, and the reason is what the
 *  audit log carries. */
function ReasonDialog({
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
  const [shown, setShown] = useState(false);
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
      setShown(false);
      await queryClient.invalidateQueries({ queryKey: purchaseQueryKey(id) });
      await queryClient.invalidateQueries({ queryKey: purchasesQueryKey });
    },
    onError: (error: unknown) => setProblem(errorKey(error)),
  });

  return (
    <Dialog
      open={shown}
      onOpenChange={(next) => {
        setShown(next);
        if (!next) {
          setReason("");
          setProblem(null);
        }
      }}
    >
      <DialogTrigger asChild>
        <Button variant="outline">{t(title)}</Button>
      </DialogTrigger>
      <DialogContent>
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault();
            if (reason.trim() === "") {
              setProblem("purchases_reason_needed");
              return;
            }
            void send.mutateAsync(reason).catch(() => undefined);
          }}
        >
          <DialogHeader>
            <DialogTitle>{t(title)}</DialogTitle>
            <DialogDescription>{t(hint)}</DialogDescription>
          </DialogHeader>
          <FormField label={t("purchases_reason")} required>
            {(parts) => (
              <Input {...parts} value={reason} onChange={(e) => setReason(e.target.value)} />
            )}
          </FormField>
          {problem === null ? null : (
            <p role="alert" className="text-sm text-fg-danger">
              {t(problem)}
            </p>
          )}
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => setShown(false)}>
              {t("action_cancel")}
            </Button>
            <Button type="submit" variant="destructive" disabled={send.isPending}>
              {t(action)}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
