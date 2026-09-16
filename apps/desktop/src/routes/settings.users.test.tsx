// The users screen: what it lists, what it posts, and what it shows when the
// server refuses. The rules themselves (a cashier or a manager refused
// `ManageUsers`, the last owner, an owner's own row) are the API crate's
// tests (`crates/api/tests/users_api.rs`) and the core's
// (`crates/core/tests/users_service.rs`); what these hold is the wiring.
//
// Mounted through a memory router carrying `/settings` and `/settings/users`,
// the same two paths the app has, because the back link builds a real `Link`
// and a `Link` without a router is a screen that cannot render.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import type { UserDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import { UsersScreen } from "./settings.users";

const owner: UserDto = {
  id: 1,
  shop_id: 1,
  name: "Amel",
  role: "owner",
  has_pin: true,
  has_password: false,
  active: true,
};

const cashier: UserDto = {
  id: 2,
  shop_id: 1,
  name: "Yacine",
  role: "cashier",
  has_pin: false,
  has_password: false,
  active: true,
};

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function isInit(value: unknown): value is RequestInit {
  return typeof value === "object" && value !== null;
}

function posts(): { url: string; body: unknown }[] {
  const found: { url: string; body: unknown }[] = [];
  for (const call of fetchMock.mock.calls) {
    const init: unknown = call[1];
    if (!isInit(init) || init.method !== "POST") continue;
    found.push({
      url: String(call[0]),
      body: typeof init.body === "string" ? JSON.parse(init.body) : null,
    });
  }
  return found;
}

/** The fiches the screen lists, without the table's header row. */
function userRows(): HTMLElement[] {
  return within(screen.getByTestId("users-table")).queryAllByRole("row").slice(1);
}

function SettingsStub() {
  return <p>{fr.settings_title}</p>;
}

/** The screen under a router of its own, carrying the two paths the app has
 *  so the back link's `Link` can render and be followed. */
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
  const usersRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings/users",
    component: UsersScreen,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([settingsRoute, usersRoute]),
    history: createMemoryHistory({ initialEntries: ["/settings/users"] }),
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
let listed: UserDto[];
let listAnswer: (() => Response) | null;
let createAnswer: (() => Response) | null;
let pinAnswer: (() => Response) | null;
let deactivateAnswer: (() => Response) | null;
let reactivateAnswer: (() => Response) | null;

beforeEach(() => {
  listed = [owner, cashier];
  listAnswer = null;
  createAnswer = null;
  pinAnswer = null;
  deactivateAnswer = null;
  reactivateAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    const method = init?.method ?? "GET";

    if (method === "GET" && url.endsWith("/users")) {
      if (listAnswer !== null) return Promise.resolve(listAnswer());
      return Promise.resolve(json(200, listed));
    }
    if (method === "POST" && url.endsWith("/users")) {
      if (createAnswer !== null) return Promise.resolve(createAnswer());
      const body: { name: string; role: UserDto["role"] } = JSON.parse(String(init?.body));
      const made: UserDto = {
        id: 3,
        shop_id: 1,
        name: body.name,
        role: body.role,
        has_pin: false,
        has_password: false,
        active: true,
      };
      listed = [...listed, made];
      return Promise.resolve(json(201, made));
    }
    if (method === "POST" && url.endsWith("/pin")) {
      if (pinAnswer !== null) return Promise.resolve(pinAnswer());
      const id = Number(url.split("/").slice(-2)[0]);
      listed = listed.map((row) => (row.id === id ? { ...row, has_pin: true } : row));
      const updated = listed.find((row) => row.id === id);
      return Promise.resolve(json(200, updated));
    }
    if (method === "POST" && url.endsWith("/deactivate")) {
      if (deactivateAnswer !== null) return Promise.resolve(deactivateAnswer());
      const id = Number(url.split("/").slice(-2)[0]);
      listed = listed.map((row) => (row.id === id ? { ...row, active: false } : row));
      const updated = listed.find((row) => row.id === id);
      return Promise.resolve(json(200, updated));
    }
    if (method === "POST" && url.endsWith("/reactivate")) {
      if (reactivateAnswer !== null) return Promise.resolve(reactivateAnswer());
      const id = Number(url.split("/").slice(-2)[0]);
      listed = listed.map((row) => (row.id === id ? { ...row, active: true } : row));
      const updated = listed.find((row) => row.id === id);
      return Promise.resolve(json(200, updated));
    }
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the list", () => {
  test("shows one row per fiche, with its role", async () => {
    app();
    await screen.findByTestId("users-table");
    const rows = userRows();
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("Amel");
    expect(rows[0]).toHaveTextContent(fr.role_owner);
    expect(rows[1]).toHaveTextContent("Yacine");
    expect(rows[1]).toHaveTextContent(fr.role_cashier);
  });

  test("marks a fiche with no PIN yet", async () => {
    app();
    await screen.findByTestId("users-table");
    const rows = userRows();
    expect(rows[1]).toHaveTextContent(fr.users_no_pin);
    expect(rows[0]).not.toHaveTextContent(fr.users_no_pin);
  });

  test("says so when the shop has no other user yet", async () => {
    listed = [];
    app();
    expect(await screen.findByTestId("empty-state")).toHaveTextContent(fr.users_empty);
  });

  test("a fiche without ManageUsers is told rather than shown a table", async () => {
    // What a cashier, or a manager, sees if they type this address by
    // hand: `crate::gates` names `ManageUsers` on `GET /users`, owner
    // only, so the API answers 403 `forbidden` and the screen shows the
    // translated refusal in place of the table (never the table with
    // nothing in it, which would read as "no staff").
    listAnswer = () => json(403, { error: { code: "forbidden", message: "no" } });
    app();
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_forbidden);
    expect(screen.queryByTestId("users-table")).not.toBeInTheDocument();
  });

});

