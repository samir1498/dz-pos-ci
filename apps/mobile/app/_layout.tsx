// The root layout: everything that must outlive a route change lives here.
//
// Order matters. The theme wraps everything so a screen never renders
// untokened. The session sits above the router because `(signed-in)/_layout`
// is the auth gate and has to read it. The cart sits above the gate too, so
// a session that idles out mid-sale sends the cashier to a PIN box without
// emptying the basket in front of the customer.

import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { SafeAreaProvider } from "react-native-safe-area-context";

import { ThemeProvider } from "../design/theme";
import { CartProvider } from "../providers/CartProvider";
import { QueryProvider } from "../providers/QueryProvider";
import { SessionProvider } from "../providers/SessionProvider";

export default function RootLayout() {
  return (
    <SafeAreaProvider>
      <ThemeProvider>
        <QueryProvider>
          <SessionProvider>
            <CartProvider>
              <StatusBar style="auto" />
              <Stack screenOptions={{ headerShown: false }} />
            </CartProvider>
          </SessionProvider>
        </QueryProvider>
      </ThemeProvider>
    </SafeAreaProvider>
  );
}
