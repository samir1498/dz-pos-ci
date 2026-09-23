// The one place the phone talks to a shop's core.
//
// Three credentials ride every guarded call, each proving one thing: the
// launch token says the caller may reach the server at all, the device token
// says which paired phone, the session token says which person. A 401 names
// which of the three failed in its error code, which is what `outcome.ts`
// reads to decide whether the cashier needs a PIN or a manager's QR.

import Constants from "expo-constants";

import { outcomeOf, type ApiError, type Outcome, type Refusal, type Say } from "./outcome";

/** Where the shop's core is, and the operator's secret for reaching it.
 *
 *  `extra` is whatever `app.config.js` happens to hold, so it arrives as
 *  `unknown` and is checked rather than asserted. A build whose config
 *  carried a number there would otherwise have put a number where every
 *  URL in this file is built from a string. */
const configuredBase: unknown = Constants.expoConfig?.extra?.apiUrl;

export const API_BASE: string =
  (typeof configuredBase === "string" ? configuredBase : undefined) ??
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
  // `headers` is narrowed to a plain record here rather than taking
  // `RequestInit`'s three shapes. Every caller in this app passes a plain
  // object or nothing, and the alternative was asserting the union into
  // one at the spread, which is the same guess written later.
  init: Omit<RequestInit, "credentials" | "headers"> & {
    credentials?: Credentials;
    headers?: Record<string, string>;
  } = {},
): Promise<{ outcome: Outcome; body: T | null; status: number | null }> {
  const { credentials = {}, headers, ...rest } = init;
  let res: Response | null = null;
  try {
    res = await fetch(`${API_BASE}${path}`, {
      ...rest,
      headers: { ...headersFor(credentials), ...headers },
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
  return { outcome: outcomeOf(res.status, errorIn(parsed)), body: bodyOf<T>(parsed), status: res.status };
}

/** The error the core puts in a refusal body, if this answer has one.
 *
 *  Read rather than asserted. A 500 from something in front of the server,
 *  a proxy or a captive portal, answers HTML or a bare string, and
 *  reaching into that for `.error.code` is how a screen decides the phone
 *  was unpaired because a hotel Wi-Fi said hello. */
function errorIn(parsed: unknown): ApiError {
  if (typeof parsed !== "object" || parsed === null || !("error" in parsed)) return null;
  const error: unknown = parsed.error;
  if (typeof error !== "object" || error === null) return null;
  const code = "code" in error && typeof error.code === "string" ? error.code : undefined;
  const message =
    "message" in error && typeof error.message === "string" ? error.message : undefined;
  return { code, message };
}

/** The one assertion left in the phone, and it is the wire.
 *
 *  `T` is what the caller says the route answers; nothing exists at
 *  runtime to check it against. Removing it means a schema per DTO, which
 *  is a real piece of work and is written down on the architecture plan
 *  rather than hidden behind a guess here. What it is not is a licence:
 *  everything read *out* of this answer on a refusal path goes through
 *  `errorIn` above, so a body that is not the shape the caller expected
 *  cannot decide what the phone does next. It can only be rendered empty. */
function bodyOf<T>(parsed: unknown): T | null {
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions
  return (parsed ?? null) as T | null;
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
  const code = errorIn(body)?.code ?? null;
  throw new ApiRefusal(status ?? 0, code, refusal);
}
