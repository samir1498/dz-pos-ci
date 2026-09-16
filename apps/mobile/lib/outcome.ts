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

/** What the till does next. */
export type Outcome =
  /** The sale is stored; the server rang it (fresh or replayed). */
  | { kind: "rang" }
  /** Nobody is signed in any more: drop the session, show the sign-in. */
  | { kind: "sign-in-again"; message: string }
  /** This phone is no longer trusted: drop the device token too, re-pair. */
  | { kind: "pair-again"; message: string }
  /** The server read the request and said no. Show it; queue nothing. */
  | { kind: "refused"; message: string }
  /** The call did not get an answer. Queue it and retry later. */
  | { kind: "queue" };

/** The error body every refusal carries (`ApiErrorPayloadDto`). */
export type ApiError = { code?: string; message?: string } | null;

const SIGN_IN_AGAIN = "Session expired — sign in again.";
const PAIR_AGAIN = "This phone is no longer paired. Ask a manager for a new QR.";

/**
 * Sorts one HTTP answer.
 *
 * `status` is the response status, or null when `fetch` itself threw —
 * airplane mode, the desktop asleep, Wi-Fi gone. That, and only that, plus
 * a server that could not answer at all (5xx), is what the queue is for.
 *
 * A 401 names which of the three credentials failed in its error code
 * (`auth_refused` is the device; anything else is the person's session),
 * so the phone can clear exactly what died instead of signing the cashier
 * out of a shift they are still in the middle of.
 */
export function outcomeOf(status: number | null, error: ApiError): Outcome {
  if (status === null) return { kind: "queue" };
  if (status >= 200 && status < 300) return { kind: "rang" };
  if (status >= 500) return { kind: "queue" };

  const message = error?.message ?? "";
  if (status === 401) {
    return error?.code === "auth_refused"
      ? { kind: "pair-again", message: message || PAIR_AGAIN }
      : { kind: "sign-in-again", message: message || SIGN_IN_AGAIN };
  }
  if (status === 403) {
    // The person is signed in; their role does not reach this. Neither
    // re-pairing nor signing in again changes that.
    return { kind: "refused", message: message || "You are not allowed to do that." };
  }
  // 409 is a refusal, not a queue. The core answers it two ways on the same
  // code and field (`CoreError::conflict("idempotency_key", …)`): a key
  // whose winner is readable — worth sending again — and a key reused on a
  // different basket, which no retry will ever fix. Only the prose tells
  // them apart, and queueing on a message string is how the bug this
  // module exists to kill got written. Both are shown to the cashier, who
  // rings again with a fresh key.
  return { kind: "refused", message: message || `Refused (${status}).` };
}
