// The backups block on the settings screen: what it lists, what it posts,
// and what it refuses to post. The rules (which copy may be restored, what a
// restore does to the file) are the API crate's tests.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { BackupDto, BackupsDto } from "@dzpos/shared";
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

const safety: BackupDto = {
  name: "dzpos.db.before-restore-20260909-101500-250.sqlite",
  taken_at: "2026-09-09T10:15:00",
  bytes: 2_150_400,
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

/** The copies the panel lists, without the table's header row. */
function backupRows(): HTMLElement[] {
  return within(screen.getByTestId("backups-table")).queryAllByRole("row").slice(1);
}

/** The copies kept before a restore, same shape, their own table. */
function safetyRows(): HTMLElement[] {
  return within(screen.getByTestId("safety-copies-table")).queryAllByRole("row").slice(1);
}

/** Presses restore on a row and answers the dialog it opens. */
async function askToRestore(
  user: ReturnType<typeof userEvent.setup>,
  row: HTMLElement,
  answer: "confirm" | "cancel",
): Promise<void> {
  await user.click(within(row).getByRole("button", { name: fr.action_restore }));
  const dialog = await screen.findByRole("dialog");
  expect(dialog).toHaveTextContent(fr.backups_confirm_restore);
  const button = answer === "confirm" ? fr.backups_restore_confirm : fr.action_cancel;
  await user.click(within(dialog).getByRole("button", { name: button }));
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
let listed: BackupsDto;
let createAnswer: (() => Response) | null;
let restoreAnswer: (() => Response) | null;

beforeEach(() => {
  listed = { backups: [newest, older], safety_copies: [] };
  createAnswer = null;
  restoreAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST" && url.endsWith("/restore")) {
      if (restoreAnswer !== null) return Promise.resolve(restoreAnswer());
      listed = { ...listed, safety_copies: [safety] };
      return Promise.resolve(
        json(200, {
          restored_from: older.name,
          safety_copy: safety.name,
          products: 12,
          documents: null,
        }),
      );
    }
    if (init?.method === "POST" && url.endsWith("/backups")) {
      if (createAnswer !== null) return Promise.resolve(createAnswer());
      const made: BackupDto = {
        name: "dzpos-20260909-101500.sqlite",
        taken_at: "2026-09-09T10:15:00",
        bytes: 2_150_400,
      };
      listed = { ...listed, backups: [made, ...listed.backups] };
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
    const rows = backupRows();
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent("2026-09-08 09:30");
    expect(rows[0]).toHaveTextContent(`2,1 ${fr.unit_mb}`);
    expect(rows[1]).toHaveTextContent(`140 ${fr.unit_kb}`);
  });

  test("keeps the copies taken before a restore under their own heading", async () => {
    listed = { backups: [newest], safety_copies: [safety] };
    mount();
    expect(
      await screen.findByRole("heading", { name: fr.settings_safety_copies }),
    ).toBeInTheDocument();
    const kept = safetyRows();
    expect(kept).toHaveLength(1);
    expect(kept[0]).toHaveTextContent("2026-09-09 10:15");
    // They are shown, never offered: restoring one is not one more click.
    expect(within(kept[0] ?? document.body).queryByRole("button")).toBeNull();
    expect(backupRows()).toHaveLength(1);
  });

  test("shows no safety heading before anything has been restored", async () => {
    mount();
    await screen.findByTestId("backups-newest");
    expect(screen.queryByRole("heading", { name: fr.settings_safety_copies })).toBeNull();
  });

  test("says so when the shop has no copy yet", async () => {
    listed = { backups: [], safety_copies: [] };
    mount();
    // Twice: the summary line above the list, and the empty state in its
    // place, which is what a shop reads first on the first morning.
    expect(await screen.findAllByText(fr.backups_none)).toHaveLength(2);
    expect(backupRows()).toHaveLength(0);
    expect(screen.getByTestId("empty-state")).toBeInTheDocument();
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
    await waitFor(() => expect(backupRows()).toHaveLength(3));
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
  test("asks in a dialog of its own, and posts the copy's own name once confirmed", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("backups-newest");
    const second = backupRows()[1];
    if (second === undefined) throw new Error("no second row");
    // The dialog names the copy the press was about, not just any copy: two
    // rows an hour apart read the same until the date is repeated.
    await user.click(within(second).getByRole("button", { name: fr.action_restore }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent(fr.backups_confirm_restore);
    expect(dialog).toHaveTextContent("2026-09-07 09:30");
    await user.click(within(dialog).getByRole("button", { name: fr.backups_restore_confirm }));

    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(fr.backups_restored));
    expect(posts()).toEqual([
      expect.stringMatching(/\/backups\/dzpos-20260907-093000\.sqlite\/restore$/),
    ]);
  });

  test("a dialog closed on the way out posts nothing", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("backups-newest");
    const first = backupRows()[0];
    if (first === undefined) throw new Error("no first row");
    await askToRestore(user, first, "cancel");

    expect(posts()).toEqual([]);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  test("a copy the server refuses is reported translated", async () => {
    restoreAnswer = () => json(422, { error: { code: "validation", message: "backup" } });
    await restoreTheFirstCopy();
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_validation);
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  // The two ways a restore can leave the shop file closed. They read the
  // same to the server and not at all the same to the owner: after one of
  // them the copy is the shop file, after the other nothing was replaced.
  // Falling back to "error_unknown" would hide which one happened.
  test("a shop file the app closed and could not reopen asks for a relaunch", async () => {
    restoreAnswer = () => json(500, { error: { code: "restart_needed", message: "closed" } });
    await restoreTheFirstCopy();
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_restart_needed);
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  test("a restore that did not happen says that too, not only the relaunch", async () => {
    restoreAnswer = () =>
      json(500, { error: { code: "restore_failed_restart_needed", message: "closed" } });
    await restoreTheFirstCopy();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      fr.error_restore_failed_restart_needed,
    );
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  async function restoreTheFirstCopy() {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("backups-newest");
    const first = backupRows()[0];
    if (first === undefined) throw new Error("no first row");
    await askToRestore(user, first, "confirm");
  }
});
