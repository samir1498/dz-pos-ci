// The phone's end of the shared design system.
//
// dz-pos already has the three-tier token pipeline the desktop is built on:
// `packages/design` resolves primitives → semantic roles → one plain object
// per theme. Its own header says why that object is plain rather than a
// hook: "React Native has no cascade, so mobile picks the object and
// re-renders." Nobody had ever wired the phone to it. This file is that
// wiring, and it is deliberately thin — no second set of tokens lives here,
// so a colour the desktop changes is a colour the phone changes.
//
// Four themes ship. Only one of them is dark (`observe-dark`), so the phone
// follows the system's preference between that and Comptoir, the shop's
// default. A shop-chosen theme is a later setting; the tokens already allow
// it, which is the point of wiring the package rather than copying values.

import { resolvedThemes, type Theme } from "@dzpos/design";
import { createContext, useContext, useMemo, type ReactNode } from "react";
import { useColorScheme } from "react-native";

const ThemeContext = createContext<Theme>(resolvedThemes.comptoir);

/** The resolved tokens for the theme in force. Safe at any depth. */
export const useTheme = (): Theme => useContext(ThemeContext);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const scheme = useColorScheme();
  const value = useMemo(
    () => (scheme === "dark" ? resolvedThemes["observe-dark"] : resolvedThemes.comptoir),
    [scheme],
  );
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export type { Theme };
