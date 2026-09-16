// A refusal is not a disconnection. The old till queued both, so a sale the
// server would never accept was resent for ever and a cashier whose session
// had idled out kept handing goods over with nothing recorded.
//
// Each case below names what the server actually answers, from
// crates/api/src/error.rs and the gates.

import { describe, expect, it } from "vitest";

import { outcomeOf } from "./outcome";

describe("a call that reached a server", () => {
  it("rang, when the sale was written", () => {
    expect(outcomeOf(201, null).kind).toBe("rang");
  });

  it("rang, when the retry key replayed the original", () => {
    // The server answers 200 with the stored paper, not a second sale.
    expect(outcomeOf(200, null).kind).toBe("rang");
  });
});

describe("a call the server refused", () => {
  it("sends the person back to sign in when their session died", () => {
    const outcome = outcomeOf(401, { code: "session_required", message: "sign in" });
    expect(outcome.kind).toBe("sign-in-again");
  });

  it("sends the phone back to pairing when the device was revoked", () => {
    // `auth_refused` is the device gate, not the person: signing in again
    // would not help, a manager has to hand over a new QR.
    const outcome = outcomeOf(401, { code: "auth_refused", message: "revoked" });
    expect(outcome.kind).toBe("pair-again");
  });

  it("shows a role refusal without signing anyone out", () => {
    const outcome = outcomeOf(403, { code: "forbidden", message: "not allowed" });
    expect(outcome.kind).toBe("refused");
  });

  it("shows a validation refusal and does not queue it", () => {
    // The case the old screen queued for ever: tendered under what is owed.
    const outcome = outcomeOf(422, {
      code: "validation",
      message: "tendered is invalid: less than the amount to pay",
    });
    expect(outcome.kind).toBe("refused");
    expect(outcome.kind).not.toBe("queue");
  });

  it("shows a conflict rather than resending a key the server already knows", () => {
    const outcome = outcomeOf(409, { code: "conflict", message: "already rang a different sale" });
    expect(outcome.kind).toBe("refused");
  });

  it("carries the server's own words when it has some", () => {
    const outcome = outcomeOf(422, { code: "validation", message: "less than the amount to pay" });
    expect(outcome.kind === "refused" && outcome.message).toBe("less than the amount to pay");
  });

  it("still says something when the body carried no message", () => {
    const outcome = outcomeOf(422, null);
    expect(outcome.kind === "refused" && outcome.message.length > 0).toBe(true);
  });
});

describe("a call that got no answer", () => {
  it("queues when fetch itself threw", () => {
    // Airplane mode, desktop asleep, Wi-Fi gone. This is what the queue is for.
    expect(outcomeOf(null, null).kind).toBe("queue");
  });

  it("queues when the server could not answer at all", () => {
    expect(outcomeOf(500, { code: "internal", message: "" }).kind).toBe("queue");
    expect(outcomeOf(503, null).kind).toBe("queue");
  });
});
