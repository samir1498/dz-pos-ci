import { useEffect, useState } from "react";
import { Button, FlatList, Text, TextInput, View } from "react-native";

import type { RegimeDto } from "@dzpos/shared";

import {
  formatCentimes,
  priceBasket,
  readTendered,
  type CartLine,
  type Product,
} from "../lib/basket";
import { outcomeOf, type ApiError } from "../lib/outcome";
import { enqueue, list, newIdempotencyKey, retry } from "../lib/queue";
import type { Session } from "../lib/session";

/** The thin till (M6 T5, sessions M7 T3). Every call carries three
 * credentials, each proving one thing: the launch token (build-time env,
 * the server operator's secret, same one the desktop's own window shows)
 * says the caller may reach the server; the device token says which paired
 * phone; the session token says which signed-in person. A 401 names which
 * of the three failed, in the server's error code.
 *
 * What the shop is owed is priced through `@dzpos/shared` exactly as the
 * desktop till prices it (see `../lib/basket`), and what the customer hands
 * over is typed by the cashier. The screen posts the two separately; the
 * change is the server's answer, never this screen's arithmetic. */
export function Till({
  apiBase,
  launch,
  session,
  onSignOut,
  onSessionLost,
  onDeviceLost,
}: {
  apiBase: string;
  launch: string;
  session: Session;
  onSignOut: () => void;
  onSessionLost: (message: string) => void;
  onDeviceLost: (message: string) => void;
}) {
  const [products, setProducts] = useState<Product[]>([]);
  const [cart, setCart] = useState<CartLine[]>([]);
  const [queued, setQueued] = useState(0);
  const [skipped, setSkipped] = useState(0);
  const [regime, setRegime] = useState<RegimeDto | null>(null);
  const [tendered, setTendered] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const [change, setChange] = useState<number | null>(null);

  const headers = (): Record<string, string> => ({
    "content-type": "application/json",
    Authorization: `Bearer ${launch}`,
    "x-dzpos-device": session.deviceToken,
    "x-dzpos-session": session.sessionToken,
  });

  useEffect(() => {
    fetch(`${apiBase}/products`, { headers: headers() })
      .then((r) => r.json())
      .then((data) => setProducts(Array.isArray(data) ? data : []))
      .catch(() => setProducts([]));
    // The régime decides whether a price is HT or the whole price, so the
    // basket cannot be priced without it. Until it arrives the screen shows
    // no total and refuses to pay, rather than guessing `reel` and asking
    // the customer for a TVA an IFU shop does not charge.
    fetch(`${apiBase}/settings`, { headers: headers() })
      .then((r) => r.json())
      .then((data) => setRegime(data?.regime?.regime ?? null))
      .catch(() => setRegime(null));
    list().then((q) => setQueued(q.length));
  }, [apiBase]);

  const add = (p: Product) => {
    setChange(null);
    setCart((c) => {
      const found = c.find((x) => x.product.id === p.id);
      if (found) return c.map((x) => (x.product.id === p.id ? { ...x, qty: x.qty + 1 } : x));
      return [...c, { product: p, qty: 1 }];
    });
  };

  // What the shop is owed, priced the way the core prices it. A basket the
  // money rules cannot express (an amount past what a Number holds exactly)
  // leaves this null, and the screen says so instead of posting a total.
  let owed: number | null = null;
  if (regime !== null && cart.length > 0) {
    try {
      owed = priceBasket(cart, regime).netToPay;
    } catch {
      owed = null;
    }
  }

  const handed = readTendered(tendered);
  const short = owed !== null && handed !== null && handed < owed;
  const canPay = owed !== null && handed !== null && !short;

  /** Sorts one answer and tells the caller whether the sale is done with. */
  const settle = async (
    res: Response | null,
  ): Promise<{ done: boolean; sale?: { id?: number; change_centimes?: number } }> => {
    if (res === null) return { done: false };
    let body: unknown = null;
    try {
      body = await res.json();
    } catch {
      body = null;
    }
    const payload = body as { error?: ApiError; id?: number; change_centimes?: number } | null;
    const outcome = outcomeOf(res.status, payload?.error ?? null);
    switch (outcome.kind) {
      case "rang":
        return { done: true, sale: payload ?? undefined };
      case "sign-in-again":
        onSessionLost(outcome.message);
        return { done: true };
      case "pair-again":
        onDeviceLost(outcome.message);
        return { done: true };
      case "refused":
        setNotice(outcome.message);
        return { done: true };
      case "queue":
        return { done: false };
    }
  };

  const pay = async () => {
    if (cart.length === 0 || handed === null || owed === null || short) return;
    setNotice(null);
    // The key is minted before the first try so the try and every retry
    // promise the same sale; the server answers the original on a replay.
    const body = JSON.stringify({
      lines: cart.map((c) => ({ product_id: c.product.id, qty_milli: c.qty * 1000 })),
      payment_mode: "cash",
      tendered_centimes: handed,
      idempotency_key: newIdempotencyKey(),
    });
    let res: Response | null = null;
    try {
      res = await fetch(`${apiBase}/sales`, { method: "POST", headers: headers(), body });
    } catch {
      res = null;
    }
    const { done, sale } = await settle(res);
    if (sale?.id !== undefined) {
      // Print through desktop (M6 T6): same bytes the desktop's own till prints,
      // spooled beside the shop file and optionally pushed to TCP 9100.
      fetch(`${apiBase}/sales/${sale.id}/print?lang=fr`, {
        method: "POST",
        headers: headers(),
      }).catch(() => {});
      setChange(sale.change_centimes ?? null);
      setCart([]);
      setTendered("");
      return;
    }
    if (done) return;
    // Only here: the call never got an answer, or the server could not give
    // one. That is what the queue is for.
    await enqueue({
      method: "POST",
      url: `${apiBase}/sales`,
      sessionToken: session.sessionToken,
      body,
    });
    setCart([]);
    setTendered("");
    const q = await list();
    setQueued(q.length);
  };

  const retryAll = async () => {
    setNotice(null);
    const outcome = await retry(
      async (req) => {
        let res: Response | null = null;
        try {
          res = await fetch(req.url, { method: req.method, headers: headers(), body: req.body });
        } catch {
          return false;
        }
        const { done, sale } = await settle(res);
        if (sale?.id !== undefined) {
          // The ring went through on retry: print it like a first try
          // would have. Best-effort — the sale stands either way.
          fetch(`${apiBase}/sales/${sale.id}/print?lang=fr`, {
            method: "POST",
            headers: headers(),
          }).catch(() => {});
        }
        // A refusal is done with: dropping it from the queue stops the
        // phone resending a body the server will never accept.
        return done;
      },
      session.sessionToken,
    );
    const q = await list();
    setQueued(q.length);
    setSkipped(outcome.skipped);
  };

  return (
    <View style={{ flex: 1, padding: 16 }}>
      <Text style={{ fontWeight: "600" }}>Till — thin client (M6 T5)</Text>
      <View style={{ flexDirection: "row", justifyContent: "space-between", alignItems: "center" }}>
        <Text>
          {session.name} · {session.role}
        </Text>
        <Button title="Sign out" onPress={onSignOut} />
      </View>
      <Text>Queued: {queued}</Text>
      {skipped > 0 && <Text>{skipped} waiting — queued under another sign-in</Text>}
      {queued > 0 && <Button title="Retry queued" onPress={retryAll} />}
      {notice !== null && <Text>{notice}</Text>}
      {change !== null && <Text>Change: {formatCentimes(change)}</Text>}
      <FlatList
        data={products}
        keyExtractor={(p) => String(p.id)}
        renderItem={({ item }) => (
          <View style={{ flexDirection: "row", justifyContent: "space-between", paddingVertical: 8 }}>
            <Text>{item.name}</Text>
            <Button title="Add" onPress={() => add(item)} />
          </View>
        )}
      />
      <View style={{ borderTopWidth: 1, paddingTop: 12 }}>
        <Text>Cart: {cart.map((c) => `${c.product.name}×${c.qty}`).join(", ") || "empty"}</Text>
        {regime === null && cart.length > 0 && <Text>No total yet — the shop settings have not loaded.</Text>}
        {owed !== null && <Text>To pay: {formatCentimes(owed)}</Text>}
        <TextInput
          accessibilityLabel="Tendered"
          placeholder="Amount handed over"
          keyboardType="decimal-pad"
          value={tendered}
          onChangeText={(text) => {
            setChange(null);
            setTendered(text);
          }}
        />
        {owed !== null && (
          <Button title="Pay exact" onPress={() => setTendered(formatCentimes(owed))} />
        )}
        {short && <Text>Less than the amount to pay.</Text>}
        <Button title="Pay (cash)" onPress={pay} disabled={!canPay} />
      </View>
    </View>
  );
}
