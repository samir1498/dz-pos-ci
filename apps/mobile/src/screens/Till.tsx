import { useEffect, useState } from "react";
import { Button, FlatList, Text, TextInput, View } from "react-native";

import { enqueue, list, newIdempotencyKey, retry } from "../lib/queue";
import type { Session } from "../lib/session";

type Product = { id: number; name: string; selling_centimes: number };

/** The thin till (M6 T5, sessions M7 T3). Every call carries three
 * credentials, each proving one thing: the launch token (build-time env,
 * the server operator's secret, same one the desktop's own window shows)
 * says the caller may reach the server; the device token says which paired
 * phone; the session token says which signed-in person. A 401 names which
 * of the three failed, in the server's error code. */
export function Till({
  apiBase,
  launch,
  session,
  onSignOut,
}: {
  apiBase: string;
  launch: string;
  session: Session;
  onSignOut: () => void;
}) {
  const [products, setProducts] = useState<Product[]>([]);
  const [cart, setCart] = useState<{ product: Product; qty: number }[]>([]);
  const [queued, setQueued] = useState(0);
  const [skipped, setSkipped] = useState(0);

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
    list().then((q) => setQueued(q.length));
  }, [apiBase]);

  const add = (p: Product) => {
    setCart((c) => {
      const found = c.find((x) => x.product.id === p.id);
      if (found) return c.map((x) => (x.product.id === p.id ? { ...x, qty: x.qty + 1 } : x));
      return [...c, { product: p, qty: 1 }];
    });
  };

  const pay = async () => {
    if (cart.length === 0) return;
    // The key is minted before the first try so the try and every retry
    // promise the same sale; the server answers the original on a replay.
    const body = JSON.stringify({
      lines: cart.map((c) => ({ product_id: c.product.id, qty_milli: c.qty * 1000 })),
      payment_mode: "cash",
      tendered_centimes: cart.reduce((s, c) => s + c.product.selling_centimes * c.qty, 0),
      idempotency_key: newIdempotencyKey(),
    });
    try {
      const res = await fetch(`${apiBase}/sales`, {
        method: "POST",
        headers: headers(),
        body,
      });
      if (!res.ok) throw new Error(String(res.status));
      const sale = (await res.json()) as { id: number };
      // Print through desktop (M6 T6): same bytes the desktop's own till prints,
      // spooled beside the shop file and optionally pushed to TCP 9100.
      fetch(`${apiBase}/sales/${sale.id}/print?lang=fr`, {
        method: "POST",
        headers: headers(),
      }).catch(() => {});
      setCart([]);
    } catch {
      await enqueue({
        method: "POST",
        url: `${apiBase}/sales`,
        sessionToken: session.sessionToken,
        body,
      });
      const q = await list();
      setQueued(q.length);
    }
  };

  const retryAll = async () => {
    const outcome = await retry(
      async (req) => {
        try {
          const res = await fetch(req.url, {
            method: req.method,
            headers: headers(),
            body: req.body,
          });
          if (!res.ok) return false;
          // The ring went through on retry: print it like a first try
          // would have. Best-effort — the sale stands either way.
          const sale = (await res.json()) as { id?: number };
          if (typeof sale.id === "number") {
            fetch(`${apiBase}/sales/${sale.id}/print?lang=fr`, {
              method: "POST",
              headers: headers(),
            }).catch(() => {});
          }
          return true;
        } catch {
          return false;
        }
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
        <Button title="Pay (cash)" onPress={pay} />
      </View>
    </View>
  );
}
