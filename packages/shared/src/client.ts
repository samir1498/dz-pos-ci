// The one HTTP client. The desktop webview, the browser preview and, later,
// the phone all use it, so none of them knows which mode it is in (rule 1).
//
// No `as` casts: every answer is parsed by the zod schema of its DTO
// (`./schemas`), so a server that answers the wrong shape raises a
// translatable error instead of leaking a half-typed object into the UI. The
// schemas are pinned to `./generated` in both directions, so the check the
// client runs cannot drift from the Rust struct it is checking.
//
// The calls themselves live one file per domain under `./client/`, the same
// split `crates/api/src/routes/` uses, each taking a `Transport` rather than
// reaching into this file's closure. `health`, `getBuildInfo` and `clock`
// stay here because they stay in `routes/mod.rs` too, ungrouped there; and
// `setSession`/`logout` stay here because both touch the `session` variable
// this closure holds, which no domain file can reach.

import type { BuildInfoDto } from "./generated/BuildInfoDto";
import type { ClockDto } from "./generated/ClockDto";
import type { HealthDto } from "./generated/HealthDto";
export {
  ApiError,
  type Download,
  type ExportKind,
  type PrintLang,
  type PrintPaper,
} from "./client-response";

import { ApiError, narrow, unwrap, unwrapFile, unwrapText, type Download } from "./client-response";
import { buildInfoSchema, clockSchema, healthSchema } from "./schemas/settings";

import { auditClient } from "./client/audit";
import { bookClient } from "./client/book";
import { authClient } from "./client/auth";
import { backupsClient } from "./client/backups";
import { categoriesClient } from "./client/categories";
import { customersClient } from "./client/customers";
import { dashboardClient } from "./client/dashboard";
import { expensesClient } from "./client/expenses";
import { exportClient } from "./client/export";
import { importClient } from "./client/import";
import { pairingClient } from "./client/pairing";
import { patientsClient } from "./client/patients";
import { productsClient } from "./client/products";
import { purchasesClient } from "./client/purchases";
import { queueClient } from "./client/queue";
import { salesClient } from "./client/sales";
import { settingsClient } from "./client/settings";
import { stockClient } from "./client/stock";
import { suppliersClient } from "./client/suppliers";
import { supportClient } from "./client/support";
import { tillClient } from "./client/till";
import { usersClient } from "./client/users";

export type ApiClient = ReturnType<typeof createClient>;

export interface ClientOptions {
  /** The launch token the server was started with; sent as a bearer on
   * every call. The browser preview reads VITE_API_TOKEN and passes it as a
   * plain string. The desktop never holds the token in a variable of its
   * own: it passes a function that asks the Tauri side for it, awaited here
   * on the request that needs it, so a call this client never makes is a
   * call the token is never fetched for. Without it every route but
   * /health answers 401. */
  readonly token?: string | (() => Promise<string | undefined>);
  /** The session token, if one is already in hand. Two different things
   * (M4 T2): the launch token above says the caller is this machine's own
   * screen, this says which person is at it. A browser leaves this alone and
   * lets the httpOnly cookie the sign-in set travel by itself; the desktop
   * cannot read that cookie, so it holds the token and `setSession` puts it
   * here. Without one every route but /health and the auth ones answers 401
   * `session_required`. */
  readonly session?: string;
  /** A fetch to use instead of the global one (tests). */
  readonly fetch?: typeof fetch;
}

/** The header the session token travels in. Not `Authorization`, which
 * already carries the launch token: one header cannot carry two credentials,
 * and the two gates are separate on purpose
 * (`docs/architecture.md` § Transport and auth). */
export const SESSION_HEADER = "x-dzpos-session";

/** The three ways a call gets sent, handed to each domain factory under
 * `./client/` so none of them has to hold the token, the session or the
 * fetch to use. */
export interface Transport {
  send(path: string, init?: RequestInit): Promise<unknown>;
  sendText(path: string, init?: RequestInit): Promise<string>;
  sendFile(path: string, init?: RequestInit): Promise<Download>;
}

