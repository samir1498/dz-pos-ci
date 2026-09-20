// Ringing a sale from the phone, and what happens when the Wi-Fi does not
// cooperate.
//
// Three things here are deliberate and were each paid for once:
//
//  - The idempotency key is minted before the first attempt, so the attempt
//    and every retry promise the *same* sale. The server answers a replay
//    with the original rather than ringing it twice.
//  - What the customer handed over is typed by the cashier and sent as
//    typed. The change is the server's answer, never this hook's
//    arithmetic (rule 2: a document's amounts come from the core).
//  - A refusal leaves the queue alone in the sense that it is *dropped*, not
//    kept: the server read the body and said no, so resending it forever
//    only fills the phone up.

import type { NewSaleDto, SaleDto } from "@dzpos/shared";
import { useCallback, useEffect, useState } from "react";

import { call } from "../../lib/api";
import { readChange, type CartLine } from "../../lib/basket";
import type { Say } from "../../lib/outcome";
import { enqueue, list, newIdempotencyKey, retry } from "../../lib/queue";
import type { Session } from "../../lib/session";

type Settled = { done: boolean; sale: SaleDto | null };

export type Ringing = {
  /** How many sales are waiting on disk for the Wi-Fi to come back. */
  queued: number;
  /** Waiting, but queued by somebody else — this signer cannot send them. */
  skipped: number;
  /** The last thing the server refused, in words a cashier can act on. */
  notice: Say | null;
  /** The change the server worked out for the sale just rung. */
  change: number | null;
  busy: boolean;
  ring: (tenderedCentimes: number) => Promise<void>;
  retryQueued: () => Promise<void>;
  dismiss: () => void;
};

export function useRing({
  session,
  lines,
  onRung,
  onSessionLost,
  onDeviceLost,
}: {
  session: Session;
  lines: readonly CartLine[];
  /** Clears the basket. Called for a sale that is done with, queued or not:
   * either way this customer has been served. */
  onRung: () => void;
  onSessionLost: (say: Say) => void;
  onDeviceLost: (say: Say) => void;
}): Ringing {
  const [queued, setQueued] = useState(0);
  const [skipped, setSkipped] = useState(0);
  const [notice, setNotice] = useState<Say | null>(null);
  const [change, setChange] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);

  const credentials = {
    deviceToken: session.deviceToken,
    sessionToken: session.sessionToken,
  };

  useEffect(() => {
    void list().then((q) => setQueued(q.length));
  }, []);

  /** Best-effort: the same bytes the desktop till prints, spooled beside the
   * shop file. The sale stands whether or not the paper does. */
  const print = useCallback(
    (saleId: number) => {
      void call(`/sales/${saleId}/print?lang=fr`, { method: "POST", credentials });
    },
    [session.deviceToken, session.sessionToken],
  );

  /** Sorts one answer and says whether this sale is done with. */
  const settle = useCallback(
    async (path: string, body: string): Promise<Settled> => {
      const { outcome, body: parsed } = await call<SaleDto>(path, {
        method: "POST",
        credentials,
        body,
      });
      switch (outcome.kind) {
        case "rang":
          return { done: true, sale: parsed };
        case "sign-in-again":
          onSessionLost(outcome.say);
          return { done: true, sale: null };
        case "pair-again":
          onDeviceLost(outcome.say);
          return { done: true, sale: null };
        case "refused":
          setNotice(outcome.say);
          return { done: true, sale: null };
        case "queue":
          return { done: false, sale: null };
      }
    },
    [session.deviceToken, session.sessionToken, onSessionLost, onDeviceLost],
  );

  const ring = useCallback(
    async (tenderedCentimes: number) => {
      if (lines.length === 0) return;
      setBusy(true);
      setNotice(null);
      // Everything the till does not decide is left out rather than sent as
      // a zero: `global_discount_centimes`, `customer_id`, `override` and
      // `kind` all carry a serde default in `crates/api/src/dto.rs`, and a
      // phone that spelled them out would be restating the core's defaults.
      const basket: Pick<NewSaleDto, "lines" | "payment_mode" | "tendered_centimes"> & {
        idempotency_key: string;
      } = {
        lines: lines.map((line) => ({
          product_id: line.product.id,
          qty_milli: line.qty * 1000,
          unit_price_centimes: null,
          line_discount_centimes: 0,
        })),
        payment_mode: "cash",
        tendered_centimes: tenderedCentimes,
        idempotency_key: newIdempotencyKey(),
      };
      const body = JSON.stringify(basket);

      const { done, sale } = await settle("/sales", body);
      if (sale !== null) {
        print(sale.id);
        setChange(readChange(sale.change_centimes));
        onRung();
        setBusy(false);
        return;
      }
      if (done) {
        setBusy(false);
        return;
      }

      // Only here: no answer at all, or the server could not give one. That
      // is the one thing the queue exists for.
      await enqueue({
        method: "POST",
        url: "/sales",
        sessionToken: session.sessionToken,
        body,
      });
      onRung();
      setQueued((await list()).length);
      setBusy(false);
    },
    [lines, settle, print, onRung, session.sessionToken],
  );

  const retryQueued = useCallback(async () => {
    setBusy(true);
    setNotice(null);
    const outcome = await retry(async (req) => {
      // Queued items are stored as paths so a shop that changes the till
      // computer's address can still send yesterday's sale; anything from
      // before that has an origin baked in, and it is this phone's own
      // base that should win either way.
      const path = req.url.replace(/^https?:\/\/[^/]+/, "");
      const { done, sale } = await settle(path, req.body ?? "");
      if (sale !== null) print(sale.id);
      return done;
    }, session.sessionToken);
    setQueued((await list()).length);
    setSkipped(outcome.skipped);
    setBusy(false);
  }, [settle, print, session.sessionToken]);

  const dismiss = useCallback(() => {
    setNotice(null);
    setChange(null);
  }, []);

  return { queued, skipped, notice, change, busy, ring, retryQueued, dismiss };
}
