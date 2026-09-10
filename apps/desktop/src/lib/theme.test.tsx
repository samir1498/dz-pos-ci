// What the provider decides, and what it leaves on the document.
//
// The rules worth pinning: the shop's saved choice wins, no saved choice is
// Comptoir whatever the machine's light or dark setting says, and whatever
// wins ends up in two places, the `data-theme` attribute the stylesheet reads
// and the localStorage key the blocking script in index.html reads before
// React exists.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import type { SettingsDto, ThemeDto } from "@dzpos/shared";

import { ThemeSwitcher } from "@/components/ThemeSwitcher";
import { I18nProvider } from "@/i18n";
import { STORAGE_KEY, ThemeProvider, useTheme } from "@/lib/theme";

const settings: SettingsDto = {
  store: {
    name: "Mon magasin",
    rc: null,
    nif: null,
    nis: null,
    ai: null,
    address: null,
    phone: null,
  },
  regime: { regime: "reel", valid_from: "2026-01-01" },
  regime_planned: null,
  theme: null,
};

let fetchMock: ReturnType<typeof vi.fn>;
let current: SettingsDto;
let puts: unknown[];

function json(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

function isInit(value: unknown): value is RequestInit {
  return typeof value === "object" && value !== null;
}

/** The machine says dark, or does not. */
function machineSays(dark: boolean) {
  window.matchMedia = (query: string): MediaQueryList => ({
    matches: dark,
    media: query,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  });
}

beforeEach(() => {
  current = settings;
  puts = [];
  machineSays(false);
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-theme");
  fetchMock = vi.fn((input: unknown, init?: unknown) => {
    const url = String(input);
    if (isInit(init) && init.method === "PUT" && url.endsWith("/settings/theme")) {
      const body: unknown = JSON.parse(String(init.body));
      puts.push(body);
      const chosen =
        typeof body === "object" && body !== null && "theme" in body ? body.theme : null;
      current = { ...current, theme: chosen === null ? null : toTheme(String(chosen)) };
      return Promise.resolve(json(current));
    }
    return Promise.resolve(json(current));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

/** Narrows without an `as`; the schema has already vetted the wire value. */
function toTheme(value: string): ThemeDto | null {
  const names: readonly ThemeDto[] = ["comptoir", "registre", "observe", "observe-dark"];
  return names.find((name) => name === value) ?? null;
}

function Reads() {
  const { resolved, choice } = useTheme();
  return (
    <>
      <span data-testid="resolved">{resolved}</span>
      <span data-testid="choice">{choice ?? "none"}</span>
    </>
  );
}

function renderWith(children: React.ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <I18nProvider lang="fr">
      <QueryClientProvider client={client}>
        <ThemeProvider>{children}</ThemeProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

describe("the theme the app opens on", () => {
  test("opens on Comptoir when nothing is saved and the machine is light", async () => {
    renderWith(<Reads />);
    await waitFor(() => expect(screen.getByTestId("resolved")).toHaveTextContent("comptoir"));
    expect(screen.getByTestId("choice")).toHaveTextContent("none");
    expect(document.documentElement.dataset.theme).toBe("comptoir");
  });

  test("opens on Comptoir when nothing is saved and the machine is dark", async () => {
    machineSays(true);
    renderWith(<Reads />);
    await waitFor(() => expect(screen.getByTestId("resolved")).toHaveTextContent("comptoir"));
    expect(document.documentElement.dataset.theme).toBe("comptoir");
  });

  /** The shop chose; the machine's preference stops being consulted. */
  test("a saved theme beats the machine, dark or light", async () => {
    machineSays(true);
    current = { ...settings, theme: "observe" };
    renderWith(<Reads />);
    await waitFor(() => expect(screen.getByTestId("resolved")).toHaveTextContent("observe"));
    expect(document.documentElement.dataset.theme).toBe("observe");
  });

  /**
   * The blocking script in index.html reads this key before React exists.
   * Without it a shop on a dark theme gets a white flash on every load.
   */
  test("mirrors the resolved theme into storage for the next first paint", async () => {
    current = { ...settings, theme: "observe-dark" };
    renderWith(<Reads />);
    await waitFor(() => expect(window.localStorage.getItem(STORAGE_KEY)).toBe("observe-dark"));
  });
});

describe("the switcher", () => {
  test("puts each theme the shop picks and repaints the document", async () => {
    const user = userEvent.setup();
    renderWith(
      <>
        <ThemeSwitcher />
        <Reads />
      </>,
    );
    await waitFor(() => expect(screen.getByTestId("resolved")).toHaveTextContent("comptoir"));

    await user.selectOptions(screen.getByTestId("theme-switcher"), "observe-dark");
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("observe-dark"));
    expect(puts).toEqual([{ theme: "observe-dark" }]);
  });

  /** Comptoir chosen on purpose is stored as Comptoir, never as "nothing". */
  test("choosing Comptoir sends comptoir, not null, whatever the machine says", async () => {
    const user = userEvent.setup();
    machineSays(true);
    current = { ...settings, theme: "observe" };
    renderWith(
      <>
        <ThemeSwitcher />
        <Reads />
      </>,
    );
    await waitFor(() => expect(screen.getByTestId("resolved")).toHaveTextContent("observe"));

    await user.selectOptions(screen.getByTestId("theme-switcher"), "comptoir");
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("comptoir"));
    expect(puts).toEqual([{ theme: "comptoir" }]);
  });

  test("offers every theme the design package emits a block for", async () => {
    renderWith(<ThemeSwitcher />);
    const options = await screen.findAllByRole("option");
    expect(options.map((option) => option.getAttribute("value"))).toEqual([
      "comptoir",
      "registre",
      "observe",
      "observe-dark",
    ]);
  });
});