export function createClient(baseUrl: string, options: ClientOptions | typeof fetch = {}) {
  const base = baseUrl.replace(/\/+$/, "");
  const opts: ClientOptions = typeof options === "function" ? { fetch: options } : options;
  // Resolved on each call, not captured at module load: a test that stubs
  // globalThis.fetch after importing this module must still be seen.
  const send0: typeof fetch = opts.fetch ?? ((input, init) => globalThis.fetch(input, init));
  const token = opts.token;
  // Mutable, unlike the launch token: a sign-in hands one over and a sign-out
  // takes it away, both while the same client object is in use.
  let session = opts.session;

  /** Puts the launch token and the session, if there is one, on a request.
   * One place, so a call added later cannot forget either. `credentials` is
   * what makes a browser attach the httpOnly cookie across the preview's
   * origin; the desktop sends the header instead and the server takes the
   * header first.
   *
   * Async because the desktop's token is: `token` there is a function, not
   * a string, and it is only called here, on the request that needs it. */
  async function authorised(init?: RequestInit): Promise<RequestInit> {
    const headers = new Headers(init?.headers);
    const shown = typeof token === "function" ? await token() : token;
    if (shown !== undefined && shown !== "") headers.set("authorization", `Bearer ${shown}`);
    if (session !== undefined && session !== "") headers.set(SESSION_HEADER, session);
    return { ...init, headers, credentials: "include" };
  }

  async function send(path: string, init?: RequestInit): Promise<unknown> {
    let res: Response;
    try {
      res = await send0(`${base}${path}`, await authorised(init));
    } catch (cause) {
      throw new ApiError("unreachable", `cannot reach ${base}`, 0);
    }
    return unwrap(res);
  }

  /** The same call, for a route that answers a document instead of JSON. */
  async function sendText(path: string, init?: RequestInit): Promise<string> {
    let res: Response;
    try {
      res = await send0(`${base}${path}`, await authorised(init));
    } catch {
      throw new ApiError("unreachable", `cannot reach ${base}`, 0);
    }
    return unwrapText(res);
  }

  /** The same call again, for a route that answers a file. The name the
   * server put on it comes back beside the bytes: the desktop saves under
   * that name rather than inventing one, which is why the CORS layer
   * exposes `content-disposition`. */
  async function sendFile(path: string, init?: RequestInit): Promise<Download> {
    let res: Response;
    try {
      res = await send0(`${base}${path}`, await authorised(init));
    } catch {
      throw new ApiError("unreachable", `cannot reach ${base}`, 0);
    }
    return unwrapFile(res);
  }

  const transport: Transport = { send, sendText, sendFile };

  return {
    baseUrl: base,

    /** The session token this client shows from now on, or `null` to stop
     * showing one. The desktop calls it after a sign-in and after a sign-out
     * (T4); a browser never needs to, because its cookie travels on its own. */
    setSession(next: string | null): void {
      session = next ?? undefined;
    },

    /** Ends the session and forgets the token, whether or not the server had
     * one to end. */
    async logout(): Promise<void> {
      try {
        await send("/auth/logout", { method: "POST" });
      } finally {
        session = undefined;
      }
    },

    async health(): Promise<HealthDto> {
      return narrow(await send("/health"), healthSchema, "health answer");
    },

    /** The version, the git short hash and the build date this server was
     * built with, and whether it is a debug build. The About screen's only
     * source: it never keeps its own copy of any of the three. */
    async getBuildInfo(): Promise<BuildInfoDto> {
      return narrow(await send("/build-info"), buildInfoSchema, "build info");
    },

    /** The day the shop is on. Asked for rather than read off the machine:
     * the core dates documents on Algeria's calendar and a browser in
     * another zone would be a day out either way. */
    async clock(): Promise<ClockDto> {
      return narrow(await send("/clock"), clockSchema, "clock answer");
    },

    ...authClient(transport),
    ...pairingClient(transport),
    ...auditClient(transport),
    ...categoriesClient(transport),
    ...productsClient(transport),
    ...exportClient(transport),
    ...importClient(transport),
    ...settingsClient(transport),
    ...backupsClient(transport),
    ...salesClient(transport),
    ...customersClient(transport),
    ...suppliersClient(transport),
    ...usersClient(transport),
    ...expensesClient(transport),
    ...dashboardClient(transport),
    ...purchasesClient(transport),
    ...stockClient(transport),
    ...supportClient(transport),
    ...tillClient(transport),
    ...patientsClient(transport),
    ...queueClient(transport),
    ...bookClient(transport),
  };
}
