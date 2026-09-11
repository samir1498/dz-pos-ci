// The About screen: what it shows is exactly what `GET /build-info`
// answers, never a second copy typed into this file. The three values
// below (`9.9.9-test`, `deadbeef`, `1999-12-31`) are deliberately not the
// app's real version, hash or build date, so a component that hardcoded any
// of the three instead of reading the query would fail here rather than
// pass by coincidence.
//
// Mounted through a memory router carrying `/settings` and `/settings/about`,
// the same shape `settings_.users.test.tsx` uses, because the back link
// builds a real `Link` and a `Link` without a router is a screen that
// cannot render.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import type { BuildInfoDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import { AboutScreen } from "./settings_.about";

const NOT_THE_REAL_BUILD: BuildInfoDto = {
  version: "9.9.9-test",
  git_hash: "deadbeef",
  build_date: "1999-12-31",
  debug: false,
};

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function SettingsStub() {
  return <p>{fr.settings_title}</p>;
}

function app(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const rootRoute = createRootRoute();
  const settingsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings",
    component: SettingsStub,
  });
  const aboutRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings/about",
    component: AboutScreen,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([settingsRoute, aboutRoute]),
    history: createMemoryHistory({ initialEntries: ["/settings/about"] }),
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let answer: BuildInfoDto;

beforeEach(() => {
  answer = NOT_THE_REAL_BUILD;
  fetchMock = vi.fn((input: unknown) => {
    const url = String(input);
    if (url.endsWith("/build-info")) return Promise.resolve(json(200, answer));
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no route" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("AboutScreen", () => {
  test("shows exactly the version, the commit and the build date the server answered, and not a debug badge", async () => {
    app();
    expect(await screen.findByTestId("about-version")).toHaveTextContent("9.9.9-test");
    expect(screen.getByTestId("about-git-hash")).toHaveTextContent("deadbeef");
    expect(screen.getByTestId("about-build-date")).toHaveTextContent("1999-12-31");
    expect(screen.queryByTestId("about-debug-badge")).not.toBeInTheDocument();
  });

  test("a debug build shows the debug badge", async () => {
    answer = { ...NOT_THE_REAL_BUILD, debug: true };
    app();
    expect(await screen.findByTestId("about-debug-badge")).toBeInTheDocument();
  });

  test("the back link returns to settings", async () => {
    const user = userEvent.setup();
    app();
    await screen.findByTestId("about-version");
    await user.click(screen.getByRole("link", { name: fr.action_back_to_settings }));
    expect(await screen.findByText(fr.settings_title)).toBeInTheDocument();
  });
});
