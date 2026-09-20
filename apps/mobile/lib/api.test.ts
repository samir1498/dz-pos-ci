// What a read that was refused tells the screen that asked for it.
//
// The sign-in screen fills itself from `GET /auth/staff`, which sits behind
// the device gate (`crates/api/src/router.rs`, the `phone_auth` router). So
// a phone a manager has just unpaired learns it on that read, through
// TanStack Query, and never on the login call where the same answer was
// already handled. Before 2026-09-20 the screen said it could not read the
// staff list, the retry button earned the same 401 for ever, and nobody was
// sent to find a manager for a fresh QR.
//
// The screen cannot be rendered here: `vitest.config.ts` collects `lib/`
// only and there is no test renderer. What is held instead is the thing the
// screen reads, which is that a thrown refusal carries the outcome whole
// rather than a status the screen would have to sort a second time.

import { describe, expect, it, vi } from "vitest";

vi.mock("expo-constants", () => ({ default: { expoConfig: { extra: {} } } }));

import { ApiRefusal, get } from "./api";

function answers(status: number, body: unknown) {
  vi.stubGlobal("fetch", () =>
    Promise.resolve({
      status,
      text: () => Promise.resolve(JSON.stringify(body)),
      json: () => Promise.resolve(body),
    }),
  );
}

async function refusalFrom(status: number, body: unknown): Promise<ApiRefusal> {
  answers(status, body);
  try {
    await get("/auth/staff", { deviceToken: "d".repeat(64) });
  } catch (thrown) {
    if (thrown instanceof ApiRefusal) return thrown;
    throw thrown;
  } finally {
    vi.unstubAllGlobals();
  }
  throw new Error("the read was not refused");
}

describe("a read the server refused", () => {
  /** What is held here and nowhere else: the answer `outcomeOf` reached
   *  arrives on the thrown refusal whole, so the sign-in screen reads a
   *  kind rather than sorting a status and a code a second time and
   *  disagreeing. Which status maps to which outcome is `outcome.test.ts`'s
   *  table; this is only the wire carrying it. */
  it("carries the outcome whole to whoever catches it", async () => {
    const refusal = await refusalFrom(401, { error: { code: "device_refused", message: "revoked" } });
    expect(refusal.outcome).toEqual({ kind: "pair-again", say: { key: "error_pair_again" } });
    expect(refusal.code).toBe("device_refused");
  });

  /** A refusal body the core did not write. A proxy, a captive portal or
   *  a gateway answers its own page on the way out, and the phone reads
   *  whatever that is for `.error.code`. Read as an object it throws on
   *  the way into the screen; read loosely it finds a "code" that belongs
   *  to somebody else's vocabulary, or is not a string at all. None of
   *  those may end in "ask a manager for a new QR", so the answer has to
   *  stay the plain one a 401 gets when nothing names a reason.
   *
   *  The row that discriminates is the numeric code: a body whose `error`
   *  is a string or a page reaches the same answer through a blind read
   *  too, because a string has no `.code` either. */
  it.each([
    ["a page from something in front of the server", "<html>Sign in to the Wi-Fi</html>"],
    ["an error that is a word, not a fiche", { error: "device_refused" }],
    ["a code that is not a string", { error: { code: 42 } }],
  ])("does not read a code out of %s", async (_what, body) => {
    const refusal = await refusalFrom(401, body);
    expect(refusal.outcome).toEqual({ kind: "sign-in-again", say: { key: "error_sign_in_again" } });
    expect(refusal.code).toBeNull();
  });
});
