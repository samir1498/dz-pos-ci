// The root layout: everything that must outlive a route change lives here.
//
// Order matters. The theme wraps everything so a screen never renders
// untokened, and the language sits with it for the same reason: the pairing
// and sign-in screens have words on them and render before anything signed
// in does. The session sits above the router because `(signed-in)/_layout`
// is the auth gate and has to read it. The cart sits above the gate too, so
// a session that idles out mid-sale sends the cashier to a PIN box without
// emptying the basket in front of the customer.

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
