// Sign-in and sign-out are the two seams where the query cache must not
// carry the wrong person's data forward. `establish()` and `forgetSession()`
// both call `queryClient.clear()`, not `invalidateQueries()`: an invalidated
// query still renders its last answer while a refetch nobody may even be
// listening for is in flight, and under `staleTime: 30_000` (`main.tsx`) a
// query that was already fresh does not refetch on its own for half a
// minute regardless. Either way a cashier signing in right after an owner,
// on the same machine, would be served the owner's cached rows — the audit
// log screen (the first screen gated on a permission) is where that stops
// being hypothetical. So this seeds a cache entry as one person, signs in
// (or out) as another, and reads the entry back: gone, not merely stale.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { ReactNode } from "react";

import type { MeDto } from "@dzpos/shared";

import { SessionProvider, useSession } from "./session";

const OWNER: MeDto = { user_id: 1, name: "Propriétaire", role: "owner", permissions: [] };
const CASHIER: MeDto = { user_id: 2, name: "Caissier", role: "cashier", permissions: [] };

const json = (status: number, body: unknown): Response =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

const notFound = () => json(404, { error: { code: "not_found", message: "no" } });

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  // Nobody signed in yet: the boot effect's `GET /auth/me` is refused, the
  // way a desktop webview with no cookie sees it, so every test starts from
  // the same "signed-out" ground.
  fetchMock = vi.fn((input: unknown) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) {
      return Promise.resolve(json(401, { error: { code: "session_required", message: "no" } }));
    }
    return Promise.resolve(notFound());
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

function wrapper(client: QueryClient) {
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={client}>
        <SessionProvider>{children}</SessionProvider>
      </QueryClientProvider>
    );
  };
}

describe("the query cache does not survive a change of who is signed in", () => {
  test("signing in clears it — the next person is not served the last person's cache", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result } = renderHook(() => useSession(), { wrapper: wrapper(client) });
    await waitFor(() => expect(result.current.status).toBe("signed-out"));

    // A row only the owner should ever have been served — the shape of the
    // audit log's own query, not just any key.
    client.setQueryData(["audit-log"], [{ actor: "Propriétaire", action: "deleted a product" }]);
    expect(client.getQueryData(["audit-log"])).toBeDefined();

    fetchMock.mockImplementation((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/auth/login")) {
        return Promise.resolve(json(200, { me: CASHIER, token: "cashier-token", idle_minutes: 15 }));
      }
      return Promise.resolve(notFound());
    });

    await act(async () => {
      await result.current.signInWithPin(2, "1379");
    });
    await waitFor(() => expect(result.current.status).toBe("signed-in"));
    expect(result.current.me).toEqual(CASHIER);

    // Gone, not merely stale-and-still-rendering: `invalidateQueries` would
    // have left this exact array in place (marked for a refetch this test
    // has no observer to even trigger), and the cashier's screen would have
    // painted the owner's audit rows for however long the round trip to
    // ask again takes.
    expect(client.getQueryData(["audit-log"])).toBeUndefined();
  });

  test("unlocking clears it too — the same call as a fresh sign-in, on purpose", async () => {
    // `establish()` cannot tell an unlock apart from a different person
    // sitting down; a session resumed from a cookie carries no signal for
    // it either. So the clear happens every time, the cost of a wasted
    // refetch on an unlock being far smaller than getting this wrong.
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result } = renderHook(() => useSession(), { wrapper: wrapper(client) });
    await waitFor(() => expect(result.current.status).toBe("signed-out"));

    fetchMock.mockImplementation((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/auth/login")) {
        return Promise.resolve(json(200, { me: OWNER, token: "owner-token", idle_minutes: 15 }));
      }
      return Promise.resolve(notFound());
    });
    await act(async () => {
      await result.current.signInWithPassword("Propriétaire", "developpement");
    });
    await waitFor(() => expect(result.current.status).toBe("signed-in"));

    client.setQueryData(["audit-log"], [{ actor: "Propriétaire", action: "deleted a product" }]);
    expect(client.getQueryData(["audit-log"])).toBeDefined();

    // The lock screen's own unlock is `signInWithPassword` again, the same
    // owner this time — still cleared.
    await act(async () => {
      await result.current.signInWithPassword("Propriétaire", "developpement");
    });
    expect(client.getQueryData(["audit-log"])).toBeUndefined();
  });

  test("signing out clears it", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { result } = renderHook(() => useSession(), { wrapper: wrapper(client) });
    await waitFor(() => expect(result.current.status).toBe("signed-out"));

    fetchMock.mockImplementation((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/auth/login")) {
        return Promise.resolve(json(200, { me: OWNER, token: "owner-token", idle_minutes: 15 }));
      }
      if (url.endsWith("/auth/logout")) return Promise.resolve(json(200, {}));
      return Promise.resolve(notFound());
    });
    await act(async () => {
      await result.current.signInWithPassword("Propriétaire", "developpement");
    });
    await waitFor(() => expect(result.current.status).toBe("signed-in"));

    client.setQueryData(["audit-log"], [{ actor: "Propriétaire", action: "deleted a product" }]);
    expect(client.getQueryData(["audit-log"])).toBeDefined();

    await act(async () => {
      await result.current.signOut();
    });
    await waitFor(() => expect(result.current.status).toBe("signed-out"));
    expect(client.getQueryData(["audit-log"])).toBeUndefined();
  });
});
