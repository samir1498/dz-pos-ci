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
  /** The claim the sign-in screen acts on. `pair-again` and not the status
   *  or the code, so the screen does not sort the answer a second time and
   *  cannot disagree with `outcomeOf` about what it means. */
  it("says the pairing is gone when the device gate refused it", async () => {
    const refusal = await refusalFrom(401, { error: { code: "device_refused", message: "revoked" } });
    expect(refusal.outcome.kind).toBe("pair-again");
    expect(refusal.outcome.say.key).toBe("error_pair_again");
  });

  /** And the other way, which is the expensive half: a phone unpaired over
   *  a permission would send a cashier to a manager for a QR they did not
   *  need, in the middle of a queue. */
  it("does not say the pairing is gone for a role or a mistyped PIN", async () => {
    const role = await refusalFrom(403, { error: { code: "forbidden", message: "no" } });
    expect(role.outcome.kind).toBe("refused");

    const pin = await refusalFrom(401, { error: { code: "auth_refused", message: "no" } });
    expect(pin.outcome.kind).toBe("refused");
  });
});
