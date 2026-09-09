// The backups block on the settings screen: what it lists, what it posts,
// and what it refuses to post. The rules (which copy may be restored, what a
// restore does to the file) are the API crate's tests.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { BackupDto } from "@dzpos/shared";
import { I18nProvider } from "@/i18n";
import fr from "@/i18n/fr.json";
import { BackupsPanel } from "./BackupsPanel";

const newest: BackupDto = {
  name: "dzpos-20260908-093000.sqlite",
  taken_at: "2026-09-08T09:30:00",
  bytes: 2_150_400,
};

const older: BackupDto = {
  name: "dzpos-20260907-093000.sqlite",
  taken_at: "2026-09-07T09:30:00",
  bytes: 143_360,
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

function posts(): string[] {
  return fetchMock.mock.calls
    .filter((call) => {
      const init: unknown = call[1];
      return isInit(init) && init.method === "POST";
    })
    .map((call) => String(call[0]));
}

function mount() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang="fr">
      <QueryClientProvider client={client}>
        <BackupsPanel />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let listed: BackupDto[];
let createAnswer: (() => Response) | null;
let restoreAnswer: (() => Response) | null;

beforeEach(() => {
  listed = [newest, older];
  createAnswer = null;
  restoreAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST" && url.endsWith("/restore")) {
      if (restoreAnswer !== null) return Promise.resolve(restoreAnswer());
      return Promise.resolve(
        json(200, { restored_from: older.name, products: 12, documents: null }),
      );
    }
    if (init?.method === "POST" && url.endsWith("/backups")) {
      if (createAnswer !== null) return Promise.resolve(createAnswer());
      const made: BackupDto = {
        name: "dzpos-20260909-101500.sqlite",
        taken_at: "2026-09-09T10:15:00",
        bytes: 2_150_400,
      };
      listed = [made, ...listed];
      return Promise.resolve(json(201, made));
    }
    if (url.endsWith("/backups")) return Promise.resolve(json(200, listed));
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the list", () => {
  test("shows the newest copy's date and one row per copy, with its size", async () => {
    mount();
    expect(await screen.findByTestId("backups-newest")).toHaveTextContent("2026-09-08 09:30");
    const rows = screen.getAllByTestId("backup-row");
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("2026-09-08 09:30");
    expect(rows[0]).toHaveTextContent(`2,1 ${fr.unit_mb}`);
    expect(rows[1]).toHaveTextContent(`140 ${fr.unit_kb}`);
  });

  test("says so when the shop has no copy yet", async () => {
    listed = [];
    mount();
    expect(await screen.findByText(fr.backups_none)).toBeInTheDocument();
    expect(screen.queryAllByTestId("backup-row")).toHaveLength(0);
  });

  test("a refusal on load is shown translated", async () => {
    fetchMock.mockImplementation(() =>
      Promise.resolve(json(401, { error: { code: "unauthorized", message: "no" } })),
    );
    mount();
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_unauthorized);
  });
});

describe("taking one now", () => {
  test("posts to /backups and says it was taken", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("backups-newest");
    await user.click(screen.getByRole("button", { name: fr.action_backup_now }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(fr.backups_created));
    expect(posts()).toEqual([expect.stringMatching(/\/backups$/)]);
    await waitFor(() => expect(screen.getAllByTestId("backup-row")).toHaveLength(3));
  });

  test("a refusal is shown translated and nothing says it was taken", async () => {
    createAnswer = () => json(500, { error: { code: "storage", message: "disk" } });
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("backups-newest");
    await user.click(screen.getByRole("button", { name: fr.action_backup_now }));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_storage);
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});

describe("restoring one", () => {
  test("asks first, and posts the copy's own name once confirmed", async () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(true);
    const user = userEvent.setup();
    mount();
    const rows = await screen.findAllByTestId("backup-row");
    const second = rows[1];
    if (second === undefined) throw new Error("no second row");
    await user.click(within(second).getByRole("button", { name: fr.action_restore }));

    expect(confirm).toHaveBeenCalledWith(fr.backups_confirm_restore);
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(fr.backups_restored));
    expect(posts()).toEqual([
      expect.stringMatching(/\/backups\/dzpos-20260907-093000\.sqlite\/restore$/),
    ]);
  });

  test("a cancelled confirmation posts nothing", async () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    const user = userEvent.setup();
    mount();
    const rows = await screen.findAllByTestId("backup-row");
    const first = rows[0];
    if (first === undefined) throw new Error("no first row");
    await user.click(within(first).getByRole("button", { name: fr.action_restore }));

    expect(confirm).toHaveBeenCalled();
    expect(posts()).toEqual([]);
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  test("a copy the server refuses is reported translated", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    restoreAnswer = () => json(422, { error: { code: "validation", message: "backup" } });
    const user = userEvent.setup();
    mount();
    const rows = await screen.findAllByTestId("backup-row");
    const first = rows[0];
    if (first === undefined) throw new Error("no first row");
    await user.click(within(first).getByRole("button", { name: fr.action_restore }));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_validation);
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});
