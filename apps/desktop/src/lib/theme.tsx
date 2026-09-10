// The whole theme mechanism on this side: read the shop's choice, work out
// which name applies, and put it on `<html>` as `data-theme`. Nothing else.
//
// Every theme is a block of CSS variables in the generated theme.css, so a
// component never learns which one is on. There is no `theme === "registre"
// ? a : b` anywhere in the app, no per-theme component and no conditional
// class list; `src/theme.test.ts` greps for exactly that and fails the gates
// on it. The attribute goes on the document element and not on a wrapper,
// because the shadcn names (`--background` and the rest) are declared once in
// `:root` pointing at our roles, and they only follow a theme when the block
// that redefines those roles lands on the same element.
//
// The choice lives in the shop file, so a second machine in the same shop
// opens on the same theme. It is mirrored into localStorage on the way past,
// which is what the blocking script in index.html reads: the API answer
// arrives after first paint, and without the mirror a return visitor would
// see a flash of Comptoir before their dark theme arrived.

import { OS_THEME } from "@dzpos/design";
import type { SettingsDto, ThemeDto } from "@dzpos/shared";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

import { api, settingsQueryKey } from "@/api";

/** Read by the blocking script in index.html before React exists. */
export const STORAGE_KEY = "dzpos-theme";

/** The media query the two OS-driven themes are chosen by. */
const DARK_QUERY = "(prefers-color-scheme: dark)";

interface Ctx {
  /** What the shop saved. `null` is a choice: follow the machine. */
  readonly choice: ThemeDto | null;
  /** The name actually on `<html>`, which is what a screenshot shows. */
  readonly resolved: ThemeDto;
  readonly setChoice: (theme: ThemeDto | null) => void;
  readonly saving: boolean;
}

const ThemeContext = createContext<Ctx | null>(null);

/**
 * `false` where the browser has no matchMedia. A Tauri webview always has
 * one; jsdom does not, and neither does an embed with the API turned off, so
 * the app falls to the light theme rather than throwing on mount.
 */
function prefersDark(): boolean {
  if (typeof window.matchMedia !== "function") return false;
  return window.matchMedia(DARK_QUERY).matches;
}

function useSystemDark(): boolean {
  const [dark, setDark] = useState(prefersDark);
  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const query = window.matchMedia(DARK_QUERY);
    const onChange = (event: MediaQueryListEvent) => setDark(event.matches);
    query.addEventListener("change", onChange);
    // The machine may have flipped between the first render and this effect.
    setDark(query.matches);
    return () => query.removeEventListener("change", onChange);
  }, []);
  return dark;
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const dark = useSystemDark();
  // The settings page is one object and the theme rides on it, so this shares
  // the key the settings screen already reads rather than adding a route the
  // shop file would have to answer twice.
  const settings = useQuery({ queryKey: settingsQueryKey, queryFn: () => api.getSettings() });
  const choice = settings.data?.theme ?? null;

  const save = useMutation({
    mutationFn: (theme: ThemeDto | null) => api.setTheme(theme),
    onSuccess: (answer: SettingsDto) => {
      queryClient.setQueryData<SettingsDto>(settingsQueryKey, answer);
    },
  });

  const resolved: ThemeDto = choice ?? (dark ? OS_THEME.dark : OS_THEME.light);

  useEffect(() => {
    document.documentElement.dataset.theme = resolved;
    try {
      window.localStorage.setItem(STORAGE_KEY, resolved);
    } catch {
      // Storage blocked (private mode, an embed): the app still themes
      // itself, it just repaints once on the next load.
    }
  }, [resolved]);

  const value = useMemo<Ctx>(
    () => ({
      choice,
      resolved,
      setChoice: (theme) => save.mutate(theme),
      saving: save.isPending,
    }),
    [choice, resolved, save],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): Ctx {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme outside ThemeProvider");
  return ctx;
}
