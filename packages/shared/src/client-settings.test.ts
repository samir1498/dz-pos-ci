// The settings, stock-recount and backup calls: what each one sends and what
// it refuses to accept back.
//
// Apart from client.test.ts because that file is the till's half of the same
// client, and the two have nothing to say to each other. They share the
// `stub` helper below rather than a module: a stub is four lines, and a
// helper shared between two suites is a helper that can be edited once to
// make both green.

import { describe, expect, test } from "vitest";
import { ApiError, createClient } from "./client";
import type { BackupDto } from "./generated/BackupDto";
import type { BackupsDto } from "./generated/BackupsDto";
import type { LastStockRecountDto } from "./generated/LastStockRecountDto";
import type { RestoreDto } from "./generated/RestoreDto";
import type { SettingsDto } from "./generated/SettingsDto";
import type { StockDriftDto } from "./generated/StockDriftDto";
import type { StockRecountDto } from "./generated/StockRecountDto";
import type { StoreDto } from "./generated/StoreDto";

function stub(status: number, body: unknown): typeof fetch {
  return async () =>
    new Response(body === undefined ? "" : JSON.stringify(body), {
      status,
      headers: { "content-type": "application/json" },
    });
}

describe("settings", () => {
  const store: StoreDto = {
    name: "Superette El Baraka",
    rc: "16/00-1234567 B 20",
    nif: "000016001234567",
    nis: null,
    ai: null,
    address: "12 rue Didouche Mourad, Alger",
    phone: "0555 12 34 56",
  };
  const settings: SettingsDto = {
    store,
    regime: { regime: "reel", valid_from: "2026-01-01" },
    regime_planned: null,
    theme: null,
    facture_layout: "standard",

    facture_layouts: ["standard", "compact"],
    print_lang: null,
    discount_threshold_bps: 0,
  };

  test("reads the settings page and keeps a planned change", async () => {
    const planned: SettingsDto = {
      ...settings,
      regime_planned: { regime: "ifu", valid_from: "2027-01-01" },
    };
    const api = createClient("http://127.0.0.1:4317", stub(200, planned));
    await expect(api.getSettings()).resolves.toEqual(planned);
  });

  test("a settings answer with a régime the app does not know is refused", async () => {
    for (const bad of [
      { ...settings, regime: { regime: "forfait", valid_from: "2026-01-01" } },
      { ...settings, regime: { regime: "reel", valid_from: "2026-1-1" } },
      { ...settings, regime_planned: { regime: "ifu" } },
      { ...settings, store: { ...store, name: null } },
      { ...settings, store: { ...store, rc: 12 } },
    ]) {
      const api = createClient("http://127.0.0.1:4317", stub(200, bad));
      await expect(api.getSettings()).rejects.toMatchObject({ code: "bad_response" });
    }
  });

  test("updating the store puts the whole block and returns it", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(store), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.updateStore(store)).resolves.toEqual(store);
    expect(calls).toHaveLength(1);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/settings/store");
    expect(calls[0]?.init?.method).toBe("PUT");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual(store);
  });

  test("choosing a theme puts it and returns the whole page", async () => {
    const chosen: SettingsDto = { ...settings, theme: "observe-dark" };
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(chosen), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.setTheme("observe-dark")).resolves.toEqual(chosen);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/settings/theme");
    expect(calls[0]?.init?.method).toBe("PUT");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual({ theme: "observe-dark" });
  });

  /** null is a choice: it puts the shop back on the machine's preference. */
  test("clearing the theme sends null rather than leaving the field out", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(settings), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.setTheme(null)).resolves.toEqual(settings);
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual({ theme: null });
  });

  test("a settings answer naming a theme the app has no block for is refused", async () => {
    const api = createClient("http://127.0.0.1:4317", stub(200, { ...settings, theme: "midnight" }));
    await expect(api.getSettings()).rejects.toMatchObject({ code: "bad_response" });
  });

  test("choosing a print language puts it and returns the whole page", async () => {
    const chosen: SettingsDto = { ...settings, print_lang: "ar" };
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(chosen), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.setPrintLang("ar")).resolves.toEqual(chosen);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/settings/print-lang");
    expect(calls[0]?.init?.method).toBe("PUT");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual({ print_lang: "ar" });
  });

  /** null is a choice: it puts every fiscal paper back on the till's own
   *  language. */
  test("clearing the print language sends null rather than leaving the field out", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(settings), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.setPrintLang(null)).resolves.toEqual(settings);
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual({ print_lang: null });
  });

  test("a régime change posts the day and returns the whole page", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(settings), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.changeRegime({ regime: "ifu", valid_from: "2027-01-01" })).resolves.toEqual(
      settings,
    );
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/settings/regime");
    expect(calls[0]?.init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual({
      regime: "ifu",
      valid_from: "2027-01-01",
    });
  });
});

