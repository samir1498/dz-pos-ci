// The frame, rendered around a stand-in screen inside a real router, because
// the two things it does that could break silently both depend on where the
// router says it is: which sidebar item is marked current, and what the
// topbar calls the page.
//
// The sheet the sidebar becomes on a narrow window is not opened here. It is
// a Radix dialog in a portal and jsdom has neither pointer capture nor
// `scrollIntoView`; the narrow window is `kit.spec.ts`'s to prove, in a
// browser that has both.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { I18nProvider, type Lang } from "@/i18n";
import { SessionProvider } from "@/lib/session";
import { ThemeProvider } from "@/lib/theme";
import ar from "@/i18n/ar.json";
import fr from "@/i18n/fr.json";

import { SETTINGS } from "@/test/settings";
import { AppShell, NAV, activeItem } from "../../src/components/AppShell";

const SHOP_TODAY = "2026-09-10";
const SHOP_NAME = "Superette El Baraka";

const settings = { ...SETTINGS, store: { ...SETTINGS.store, name: SHOP_NAME } };

const json = (status: number, body: unknown): Response =>
  new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });

beforeEach(() => {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/clock")) return Promise.resolve(json(200, { today: SHOP_TODAY }));
      if (url.endsWith("/settings")) return Promise.resolve(json(200, settings));
      return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
    }),
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
});

/**
 * The router resolves its first match in an effect, so nothing is on the
 * screen at the end of `render`. Every test here waits for the topbar once
 * rather than reaching for `findBy` on whatever it happens to assert first.
 */
async function mount(path: string, lang: Lang = "fr") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const rootRoute = createRootRoute({
    component: () => (
      <AppShell>
        <p>écran</p>
      </AppShell>
    ),
  });
  const children = NAV.map((item) =>
    createRoute({ getParentRoute: () => rootRoute, path: item.to, component: () => null }),
  );
  const router = createRouter({
    routeTree: rootRoute.addChildren(children),
    history: createMemoryHistory({ initialEntries: [path] }),
  });
  const view = render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        {/* The same nesting main.tsx uses: the theme rides on the settings
            query, so the provider sits inside the query client, and
            `UserMenu` in the topbar reads `useSession` so the shell needs
            one even though none of these tests signs in. */}
        <ThemeProvider>
          <SessionProvider>
            <RouterProvider router={router} />
          </SessionProvider>
        </ThemeProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
  await screen.findByTestId("shell-topbar");
  return view;
}

describe("activeItem", () => {
  /**
   * The longest match wins. Without that, `/purchases/new` would light up
   * whichever of the two entries the list happened to reach first.
   */
  test("a fiche under a screen still belongs to that screen", () => {
    expect(activeItem("/customers/7")?.to).toBe("/customers");
    expect(activeItem("/purchases/new")?.to).toBe("/purchases");
  });

  test("an exact path is its own item", () => {
    expect(activeItem("/till")?.to).toBe("/till");
  });

  /** A route with no place in the navigation is a real state, not a crash. */
  test("a path the navigation does not know has no item", () => {
    expect(activeItem("/kit")).toBeUndefined();
  });

  test("no two entries claim the same path", () => {
    expect(new Set(NAV.map((item) => item.to)).size).toBe(NAV.length);
  });
});

