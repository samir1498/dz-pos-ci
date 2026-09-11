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
import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { I18nProvider, type Lang } from "@/i18n";
import { ThemeProvider } from "@/lib/theme";
import ar from "@/i18n/ar.json";
import fr from "@/i18n/fr.json";

import { AppShell, NAV, activeItem } from "./AppShell";

const SHOP_TODAY = "2026-09-10";
const SHOP_NAME = "Superette El Baraka";

const settings = {
  store: { name: SHOP_NAME, rc: null, nif: null, nis: null, ai: null, address: null, phone: null },
  regime: { regime: "reel", valid_from: "2026-01-01" },
  regime_planned: null,
  theme: null,
};

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
            query, so the provider sits inside the query client. */}
        <ThemeProvider>
          <RouterProvider router={router} />
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
    // Nobody is signed in in this suite's default fetch stub, so the one
    // item that carries a `permission` (the audit log, M4 T7) is left out
    // here and covered on its own below.
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
  });

  test("a nav entry gated on a permission shows once the session holds it", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL) => {
        const url = String(input);
        if (url.endsWith("/auth/me")) {
          return Promise.resolve(
            json(200, {
              user_id: 1,
              name: "Yasmine",
              role: "owner",
              permissions: ["see_audit_log"],
            }),
          );
        }
        if (url.endsWith("/clock")) return Promise.resolve(json(200, { today: SHOP_TODAY }));
        if (url.endsWith("/settings")) return Promise.resolve(json(200, settings));
        return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
      }),
    );
    await mount("/till");
    expect(await screen.findByTestId("nav-audit")).toBeInTheDocument();
  });

  test("the topbar heads the page with the name the sidebar uses", async () => {
    await mount("/customers");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(fr.nav_customers);
  });

  test("marks the current screen, and only it", async () => {
    await mount("/expenses");
    const current = screen
      .getAllByRole("link")
      .filter((link) => link.getAttribute("data-active") === "true");
    expect(current).toHaveLength(1);
    expect(current[0]).toHaveTextContent(fr.nav_expenses);
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

  test("carries both switches in the topbar", async () => {
    await mount("/till");
    const topbar = screen.getByTestId("shell-topbar");
    expect(topbar).toContainElement(screen.getByTestId("theme-switcher"));
    expect(topbar).toContainElement(screen.getByTestId("language-switcher"));
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
