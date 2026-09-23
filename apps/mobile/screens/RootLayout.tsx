// The root layout: everything that must outlive a route change lives here.
//
// Order matters. The theme wraps everything so a screen never renders
// untokened, and the language sits with it for the same reason: the pairing
// and sign-in screens have words on them and render before anything signed
// in does. The session sits above the router because `(signed-in)/_layout`
// is the auth gate and has to read it. The cart sits above the gate too, so
// a session that idles out mid-sale sends the cashier to a PIN box without
// emptying the basket in front of the customer.
//
// Lives in `screens/`, not `app/`, so both router roots (C7 of
// `the-first-clinic-module-patients-queue-appointments`: `app/` for a
// retail build, `app-clinic/` for one without) point their own `_layout.tsx`
// at this one file rather than carrying two copies of six providers. The
// cart stays mounted in a clinic build too: it holds nothing but a
// product-id-and-quantity array (`providers/CartProvider.tsx`), no money
// math and no server call of its own, so there is nothing here for a
// cabinet build to leak — the screens that give it meaning (`screens/
// signed-in/Till.tsx`, `features/till/`) are what a clinic build excludes.

import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { SafeAreaProvider } from "react-native-safe-area-context";

import { ThemeProvider } from "../design/theme";
import { CartProvider } from "../providers/CartProvider";
import { LanguageProvider } from "../providers/LanguageProvider";
import { QueryProvider } from "../providers/QueryProvider";
import { SessionProvider } from "../providers/SessionProvider";
import { SplashGate } from "../providers/SplashGate";

export default function RootLayout() {
  return (
    <SafeAreaProvider>
      <ThemeProvider>
        <LanguageProvider>
          <QueryProvider>
            <SessionProvider>
              <CartProvider>
                <StatusBar style="auto" />
                <SplashGate>
                  <Stack screenOptions={{ headerShown: false }} />
                </SplashGate>
              </CartProvider>
            </SessionProvider>
          </QueryProvider>
        </LanguageProvider>
      </ThemeProvider>
    </SafeAreaProvider>
  );
}
