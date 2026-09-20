// What the phone should do with an answer it did not expect.
//
// The till used to treat every non-2xx as "the network dropped": it threw,
// the catch enqueued the sale, and the retry resent the same body with the
// same dead credentials to earn the same refusal for ever. A 15-minute
// idle-out (`DEFAULT_SESSION_IDLE_MINUTES`) was enough to make a cashier
// hand goods over with no document, no number, no stock movement and no
// cash recorded. Adversarial review of M6+M7, 2026-09-16.
//
// A refusal is not a disconnection. Only a call that never reached a
// server, or one the server could not answer, is worth queueing.

import type { Key, Vars } from "@dzpos/shared";

/** What the cashier is told, in a form the screen can put in their own
 *  language.
 *
 *  A key, not a sentence. The server's `message` is a Rust `Display`
 *  string (`crates/api/src/error.rs`, `fn message`) and it is English,
 *  always: the till used to show it, so every refusal on an Arabic counter
 *  came back in English. The desktop settled this shape at
 *  `apps/desktop/src/lib/fields.tsx:63`, where the error's code picks a key
 *  and the server's own prose never reaches a screen. */
export type Say = { key: Key; vars?: Vars };

/** What the till does next. */
export type Outcome =
  /** The sale is stored; the server rang it (fresh or replayed). */
  | { kind: "rang" }
  /** Nobody is signed in any more: drop the session, show the sign-in. */
  | { kind: "sign-in-again"; say: Say }
  /** This phone is no longer trusted: drop the device token too, re-pair. */
  | { kind: "pair-again"; say: Say }
  /** The server read the request and said no. Show it; queue nothing. */
  | { kind: "refused"; say: Say }
  /** The call did not get an answer. Queue it and retry later. */
  | { kind: "queue" };

/** The outcomes that carry something to tell the cashier, and therefore
 *  the ones a thrown refusal can be built from. */
export type Refusal = Extract<Outcome, { say: Say }>;

/** The error body every refusal carries (`ApiErrorPayloadDto`). Its
 *  `message` is on the wire and is deliberately not read here; see `Say`. */
export type ApiError = { code?: string; message?: string } | null;

/**
 * Sorts one HTTP answer.
 *
 * `status` is the response status, or null when `fetch` itself threw —
 * airplane mode, the desktop asleep, Wi-Fi gone. That, and only that, plus
 * a server that could not answer at all (5xx), is what the queue is for.
 *
 * A 401 names which credential failed in its error code: `device_refused`
 * is the pairing (re-pair), `auth_refused` is the PIN or password just
 * typed (type it again, nothing is cleared), anything else is the person's
 * session. So the phone clears exactly what died instead of signing the
 * cashier out of a shift they are still in the middle of, or, as it did
 * on 2026-09-16, wiping its pairing over one mistyped digit.
 */
export function outcomeOf(status: number | null, error: ApiError): Outcome {
  if (status === null) return { kind: "queue" };
  if (status >= 200 && status < 300) return { kind: "rang" };
  if (status >= 500) return { kind: "queue" };

  if (status === 401) {
    if (error?.code === "device_refused") return { kind: "pair-again", say: { key: "error_pair_again" } };
    if (error?.code === "auth_refused") return { kind: "refused", say: { key: "error_wrong_secret" } };
    return { kind: "sign-in-again", say: { key: "error_sign_in_again" } };
  }
  if (status === 403) {
    // The person is signed in; their role does not reach this. Neither
    // re-pairing nor signing in again changes that.
    return { kind: "refused", say: { key: "error_not_allowed" } };
  }
  // 409 is a refusal, not a queue. The core answers it two ways on the same
  // code and field (`CoreError::conflict("idempotency_key", …)`): a key
  // whose winner is readable — worth sending again — and a key reused on a
  // different basket, which no retry will ever fix. Only the prose tells
  // them apart, and queueing on a message string is how the bug this
  // module exists to kill got written. Both are shown to the cashier, who
  // rings again with a fresh key.
  //
  // The status is all that sentence has to offer, and on a 422 that is less
  // than the server said. What would close the gap is the desktop's shape: a
  // record from the core's error codes to keys, so a credit limit or a
  // validation failure gets its own sentence. It is not written here because
  // the phone rings cash sales and nothing else yet, and a map built from
  // guesses is a dictionary of keys no screen reaches. It is on the plan page
  // rather than in this comment, so it is visible without opening this file.
  return { kind: "refused", say: { key: "error_refused", vars: { status } } };
}
