// A refusal is not a disconnection. The old till queued both, so a sale the
// server would never accept was resent for ever and a cashier whose session
// had idled out kept handing goods over with nothing recorded.
//
// Each row below names what the server actually answers, from
// crates/api/src/error.rs and the gates, and asserts the whole answer. Six
// of these read only `.kind` until 2026-09-20, which said nothing about the
// sentence: swapping `error_sign_in_again` for `error_pair_again` left all
// twelve green while a cashier whose session had idled out was sent to find
// a manager for a fresh QR. The tests lens found it.

import { describe, expect, it } from "vitest";

import { outcomeOf, type ApiError, type Outcome } from "../../lib/outcome";

/** The English every row hands in, so that comparing the whole answer is
 *  also the proof that the server's own prose never reaches a screen. It is
 *  a Rust `Display` string and it is English whatever the cashier reads. */
const SERVERS_OWN_WORDS = "tendered is invalid: less than the amount to pay";

const RANG: [string, number][] = [
  ["the sale was written", 201],
  // The server answers 200 with the stored paper, not a second sale.
  ["the retry key replayed the original", 200],
];

const REFUSED: [string, number, ApiError, Outcome][] = [
  // The person's token and nothing else, so a PIN fixes it.
  [
    "the session died",
    401,
    { code: "session_required", message: SERVERS_OWN_WORDS },
    { kind: "sign-in-again", say: { key: "error_sign_in_again" } },
  ],
  // The phone itself. Signing in again would not help; a manager has to
  // hand over a new QR.
  [
    "the device was revoked",
    401,
    { code: "device_refused", message: SERVERS_OWN_WORDS },
    { kind: "pair-again", say: { key: "error_pair_again" } },
  ],
  // The secret just typed, and it clears nothing. This one used to read as
  // the device gate, and on 2026-09-16 one mistyped digit sent a phone back
  // to the QR screen.
  [
    "the PIN was wrong",
    401,
    { code: "auth_refused", message: SERVERS_OWN_WORDS },
    { kind: "refused", say: { key: "error_wrong_secret" } },
  ],
  // A role. Neither re-pairing nor signing in again changes it.
  [
    "the role does not reach it",
    403,
    { code: "forbidden", message: SERVERS_OWN_WORDS },
    { kind: "refused", say: { key: "error_not_allowed" } },
  ],
  // The case the old till queued for ever: tendered under what is owed,
  // resent with the same dead credentials to earn the same refusal.
  [
    "the request did not validate",
    422,
    { code: "validation", message: SERVERS_OWN_WORDS },
    { kind: "refused", say: { key: "error_validation" } },
  ],
  // The refusal this milestone is about. Until 2026-09-20 a customer past
  // their credit limit and a template that would not render read the same
  // on the counter, because the sentence came from the status.
  [
    "the customer is past their credit limit",
    422,
    { code: "credit_limit", message: SERVERS_OWN_WORDS },
    { kind: "refused", say: { key: "error_credit_limit" } },
  ],
  // A key the server already knows. The core answers 409 two ways on the
  // same code and field: a key whose winner is readable, worth sending
  // again, and a key reused on a different basket, which no retry will fix.
  // Only the prose tells them apart, and queueing on a message string is
  // how the bug this module exists to kill got written. Both are shown, and
  // the cashier rings again with a fresh key.
  [
    "the idempotency key was already spent",
    409,
    { code: "conflict", message: SERVERS_OWN_WORDS },
    { kind: "refused", say: { key: "error_sale_already_rung" } },
  ],
  // And a body with nothing in it says the same as one with everything in
  // it, because nothing in the answer was ever where the sentence came
  // from. Written out rather than compared with the row above, which would
  // have been the function agreeing with itself.
  [
    "the body carried nothing",
    422,
    null,
    { kind: "refused", say: { key: "error_refused", vars: { status: 422 } } },
  ],
  // A code from a build of the server newer than this build of the phone.
  // It falls back to the status rather than to a general sentence: a number
  // a cashier can read out is worth more than prose that hides which
  // refusal happened.
  [
    "the code is one this build has never heard of",
    422,
    { code: "a_code_from_a_later_server", message: SERVERS_OWN_WORDS },
    { kind: "refused", say: { key: "error_refused", vars: { status: 422 } } },
  ],
];

const QUEUED: [string, number | null, ApiError][] = [
  // Airplane mode, desktop asleep, Wi-Fi gone. This is what the queue is for.
  ["fetch itself threw", null, null],
  ["the server could not answer at all", 500, { code: "internal", message: "" }],
  ["the server was not there to answer", 503, null],
];

describe("a call that reached a server", () => {
  it.each(RANG)("rang, when %s", (_case, status) => {
    expect(outcomeOf(status, null)).toEqual({ kind: "rang" });
  });
});

describe("a call the server refused", () => {
  it.each(REFUSED)("%s", (_case, status, error, expected) => {
    expect(outcomeOf(status, error)).toEqual(expected);
  });
});

describe("a call that got no answer", () => {
  it.each(QUEUED)("queues when %s", (_case, status, error) => {
    expect(outcomeOf(status, error)).toEqual({ kind: "queue" });
  });
});
