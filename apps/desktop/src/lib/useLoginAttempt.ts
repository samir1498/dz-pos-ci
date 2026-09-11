// One sign-in call, held with the one thing a sign-in or a lock screen must
// not work out for itself: the wait after too many wrong PINs. Shared by
// both, because a wrong password and a wrong PIN answer the same two codes
// (`auth_refused`, `locked_out`) off the same route, and a second copy of
// this would be the second statement architecture.md rule 2 warns about.

import { ApiError } from "@dzpos/shared";
import { useCallback, useEffect, useState } from "react";

import type { Key } from "@/i18n";

const ERROR_KEY: Record<string, Key> = {
  auth_refused: "signin_error_auth_refused",
  locked_out: "signin_error_locked_out",
  validation: "error_validation",
  not_found: "error_not_found",
  storage: "error_storage",
  restart_needed: "error_restart_needed",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unauthorized: "error_unauthorized",
  unreachable: "error_unreachable",
};

function errorKey(error: unknown): Key {
  if (error instanceof ApiError) return ERROR_KEY[error.code] ?? "error_unknown";
  return "error_unknown";
}

export interface LoginAttempt {
  readonly run: () => Promise<void>;
  readonly pending: boolean;
  readonly error: Key | null;
  /** Seconds left before the till will look at a PIN again, straight off
   * the 429's `retry_after_seconds`. `null` outside a lockout. */
  readonly retryAfter: number | null;
  /** Whether the pad or the form should refuse input right now: pending, or
   * still inside the wait the server named. */
  readonly locked: boolean;
}

export function useLoginAttempt(attempt: () => Promise<void>): LoginAttempt {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<Key | null>(null);
  const [retryAfter, setRetryAfter] = useState<number | null>(null);

  // The countdown. One second at a time, off the figure the server sent;
  // this clock never recomputes the wait, it only ticks the number down to
  // zero and stops, which re-enables the pad without retrying anything on
  // its own.
  useEffect(() => {
    if (retryAfter === null || retryAfter <= 0) return;
    const id = setTimeout(() => setRetryAfter(retryAfter - 1), 1_000);
    return () => clearTimeout(id);
  }, [retryAfter]);

  const run = useCallback(async () => {
    setPending(true);
    setError(null);
    try {
      await attempt();
    } catch (cause) {
      if (cause instanceof ApiError && cause.code === "locked_out") {
        setRetryAfter(cause.retryAfterSeconds ?? 0);
      }
      setError(errorKey(cause));
    } finally {
      setPending(false);
    }
  }, [attempt]);

  return {
    run,
    pending,
    error,
    retryAfter,
    locked: pending || (retryAfter !== null && retryAfter > 0),
  };
}