describe("adding a user", () => {
  test("posts the name and the picked role, then lists the new fiche", async () => {
    const user = userEvent.setup();
    app();
    await screen.findByTestId("users-table");
    await user.click(screen.getByRole("button", { name: fr.users_add }));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByLabelText(fr.field_name), "Karim");
    await user.click(within(dialog).getByRole("combobox"));
    await user.click(await screen.findByRole("option", { name: fr.role_manager }));
    await user.type(within(dialog).getByLabelText(fr.field_pin), "2468");
    await user.click(within(dialog).getByRole("button", { name: fr.users_add }));

    await waitFor(() => expect(userRows()).toHaveLength(3));
    const made = posts().find((p) => p.url.endsWith("/users"));
    expect(made?.body).toEqual({ name: "Karim", role: "manager" });
    // The PIN rides in the same dialog and lands on the new fiche's own
    // path, so a fiche is never listed without a way to sign in.
    const pinned = posts().find((p) => p.url.endsWith("/pin"));
    expect(pinned?.url).toMatch(/\/users\/3\/pin$/);
    expect(pinned?.body).toEqual({ pin: "2468" });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(within(userRows()[2] ?? document.body).queryByText(fr.users_no_pin)).not.toBeInTheDocument();
  });

  test("a PIN that is not four to six digits is refused before anything is posted", async () => {
    const user = userEvent.setup();
    app();
    await screen.findByTestId("users-table");
    await user.click(screen.getByRole("button", { name: fr.users_add }));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByLabelText(fr.field_name), "Karim");
    await user.type(within(dialog).getByLabelText(fr.field_pin), "12");
    await user.click(within(dialog).getByRole("button", { name: fr.users_add }));

    expect(await within(dialog).findByRole("alert")).toHaveTextContent(fr.error_pin_shape);
    expect(posts()).toEqual([]);
  });

  test("a blank name is refused before anything is posted", async () => {
    const user = userEvent.setup();
    app();
    await screen.findByTestId("users-table");
    await user.click(screen.getByRole("button", { name: fr.users_add }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: fr.users_add }));

    // Two fields complain at once now (the PIN box is blank too), so the
    // name's refusal is read by its text and not as the one alert.
    expect(await within(dialog).findByText(fr.error_name_required)).toBeInTheDocument();
    expect(posts().filter((p) => p.url.endsWith("/users"))).toEqual([]);
  });

  test("a name the server refuses as already used is shown translated", async () => {
    createAnswer = () => json(409, { error: { code: "conflict", message: "taken" } });
    const user = userEvent.setup();
    app();
    await screen.findByTestId("users-table");
    await user.click(screen.getByRole("button", { name: fr.users_add }));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByLabelText(fr.field_name), "Amel");
    await user.type(within(dialog).getByLabelText(fr.field_pin), "1357");
    await user.click(within(dialog).getByRole("button", { name: fr.users_add }));

    expect(await within(dialog).findByRole("alert")).toHaveTextContent(fr.error_conflict);
  });
});

