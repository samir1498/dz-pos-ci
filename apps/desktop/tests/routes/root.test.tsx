// T2/T3 (context/plans/20260924-shop-manual-test-findings.md): the
// language and theme choices used to sit only in the signed-in shell's
// topbar, so an owner claiming a brand-new shop, or anyone at the sign-in
// screen, never saw either one. `FloatingControls` now mounts in the root
// layout itself (`routes/__root.tsx`), above whichever screen the session
// status picks, so both are reachable before a session exists too, and
// exactly once, not doubled with a screen's own copy.

import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { render, screen } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";

import { I18nProvider } from "@/i18n";
import { SessionProvider } from "@/lib/session";
import { ThemeProvider } from "@/lib/theme";

import { Route as RootRoute } from "../../src/routes/__root";

const json = (status: number, body: unknown): Response =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

const notFound = () => json(404, { error: { code: "not_found", message: "no" } });

afterEach(() => {
  vi.unstubAllGlobals();
});

async function mount() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const indexRoute = createRoute({
    getParentRoute: () => RootRoute,
    path: "/",
    component: () => <p>écran</p>,
  });
  const router = createRouter({
    routeTree: RootRoute.addChildren([indexRoute]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  render(
    <I18nProvider lang="fr">
      <QueryClientProvider client={client}>
        <ThemeProvider>
          <SessionProvider>
            <RouterProvider router={router} />
          </SessionProvider>
        </ThemeProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

test("the floating controls are on screen before any owner has claimed the shop", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/health")) {
        return Promise.resolve(json(200, { status: "ok", shop_id: 1, needs_first_setup: true }));
      }
      return Promise.resolve(notFound());
    }),
  );
  await mount();
  await screen.findByTestId("setup-screen");
  expect(screen.getByTestId("floating-controls")).toBeInTheDocument();
});

test("the floating controls are on screen at sign-in, and only once each", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/health")) {
        return Promise.resolve(json(200, { status: "ok", shop_id: 1, needs_first_setup: false }));
      }
      if (url.endsWith("/auth/me")) {
        return Promise.resolve(json(401, { error: { code: "session_required", message: "no" } }));
      }
      return Promise.resolve(notFound());
    }),
  );
  await mount();
  await screen.findByTestId("signin-screen");
  expect(screen.getAllByTestId("language-switcher")).toHaveLength(1);
  expect(screen.getAllByTestId("theme-switcher")).toHaveLength(1);
});
