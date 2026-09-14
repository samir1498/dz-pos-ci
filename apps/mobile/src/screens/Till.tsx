import { useEffect, useState } from "react";
import { Button, FlatList, Text, TextInput, View } from "react-native";

import { enqueue, list, retry } from "../lib/queue";

type Product = { id: number; name: string; selling_centimes: number };

export function Till({ apiBase }: { apiBase: string }) {
  const [products, setProducts] = useState<Product[]>([]);
  const [cart, setCart] = useState<{ product: Product; qty: number }[]>([]);
  const [queued, setQueued] = useState(0);

  useEffect(() => {
    fetch(`${apiBase}/products`, {
      headers: { Authorization: `Bearer ${process.env.EXPO_PUBLIC_API_TOKEN ?? ""}` },
    })
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
    const body = JSON.stringify({
      lines: cart.map((c) => ({ product_id: c.product.id, qty_milli: c.qty * 1000 })),
      payment_mode: "cash",
      tendered_centimes: cart.reduce((s, c) => s + c.product.selling_centimes * c.qty, 0),
    });
    try {
      const res = await fetch(`${apiBase}/sales`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          Authorization: `Bearer ${process.env.EXPO_PUBLIC_API_TOKEN ?? ""}`,
        },
        body,
      });
      if (!res.ok) throw new Error(String(res.status));
      const sale = (await res.json()) as { id: number };
      // Print through desktop (M6 T6): same bytes the desktop's own till prints,
      // spooled beside the shop file and optionally pushed to TCP 9100.
      fetch(`${apiBase}/sales/${sale.id}/print?lang=fr`, {
        method: "POST",
        headers: { Authorization: `Bearer ${process.env.EXPO_PUBLIC_API_TOKEN ?? ""}` },
      }).catch(() => {});
      setCart([]);
    } catch {
      await enqueue({ method: "POST", url: `${apiBase}/sales`, body });
      const q = await list();
      setQueued(q.length);
    }
  };

  const retryAll = async () => {
    const { succeeded } = await retry(async (req) => {
      try {
        const res = await fetch(req.url, {
          method: req.method,
          headers: {
            "content-type": "application/json",
            Authorization: `Bearer ${process.env.EXPO_PUBLIC_API_TOKEN ?? ""}`,
          },
          body: req.body,
        });
        return res.ok;
      } catch {
        return false;
      }
    });
    if (succeeded > 0) {
      const q = await list();
      setQueued(q.length);
    }
  };

  return (
    <View style={{ flex: 1, padding: 16 }}>
      <Text style={{ fontWeight: "600" }}>Till — thin client (M6 T5)</Text>
      <Text>Queued: {queued}</Text>
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
