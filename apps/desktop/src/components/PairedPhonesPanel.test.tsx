// The paired phones block. What the routes refuse is the Rust suite's
// business (`crates/api/tests/pairing_api.rs` already proves a cashier is
// forbidden on all three); this checks the three things only the screen can
// get wrong.
//
// The countdown test is the one that matters. A pairing token lives sixty
// seconds and is single use, so a QR left on screen after it lapsed is a QR
// an owner shows a cashier who then cannot pair and has no idea why.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { I18nProvider } from "@/i18n";
import fr from "@/i18n/fr.json";
import { PairedPhonesPanel } from "./PairedPhonesPanel";

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

const TOKEN = "a".repeat(64);

function mount() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang="fr">
      <QueryClientProvider client={client}>
        <PairedPhonesPanel />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let devices: unknown[];
let revoked: number[];
/** What the server says the token has left. The panel counts this down and
 *  never assumes sixty, so a one-second token is the honest way to watch it
 *  lapse without faking a clock that testing-library also reads. */
let ttl: number;

beforeEach(() => {
  devices = [
    { id: 7, name: "Caisse 2", created_at: "2026-09-16 09:00:00", revoked_at: null, created_by: 1 },
  ];
  revoked = [];
  ttl = 60;
  fetchMock = vi.fn((input: unknown, init: unknown) => {
    const url = String(input);
    const method =
      typeof init === "object" && init !== null ? String(Reflect.get(init, "method") ?? "GET") : "GET";
    if (url.includes("/pairing/qr") && method === "POST") {
      return Promise.resolve(json(200, { pairing_token: TOKEN, expires_in_seconds: ttl }));
    }
    const revoke = /\/pairing\/devices\/(\d+)\/revoke$/.exec(url);
    if (revoke !== null && method === "POST") {
      const id = Number(revoke[1]);
      revoked.push(id);
      devices = devices.map((row) =>
        typeof row === "object" && row !== null && Reflect.get(row, "id") === id
          ? { ...row, revoked_at: "2026-09-16 10:00:00" }
          : row,
      );
      return Promise.resolve(json(200, devices[0]));
    }
    if (url.includes("/pairing/devices")) return Promise.resolve(json(200, devices));
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the paired phones block", () => {
  test("shows a QR carrying the token the server minted, and the token in full", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByRole("button", { name: fr.phones_show_qr }));
    // react-qr-code draws an <svg>; what is asserted is the token, because a
    // QR of the wrong string is a QR that scans and then fails to pair.
    expect(await screen.findByText(TOKEN)).toBeInTheDocument();
  });

  test("the QR disappears when the server's seconds are up", async () => {
    // One second rather than sixty, and a real clock rather than a faked
    // one: testing-library's own waiting runs on the same timers, so faking
    // them makes every `find` hang instead of the token lapsing.
    ttl = 1;
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByRole("button", { name: fr.phones_show_qr }));
    expect(await screen.findByText(TOKEN)).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByText(TOKEN)).not.toBeInTheDocument(), {
      timeout: 4000,
    });
  });

  test("revoking asks first, and only then calls the route", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole("button", { name: /Révoquer/ }));
    // The dialog is up and nothing has been sent yet: a phone revoked by a
    // mis-click is a cashier who cannot sell and a manager who has to be
    // found.
    expect(await screen.findByText(fr.phones_revoke_title)).toBeInTheDocument();
    expect(revoked).toEqual([]);
    await user.click(screen.getByRole("button", { name: fr.phones_revoke_confirm }));
    await waitFor(() => expect(revoked).toEqual([7]));
  });
});