describe("AppShell", () => {
  test("renders the screen it was given inside the frame", async () => {
    await mount("/till");
    expect(await screen.findByText("écran")).toBeInTheDocument();
    expect(screen.getByTestId("shell-topbar")).toBeInTheDocument();
  });

  test("shows every unconditional screen in the sidebar, grouped by section", async () => {
    // Nobody is signed in in this suite's default fetch stub, so the three
    // items that carry a `permission` (the audit log, M4 T7; the dashboard
    // and purchases entries, M4 T5) are left out here and covered on their
    // own below. Each of the three sections keeps at least one unconditional
    // entry, so the group headings still all render.
    const { container } = await mount("/till");
    for (const item of NAV.filter((item) => item.permission === undefined)) {
      expect(screen.getByTestId(`nav-${item.to.slice(1)}`)).toBeInTheDocument();
    }
    // Read off the group headings rather than searched for by text: the
    // section "Achats" and the screen "Achats" are the same word, and a text
    // query would find two nodes and fail on the ambiguity rather than on
    // anything being wrong.
    const sections = [...container.querySelectorAll('[data-slot="sidebar-group-label"]')].map(
      (node) => node.textContent,
    );
    expect(sections).toEqual([fr.nav_section_sales, fr.nav_section_purchases, fr.nav_section_manage]);
  });

  test("a nav entry gated on a permission is hidden while nobody is signed in", async () => {
    await mount("/till");
    expect(screen.queryByTestId("nav-audit")).not.toBeInTheDocument();
    // The dashboard and purchases entries joined the audit log on the M4 T5
    // review (2026-09-11): the server now refuses GET /dashboard and
    // GET /purchases outright to a cashier, so the link into either has to
    // go the same way the audit log's already did.
    expect(screen.queryByTestId("nav-dashboard")).not.toBeInTheDocument();
    expect(screen.queryByTestId("nav-purchases")).not.toBeInTheDocument();
  });

  /** A session holding just the one permission a fetch stub names, the way
   *  `mount`'s default stub signs nobody in at all. */
  function stubSignedIn(permissions: string[]) {
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = String(input);
        if (url.endsWith("/auth/me")) {
          return Promise.resolve(
            json(200, { user_id: 1, name: "Yasmine", role: "owner", permissions }),
          );
        }
        // `SessionProvider` awaits both `/auth/me` and `/auth/idle` before it
        // sets `me`, so a stub that answers the first and 404s the second
        // never reaches "signed-in" at all (M4 T5, `hasPermission` reads
        // `me` off this provider now, not a query of its own).
        if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
        if (url.endsWith("/clock")) return Promise.resolve(json(200, { today: SHOP_TODAY }));
        if (url.endsWith("/settings")) return Promise.resolve(json(200, settings));
        return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
      }),
    );
  }

  test("a nav entry gated on a permission shows once the session holds it", async () => {
    stubSignedIn(["see_audit_log"]);
    await mount("/till");
    expect(await screen.findByTestId("nav-audit")).toBeInTheDocument();
  });

  test("the dashboard entry shows once the session holds see_reports, and only then", async () => {
    stubSignedIn(["see_reports"]);
    await mount("/till");
    expect(await screen.findByTestId("nav-dashboard")).toBeInTheDocument();
    expect(screen.queryByTestId("nav-purchases")).not.toBeInTheDocument();
  });

  test("the purchases entry shows once the session holds see_cost_and_margin, and only then", async () => {
    stubSignedIn(["see_cost_and_margin"]);
    await mount("/till");
    expect(await screen.findByTestId("nav-purchases")).toBeInTheDocument();
    expect(screen.queryByTestId("nav-dashboard")).not.toBeInTheDocument();
  });

  test("the topbar heads the page with the name the sidebar uses", async () => {
    await mount("/customers");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(fr.nav_customers);
  });

  test("marks the current screen, and only it", async () => {
    // Customers, not expenses: expenses carries a permission since the
    // closing review and the session this mounts with holds none, so the
    // link it used to look for is not drawn at all.
    await mount("/customers");
    const current = screen
      .getAllByRole("link")
      .filter((link) => link.getAttribute("data-active") === "true");
    expect(current).toHaveLength(1);
    expect(current[0]).toHaveTextContent(fr.nav_customers);
  });

  /**
   * The closing review found both routes readable by a cashier on the server
   * while the dashboard that sums them was refused. Now that the server
   * refuses them, the sidebar stops offering a door that only leads to a
   * refusal, the same way it already did for the dashboard and purchases.
   */
  test("the expenses entry waits for see_reports", async () => {
    await mount("/till");
    expect(screen.queryByTestId("nav-expenses")).not.toBeInTheDocument();
    cleanup();
    stubSignedIn(["see_reports"]);
    await mount("/till");
    expect(await screen.findByTestId("nav-expenses")).toBeInTheDocument();
  });

  test("the suppliers entry waits for see_cost_and_margin", async () => {
    await mount("/till");
    expect(screen.queryByTestId("nav-suppliers")).not.toBeInTheDocument();
    cleanup();
    stubSignedIn(["see_cost_and_margin"]);
    await mount("/till");
    expect(await screen.findByTestId("nav-suppliers")).toBeInTheDocument();
  });

  /**
   * The day comes from the server's calendar, never from `new Date()`: a
   * machine set wrong, or carried across a border, is on another day than the
   * shop's ledger.
   */
  test("shows the shop's day, asked of the server", async () => {
    await mount("/till");
    expect(await screen.findByTestId("shell-day")).toHaveTextContent(SHOP_TODAY);
  });

  test("names the shop at the foot of the sidebar", async () => {
    await mount("/till");
    await waitFor(() => expect(screen.getByTestId("shell-shop")).toHaveTextContent(SHOP_NAME));
  });

  /**
   * The two switches moved out of the topbar into `FloatingControls`
   * (`routes/__root.tsx`), a fixed corner reachable before a session
   * exists too (T2/T3). This guards against either one drifting back in
   * beside `FloatingControls`'s own copy, which would put the same choice
   * on the screen twice.
   */
  test("no longer carries the language and theme switches itself", async () => {
    await mount("/till");
    await screen.findByTestId("shell-topbar");
    expect(screen.queryByTestId("theme-switcher")).not.toBeInTheDocument();
    expect(screen.queryByTestId("language-switcher")).not.toBeInTheDocument();
  });

  test("says everything in the language that is on", async () => {
    await mount("/customers", "ar");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(ar.nav_customers);
    expect(screen.getByText(ar.nav_section_sales)).toBeInTheDocument();
  });

  /**
   * The panel's edge, its border and the half it slides in from all follow
   * the same `side`, so the shell computes it from the page direction rather
   * than leaving the sidebar on the left of an Arabic screen.
   */
  test("puts the sidebar on the reading side", async () => {
    const { container, unmount } = await mount("/till", "fr");
    expect(container.querySelector('[data-slot="sidebar"]')).toHaveAttribute("data-side", "left");
    unmount();
    const arabic = await mount("/till", "ar");
    expect(arabic.container.querySelector('[data-slot="sidebar"]')).toHaveAttribute(
      "data-side",
      "right",
    );
  });
});
