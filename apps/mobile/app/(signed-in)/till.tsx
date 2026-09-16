// The till. A thin client over the shop's core: it shows what is for sale,
// keeps a basket, and asks the server to ring it. It computes a preview of
// the total so the cashier can tell the customer a number before the sale
// exists, and nothing else.
//
// The basket itself lives in `CartProvider`, above the auth gate, so a
// session that idles out three items into a sale sends the cashier to a PIN
// box and back without the customer's things having to be scanned again.

import { Link } from "expo-router";
import { useMemo, useState } from "react";
import { FlatList, View } from "react-native";

import { Button, Callout, Screen, Text } from "../../components/ui";
import { useTheme } from "../../design/theme";
import { PayPanel } from "../../features/till/PayPanel";
import { ProductRow } from "../../features/till/ProductRow";
import { useProducts, useRegime } from "../../features/till/queries";
import { useRing } from "../../features/till/useRing";
import { formatCentimes, priceBasket, readTendered } from "../../lib/basket";
import type { Session } from "../../lib/session";
import { useCart } from "../../providers/CartProvider";
import { useSession } from "../../providers/SessionProvider";

// The gate above this route guarantees a session, but the type does not, and
// there is one frame between a sign-out and the redirect where it is null.
// Narrowing out here rather than inside `Counter` is what keeps that frame
// from changing how many hooks the component calls.
export default function Till() {
  const { session } = useSession();
  if (session === null) return <Screen />;
  return <Counter session={session} />;
}

function Counter({ session }: { session: Session }) {
  const theme = useTheme();
  const { sessionLost, deviceLost, signOut } = useSession();
  const { lines, add, clear } = useCart();
  const [tendered, setTendered] = useState("");

  const products = useProducts(session);
  const regime = useRegime(session);
  const ringing = useRing({
    session,
    lines,
    onRung: () => {
      clear();
      setTendered("");
    },
    onSessionLost: sessionLost,
    onDeviceLost: deviceLost,
  });

  // What the shop is owed, priced the way the core prices it. A basket the
  // money rules cannot express (past what a Number holds exactly) leaves
  // this null, and the till says so instead of posting a total it guessed.
  const owed = useMemo(() => {
    if (regime.data == null || lines.length === 0) return null;
    try {
      return priceBasket(lines, regime.data).netToPay;
    } catch {
      return null;
    }
  }, [lines, regime.data]);

  const handed = readTendered(tendered);
  const short = owed !== null && handed !== null && handed < owed;
  const canPay = owed !== null && handed !== null && !short && !ringing.busy;

  return (
    <Screen>
      <View style={{ gap: theme.space[3], flex: 1 }}>
        <View style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between" }}>
          <View>
            <Text variant="heading">{session.name}</Text>
            <Text variant="caption" tone="secondary">
              {session.role}
            </Text>
          </View>
          <View style={{ flexDirection: "row", gap: theme.space[2] }}>
            <Link href="/settings" asChild>
              <Button title="Settings" variant="secondary" />
            </Link>
            <Button title="Sign out" variant="secondary" onPress={() => void signOut()} />
          </View>
        </View>

        {ringing.queued > 0 && (
          <View style={{ flexDirection: "row", alignItems: "center", gap: theme.space[2] }}>
            <Text variant="label" tone="secondary" style={{ flex: 1 }}>
              {ringing.queued} sale{ringing.queued === 1 ? "" : "s"} waiting to be sent
              {ringing.skipped > 0 ? ` · ${ringing.skipped} under another sign-in` : ""}
            </Text>
            <Button
              title="Send now"
              variant="secondary"
              onPress={() => void ringing.retryQueued()}
              busy={ringing.busy}
            />
          </View>
        )}

        {ringing.notice !== null && <Callout tone="danger">{ringing.notice}</Callout>}
        {ringing.change !== null && (
          <Callout tone="success">{`Change: ${formatCentimes(ringing.change)} DA`}</Callout>
        )}

        {products.isError && (
          <Callout tone="danger">
            Could not read what is for sale. Check the Wi-Fi, then pull to refresh.
          </Callout>
        )}

        <FlatList
          style={{ flex: 1 }}
          data={products.data ?? []}
          keyExtractor={(product) => String(product.id)}
          refreshing={products.isFetching}
          onRefresh={() => void products.refetch()}
          ListEmptyComponent={
            products.isLoading ? null : (
              <Text variant="body" tone="secondary">
                Nothing for sale yet — add products on the till computer.
              </Text>
            )
          }
          renderItem={({ item }) => (
            <ProductRow
              product={item}
              inCart={lines.find((line) => line.product.id === item.id)?.qty ?? 0}
              onAdd={() => {
                ringing.dismiss();
                add(item);
              }}
            />
          )}
        />

        <PayPanel
          lines={lines}
          owed={owed}
          tendered={tendered}
          onTenderedChange={(typed) => {
            ringing.dismiss();
            setTendered(typed);
          }}
          onPay={() => {
            if (handed !== null) void ringing.ring(handed);
          }}
          onClear={clear}
          busy={ringing.busy}
          short={short}
          canPay={canPay}
          noRegime={regime.data == null && lines.length > 0 && !regime.isLoading}
        />
      </View>
    </Screen>
  );
}
