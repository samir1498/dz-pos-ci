// The till. A thin client over the shop's core: it shows what is for sale,
// keeps a basket, and asks the server to ring it. It computes a preview of
// the total so the cashier can tell the customer a number before the sale
// exists, and nothing else.
//
// The basket itself lives in `CartProvider`, above the auth gate, so a
// session that idles out three items into a sale sends the cashier to a PIN
// box and back without the customer's things having to be scanned again.
//
// Owned by retail alone (C7): only `app/(signed-in)/till.tsx` re-exports
// this file; a clinic build's router root (`app-clinic/`) carries no
// `till.tsx` at all, so this screen, `features/till/` and the money math in
// `lib/basket.ts` it reads are unreachable from it.

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
import { useTranslation } from "../../providers/LanguageProvider";
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
  const { t } = useTranslation();
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
              <Button title={t("till_settings")} variant="secondary" />
            </Link>
            <Button title={t("auth_sign_out")} variant="secondary" onPress={() => void signOut()} />
          </View>
        </View>

        {ringing.queued > 0 && (
          <View style={{ flexDirection: "row", alignItems: "center", gap: theme.space[2] }}>
            <Text variant="label" tone="secondary" style={{ flex: 1 }}>
              {t("till_queue_waiting", { count: ringing.queued })}
              {ringing.skipped > 0
                ? ` · ${t("till_queue_other_signin", { count: ringing.skipped })}`
                : ""}
            </Text>
            <Button
              title={t("till_queue_send_now")}
              variant="secondary"
              onPress={() => void ringing.retryQueued()}
              busy={ringing.busy}
            />
          </View>
        )}

        {ringing.notice !== null && (
          <Callout tone="danger">{t(ringing.notice.key, ringing.notice.vars)}</Callout>
        )}
        {ringing.change !== null && (
          <Callout tone="success">
            {t("till_change_due", {
              amount: formatCentimes(ringing.change),
              currency: t("currency_suffix"),
            })}
          </Callout>
        )}

        {products.isError && <Callout tone="danger">{t("till_products_unreadable")}</Callout>}

        <FlatList
          style={{ flex: 1 }}
          data={products.data ?? []}
          keyExtractor={(product) => String(product.id)}
          refreshing={products.isFetching}
          onRefresh={() => void products.refetch()}
          ListEmptyComponent={
            products.isLoading ? null : (
              <Text variant="body" tone="secondary">
                {t("till_products_empty")}
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
