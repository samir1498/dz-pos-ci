// The one place the phone talks to a shop's core.
//
// Three credentials ride every guarded call, each proving one thing: the
// launch token says the caller may reach the server at all, the device token
// says which paired phone, the session token says which person. A 401 names
// which of the three failed in its error code, which is what `outcome.ts`
// reads to decide whether the cashier needs a PIN or a manager's QR.

import Constants from "expo-constants";

import { outcomeOf, type ApiError, type Outcome, type Refusal, type Say } from "./outcome";

/** Where the shop's core is, and the operator's secret for reaching it. */
export const API_BASE: string =
  (Constants.expoConfig?.extra?.apiUrl as string | undefined) ??
  process.env.EXPO_PUBLIC_API_URL ??
  "http://127.0.0.1:4317";

export const LAUNCH_TOKEN: string = process.env.EXPO_PUBLIC_API_TOKEN ?? "";

/** A refusal the server read the request to produce. Never worth retrying:
 * the query layer checks for this type before it retries anything. */
export class ApiRefusal extends Error {
  constructor(
    readonly status: number,
    readonly code: string | null,
    /** What `outcomeOf` made of the answer, whole.
     *
     *  Not just the sentence. A screen reads the kind from here rather
     *  than working it out again from the status and the code, so a read
     *  behind the device gate reaches the same conclusion a write behind
     *  it does: the sign-in screen finds out a phone was revoked from the
     *  staff list, which is a `useQuery`, not from the login call. The
     *  rule is docs/architecture.md's, map once and never re-derive.
     *
     *  The `Error` message beside it names the status for a log and a
     *  stack trace; it is English and no screen shows it. */
    readonly outcome: Refusal,
  ) {
    super(`refused ${status}${code === null ? "" : ` (${code})`}`);
    this.name = "ApiRefusal";
  }
}


export type Credentials = {
  deviceToken?: string | null;
  sessionToken?: string | null;
};

export function headersFor(credentials: Credentials): Record<string, string> {
  const headers: Record<string, string> = {
    "content-type": "application/json",
    Authorization: `Bearer ${LAUNCH_TOKEN}`,
  };
  if (credentials.deviceToken != null) headers["x-dzpos-device"] = credentials.deviceToken;
  if (credentials.sessionToken != null) headers["x-dzpos-session"] = credentials.sessionToken;
  return headers;
}

/** One call, with the answer already sorted. `status` is null when `fetch`
 * itself threw, which is the only thing the queue is for. */
export async function call<T>(
  path: string,
  // `RequestInit` has a `credentials` of its own (a same-origin cookie
  // policy, meaningless here), so ours takes the name and that one is
  // dropped rather than shadowed.
  init: Omit<RequestInit, "credentials"> & { credentials?: Credentials } = {},
): Promise<{ outcome: Outcome; body: T | null; status: number | null }> {
  const { credentials = {}, headers, ...rest } = init;
  let res: Response | null = null;
  try {
    res = await fetch(`${API_BASE}${path}`, {
      ...rest,
      headers: { ...headersFor(credentials), ...(headers as Record<string, string> | undefined) },
    });
  } catch {
    return { outcome: outcomeOf(null, null), body: null, status: null };
  }

  let parsed: unknown = null;
  try {
    parsed = await res.json();
  } catch {
    parsed = null;
  }
  const error = (parsed as { error?: ApiError } | null)?.error ?? null;
  return { outcome: outcomeOf(res.status, error), body: parsed as T | null, status: res.status };
}

/** The same call, for the read paths where a refusal is exceptional and
 * TanStack Query wants a thrown error rather than a sorted outcome. */
export async function get<T>(path: string, credentials: Credentials): Promise<T> {
  const { outcome, body, status } = await call<T>(path, { method: "GET", credentials });
  if (outcome.kind === "rang" && body !== null) return body;
  if (outcome.kind === "queue") throw new Error("no answer from the shop's core");
  // `rang` with no body is the one refusal `outcomeOf` cannot name: the
  // server said yes and sent nothing a caller can use.
  const refusal: Refusal =
    "say" in outcome
      ? outcome
      : { kind: "refused", say: { key: "error_refused", vars: { status: status ?? 0 } } };
  const code = (body as { error?: ApiError } | null)?.error?.code ?? null;
  throw new ApiRefusal(status ?? 0, code, refusal);
}