describe("resetting a PIN", () => {
  test("posts the new PIN alone, on the fiche's own path, and never shows the old one", async () => {
    const user = userEvent.setup();
    app();
    await screen.findByTestId("users-table");
    const row = userRows()[0];
    if (row === undefined) throw new Error("no first row");
    await user.click(within(row).getByRole("button", { name: fr.action_reset_pin }));
    const dialog = await screen.findByRole("dialog");
    // Blank, whether the fiche already had a PIN or not: there is no old
    // one to show, so the box never carries one in.
    expect(within(dialog).getByLabelText(fr.field_pin)).toHaveValue("");
    await user.type(within(dialog).getByLabelText(fr.field_pin), "4321");
    await user.click(within(dialog).getByRole("button", { name: fr.action_reset_pin }));

    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    const sent = posts().find((p) => p.url.endsWith("/pin"));
    expect(sent?.url).toMatch(/\/users\/1\/pin$/);
    expect(sent?.body).toEqual({ pin: "4321" });
  });

  test("a badly shaped PIN is refused by the server and shown translated", async () => {
    pinAnswer = () => json(422, { error: { code: "validation", message: "shape" } });
    const user = userEvent.setup();
    app();
    await screen.findByTestId("users-table");
    const row = userRows()[0];
    if (row === undefined) throw new Error("no first row");
    await user.click(within(row).getByRole("button", { name: fr.action_reset_pin }));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByLabelText(fr.field_pin), "12");
    await user.click(within(dialog).getByRole("button", { name: fr.action_reset_pin }));

    expect(await within(dialog).findByRole("alert")).toHaveTextContent(fr.error_validation);
  });
});

describe("switching a fiche off or back on", () => {
  test("deactivate posts to the fiche's own path and the row picks up 'inactive'", async () => {
    const user = userEvent.setup();
    app();
    await screen.findByTestId("users-table");
    const row = userRows()[1];
    if (row === undefined) throw new Error("no second row");
    await user.click(within(row).getByRole("button", { name: fr.action_deactivate }));

    await waitFor(() =>
      expect(userRows()[1]).toHaveTextContent(fr.users_inactive),
    );
    const sent = posts().find((p) => p.url.endsWith("/deactivate"));
    expect(sent?.url).toMatch(/\/users\/2\/deactivate$/);
  });

  test("a fiche switched off shows 'reactivate' instead, and it posts the other route", async () => {
    listed = [owner, { ...cashier, active: false }];
    const user = userEvent.setup();
    app();
    await screen.findByTestId("users-table");
    const row = userRows()[1];
    if (row === undefined) throw new Error("no second row");
    expect(within(row).queryByRole("button", { name: fr.action_deactivate })).toBeNull();
    await user.click(within(row).getByRole("button", { name: fr.action_reactivate }));

    await waitFor(() => expect(userRows()[1]).not.toHaveTextContent(fr.users_inactive));
    const sent = posts().find((p) => p.url.endsWith("/reactivate"));
    expect(sent?.url).toMatch(/\/users\/2\/reactivate$/);
  });

  // The last-owner and self refusals are never a choice this screen makes:
  // they are the server's, on the row (`services::users::deactivate`), and
  // the screen's only job is to show the refusal it sends back.
  test("the owner's own row refused by the server is reported translated, not silently ignored", async () => {
    deactivateAnswer = () => json(422, { error: { code: "validation", message: "self" } });
    const user = userEvent.setup();
    app();
    await screen.findByTestId("users-table");
    const row = userRows()[0];
    if (row === undefined) throw new Error("no first row");
    await user.click(within(row).getByRole("button", { name: fr.action_deactivate }));

    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_validation);
    // Still active: the server refused it and the row was never optimistic.
    expect(userRows()[0]).not.toHaveTextContent(fr.users_inactive);
  });
});
