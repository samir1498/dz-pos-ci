// The About screen: what it shows is exactly what `GET /build-info`
// answers, never a second copy typed into this file. The three values
// below (`9.9.9-test`, `deadbeef`, `1999-12-31`) are deliberately not the
// app's real version, hash or build date, so a component that hardcoded any
// of the three instead of reading the query would fail here rather than
// pass by coincidence.
//
// Mounted through a memory router, the same shape `settings.users.test.tsx`
// uses: the screen builds real `Link`s and a `Link` without a router is a
// screen that cannot render.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
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
import { AboutScreen } from "./settings.about";

// Only `checkForUpdate` and `installUpdate` are mocked, never
// `formatUpdateSize` (a pure function): the point of these tests is the
// screen reading the three answers `src/lib/updater.ts` can hand back, not
// re-testing Tauri's own `invoke`, which jsdom has no bridge for anyway.
vi.mock("@/lib/updater", async () => {
  const actual = await vi.importActual<typeof import("@/lib/updater")>("@/lib/updater");
  return { ...actual, checkForUpdate: vi.fn(), installUpdate: vi.fn() };
});
import { checkForUpdate, installUpdate } from "@/lib/updater";

const mockedCheck = vi.mocked(checkForUpdate);
const mockedInstall = vi.mocked(installUpdate);

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
  mockedCheck.mockReset();
  mockedInstall.mockReset();
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

});

// The check never runs on its own: every test below presses the button
// itself, and a test with no click at all (not written here, since it
// would prove nothing) would find `checkForUpdate` uncalled.
describe("UpdateCheckPanel", () => {
  async function pressCheck() {
    const user = userEvent.setup();
    app();
    await screen.findByTestId("about-version");
    await user.click(screen.getByTestId("about-update-check"));
    return user;
  }

  test("already on the newest version says so and offers no install", async () => {
    mockedCheck.mockResolvedValueOnce({ kind: "newest" });
    await pressCheck();
    expect(await screen.findByTestId("about-update-newest")).toHaveTextContent(
      fr.about_update_newest,
    );
    expect(screen.queryByTestId("about-update-install")).not.toBeInTheDocument();
  });

  test("the endpoint being unreachable is shown as an alert", async () => {
    mockedCheck.mockResolvedValueOnce({ kind: "unreachable" });
    await pressCheck();
    const alert = await screen.findByTestId("about-update-unreachable");
    expect(alert).toHaveTextContent(fr.about_update_unreachable);
    expect(alert).toHaveAttribute("role", "alert");
  });

  test("a newer version shows its number and weight, and offers to install", async () => {
    mockedCheck.mockResolvedValueOnce({ kind: "newer", version: "2.4.0", size: 31_457_280 });
    await pressCheck();
    expect(await screen.findByTestId("about-update-version")).toHaveTextContent("2.4.0");
    expect(screen.getByTestId("about-update-size")).toHaveTextContent("30.0 MB");
  });

  test("a manifest with no size shows the size-unknown wording rather than a made-up number", async () => {
    mockedCheck.mockResolvedValueOnce({ kind: "newer", version: "2.4.0", size: null });
    await pressCheck();
    expect(await screen.findByTestId("about-update-size")).toHaveTextContent(
      fr.about_update_size_unknown,
    );
  });

  test("install asks for confirmation, tells the shop the app will restart, then installs", async () => {
    mockedCheck.mockResolvedValueOnce({ kind: "newer", version: "2.4.0", size: 1_000_000 });
    mockedInstall.mockResolvedValueOnce(undefined);
    const user = await pressCheck();

    await user.click(await screen.findByTestId("about-update-install"));
    const dialog = await screen.findByTestId("about-update-install-dialog");
    expect(dialog).toHaveTextContent(fr.about_update_confirm_question);

    await user.click(screen.getByTestId("about-update-install-dialog-confirm"));
    await waitFor(() => expect(mockedInstall).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(dialog).not.toBeInTheDocument());
  });

  test("a failed install shows the failure alert rather than a silent no-op", async () => {
    mockedCheck.mockResolvedValueOnce({ kind: "newer", version: "2.4.0", size: 1_000_000 });
    mockedInstall.mockRejectedValueOnce(new Error("nope"));
    const user = await pressCheck();

    await user.click(await screen.findByTestId("about-update-install"));
    await user.click(await screen.findByTestId("about-update-install-dialog-confirm"));

    const alert = await screen.findByTestId("about-update-install-failed");
    expect(alert).toHaveTextContent(fr.about_update_install_failed);
  });
});