describe("the stock recount", () => {
  const drift: StockDriftDto = {
    product_id: 7,
    name: "Sucre 1kg",
    cached_milli: 99_000,
    ledger_milli: 24_000,
    difference_milli: -75_000,
  };

  const last: LastStockRecountDto = { last_run_day: "2026-09-10", drifts: [drift] };
  const run: StockRecountDto = { day: "2026-09-10", products_checked: 42, drifts: [drift] };

  test("reads the last run from /stock/recount", async () => {
    const api = createClient("http://127.0.0.1:4317", stub(200, last));
    await expect(api.lastStockRecount()).resolves.toEqual(last);
  });

  test("running one posts to the same address and returns the report", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(run), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.recountStock()).resolves.toEqual(run);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/stock/recount");
    expect(calls[0]?.init?.method).toBe("POST");
  });

  test("a report of the wrong shape is refused, never handed to the panel", async () => {
    for (const bad of [
      { ...run, drifts: [{ ...drift, ledger_milli: 24_000.5 }] },
      { ...run, day: "10/09/2026" },
      { ...run, products_checked: 1.5 },
      { day: run.day, drifts: [] },
    ]) {
      const api = createClient("http://x", stub(200, bad));
      await expect(api.recountStock()).rejects.toMatchObject({ code: "bad_response" });
    }
  });
});

describe("backups", () => {
  const backup: BackupDto = {
    name: "dzpos-20260908-093000.sqlite",
    taken_at: "2026-09-08T09:30:00",
    bytes: 143_360,
  };

  const all: BackupsDto = { backups: [backup], safety_copies: [], upgrade_copies: [backup] };

  test("lists the copies the server reports, the three kinds apart", async () => {
    const api = createClient("http://127.0.0.1:4317", stub(200, all));
    await expect(api.listBackups()).resolves.toEqual(all);
  });

  test("a copy of the wrong shape is refused, never handed to the UI", async () => {
    for (const bad of [
      { ...all, backups: [{ name: backup.name, taken_at: backup.taken_at }] },
      { ...all, backups: [{ ...backup, bytes: 1.5 }] },
      { backups: [backup] },
      { backups: [backup], safety_copies: [] },
      { ...all, safety_copies: {} },
      [backup],
    ]) {
      const api = createClient("http://x", stub(200, bad));
      await expect(api.listBackups()).rejects.toMatchObject({ code: "bad_response" });
    }
  });

  test("creating one posts to /backups and returns what was written", async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(backup), {
        status: 201,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.createBackup()).resolves.toEqual(backup);
    expect(calls[0]?.url).toBe("http://127.0.0.1:4317/backups");
    expect(calls[0]?.init?.method).toBe("POST");
  });

  test("restoring names the copy in the path and returns the counts", async () => {
    const answer: RestoreDto = {
      restored_from: backup.name,
      safety_copy: "dzpos.db.before-restore-20260909-101500-250.sqlite",
      products: 12,
      documents: null,
    };
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchStub: typeof fetch = async (input, init) => {
      calls.push({ url: String(input), init });
      return new Response(JSON.stringify(answer), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.restoreBackup(backup.name)).resolves.toEqual(answer);
    expect(calls[0]?.url).toBe(
      "http://127.0.0.1:4317/backups/dzpos-20260908-093000.sqlite/restore",
    );
    expect(calls[0]?.init?.method).toBe("POST");
  });

  test("a name the server never wrote is encoded, never spliced into the path", async () => {
    const calls: string[] = [];
    const fetchStub: typeof fetch = async (input) => {
      calls.push(String(input));
      return new Response(JSON.stringify({ error: { code: "validation", message: "no" } }), {
        status: 422,
        headers: { "content-type": "application/json" },
      });
    };
    const api = createClient("http://127.0.0.1:4317", fetchStub);
    await expect(api.restoreBackup("../../etc/passwd")).rejects.toMatchObject({
      code: "validation",
    });
    expect(calls[0]).toBe("http://127.0.0.1:4317/backups/..%2F..%2Fetc%2Fpasswd/restore");
  });

  test("a restore answer of the wrong shape is refused", async () => {
    const api = createClient("http://x", stub(200, { restored_from: backup.name }));
    await expect(api.restoreBackup(backup.name)).rejects.toMatchObject({
      code: "bad_response",
    });
  });
});
