// Where the UI finds the API and what it shows to be let in. In the Tauri
// window the desktop process injects the port it bound; in a browser
// (`pnpm desktop dev`) both the port and the token come from the
// environment (`just api` writes the token to .dev/api-token and `just dev`
// passes it). Same routes either way, so no screen knows which mode it is
// in (architecture.md rule 1).
//
// The launch token is the one thing that does not travel as a page global.
// A global set once at launch sits there for the rest of the session,
// readable by anything that later runs in the page, including whatever
// document the window gets navigated to (`on_navigation` in
// `src-tauri/src/lib.rs` is the guard against that navigating anywhere
// that matters, but the token not being there either is a second, cheaper
// one). The desktop instead asks for it over `invoke`, once: the Rust side
// hands it out to the first caller and refuses every caller after
// (docs/architecture.md § Release). The alternative considered was having
// Rust attach the `Authorization` header itself on every call, which would
// need the webview's own HTTP requests proxied through Tauri IPC — the
// thing rule 2 above says data never does, and CSP's `connect-src` already
// closes the direct exfiltration path a global was open to.

import { invoke, isTauri } from "@tauri-apps/api/core";
import { createClient } from "@dzpos/shared";
import type { PrintLang, PrintPaper } from "@dzpos/shared";

const FALLBACK = "http://127.0.0.1:4317";

function injected(name: "__DZPOS_API_URL__"): string | null {
  const global: unknown = globalThis;
  if (typeof global !== "object" || global === null) return null;
  if (!(name in global)) return null;
  const value: unknown = Reflect.get(global, name);
  return typeof value === "string" && value !== "" ? value : null;
}

export function apiBaseUrl(): string {
  const value = injected("__DZPOS_API_URL__");
  if (value !== null) return value;
  const configured = import.meta.env.VITE_API_URL;
  return typeof configured === "string" && configured !== "" ? configured : FALLBACK;
}

/** Asks the Tauri side for the launch token, once. The command answers
 * exactly one caller; memoized here because this module, not the command,
 * is where "once" has to be kept, since `apiToken()`'s function form is
 * called again on every request the client makes. */
let tauriToken: Promise<string | undefined> | undefined;
function tauriLaunchToken(): Promise<string | undefined> {
  tauriToken ??= invoke<string>("take_launch_token").catch(() => undefined);
  return tauriToken;
}

/** In the Tauri window, a function the client awaits on each request rather
 * than a string held here. In a browser started without one, `undefined`;
 * the API then answers 401 and the screen shows the translated
 * "unauthorized" error. */
export function apiToken(): string | undefined | (() => Promise<string | undefined>) {
  if (isTauri()) return tauriLaunchToken;
  const configured = import.meta.env.VITE_API_TOKEN;
  return typeof configured === "string" && configured !== "" ? configured : undefined;
}

export const api = createClient(apiBaseUrl(), { token: apiToken() });

export const productsQueryKey: readonly string[] = ["products"];
export const categoriesQueryKey: readonly string[] = ["categories"];
export const settingsQueryKey: readonly string[] = ["settings"];
/** The shop's day. Its own key and never cached (see lib/clock.ts): every
 * other answer here is a row that changes when someone changes it, and this
 * one changes on its own at midnight. */
export const clockQueryKey: readonly string[] = ["clock"];
export const backupsQueryKey: readonly string[] = ["backups"];
/** The last stock recount: the day it ran and what it corrected. Its own key
 * rather than a slice of the products': a recount changes the quantities the
 * product list carries, so running one invalidates both. */
export const stockRecountQueryKey: readonly string[] = ["stock-recount"];
/** The seeded expense categories. One list for the shop, so it has its own
 * key and nothing invalidates it: no screen writes one in this version. */
export const expenseCategoriesQueryKey: readonly string[] = ["expense-categories"];

/** One month of expenses with its total. The month is part of the key: two
 * months are two answers, and a row filed into one must not be read out of
 * the cache of the other. */
export function expensesQueryKey(month: string): readonly string[] {
  return ["expenses", month];
}

/** The cash position over a day or a month. The range is part of the key for
 * the reason the month's is, and the two shapes are told apart by which word
 * is in the key rather than by the value alone: a day and a month can never
 * be the same string, but a reader of the key should not have to know that. */
export function cashQueryKey(period: { day: string } | { month: string }): readonly string[] {
  return "day" in period ? ["cash", "day", period.day] : ["cash", "month", period.month];
}
/** The whole dashboard for one day and the month it falls in. The day is in
 * the key: the screen asks about the shop's today, and a window left open
 * over midnight must read the new day's answer rather than yesterday's out
 * of the cache. */
export function dashboardQueryKey(day: string): readonly string[] {
  return ["dashboard", day];
}

/** The chart behind it. Its own key rather than a slice of the dashboard's,
 * because it is a second call over a second window: the two are refetched
 * together only because the screen asks for both. */
export function dashboardSeriesQueryKey(day: string, days: number): readonly (string | number)[] {
  return ["dashboard", "series", day, days];
}

/** The customer list. The search text is appended by the screen, so an
 * invalidation of this key refreshes every search that is in the cache. */
export const customersQueryKey: readonly string[] = ["customers"];

/** The supplier list. The search text is appended by the screen, so an
 * invalidation of this key refreshes every search that is in the cache. */
export const suppliersQueryKey: readonly string[] = ["suppliers"];

/** The shop's staff (M4 T8). The owner's own read: `crate::gates` names
 * `ManageUsers` on `GET /users`, so a cashier or a manager reading this key
 * gets the translated 403 rather than a list. */
export const usersQueryKey: readonly string[] = ["users"];

/** The order list. The two filters are appended by the screen, so an
 * invalidation of this key refreshes every filter that is in the cache: a
 * delivery moves an order from one state to another, and the list narrowed
 * to the state it left must not go on showing it. */
export const purchasesQueryKey: readonly string[] = ["purchases"];

/** The owner's audit log (M4 T7). The three filters and the page are
 * appended by the screen, the same way the purchase list's are. */
export const auditLogQueryKey: readonly string[] = ["audit-log"];

/** One order with its lines and its deliveries. Its own key rather than a
 * slice of the list's: the list answers a row per order and this answers the
 * whole thing, and a delivery makes both stale. */
export function purchaseQueryKey(id: number): readonly (string | number)[] {
  return ["purchase", id];
}

/** One supplier's fiche, read on its own by the route that opens a fiche by
 * id. Under the list's key on purpose: a payment or a correction invalidates
 * `suppliersQueryKey` and this refetches with it, so the balance the page
 * shows and the balance the list shows are never two answers. */
export function supplierQueryKey(id: number): readonly (string | number)[] {
  return [...suppliersQueryKey, id];
}

/** One supplier's movements. A factory rather than a literal at the call
 * site, so the id is always the second element and never a template string. */
export function supplierLedgerQueryKey(id: number): readonly (string | number)[] {
  return ["supplier-ledger", id];
}

/** One customer's fiche, read on its own by the route that opens a fiche by
 * id. Under the list's key on purpose: a payment or a correction invalidates
 * `customersQueryKey` and this refetches with it, so the balance the page
 * shows and the balance the list shows are never two answers. */
export function customerQueryKey(id: number): readonly (string | number)[] {
  return [...customersQueryKey, id];
}

/** One customer's movements. A factory rather than a literal at the call
 * site, so the id is always the second element and never a template string. */
export function customerLedgerQueryKey(id: number): readonly (string | number)[] {
  return ["customer-ledger", id];
}

/** One customer's payments, with what each one settled. Its own key rather
 * than a slice of the ledger's: the two screens read different shapes and a
 * payment invalidates both. */
export function customerPaymentsQueryKey(id: number): readonly (string | number)[] {
  return ["customer-payments", id];
}

/** The rendered statement of one customer over a range of days. The days and
 * the language are part of the key: the core renders the page, so the same
 * customer over another month, or in Arabic, is a different document and must
 * not be answered from the one already in the cache. */
export function customerStatementQueryKey(
  id: number,
  from: string,
  to: string,
  lang: PrintLang,
): readonly (string | number)[] {
  return ["customer-statement", id, from, to, lang];
}

/** The rendered debt slip of one customer. The language is part of the key
 * for the reason the statement's is; the balance is not, because a slip asked
 * for again after a payment is a new call and the payment invalidated this
 * key along with the ledger's. */
export function customerDebtSlipQueryKey(
  id: number,
  lang: PrintLang,
): readonly (string | number)[] {
  return [...customerDebtSlipKeyPrefix(id), lang];
}

/** Every language's slip for one customer, which is what a payment or a
 * correction has just made stale: the shop may have printed the French one
 * and the Arabic one, and both now say a balance the ledger no longer sums
 * to. */
export function customerDebtSlipKeyPrefix(id: number): readonly (string | number)[] {
  return ["customer-debt-slip", id];
}

/** The rendered ticket of one stored sale. A factory rather than a literal
 * at the call site, so the id is always the second element and never a
 * template string. The language is part of the key: the core renders the
 * page in it, so the same sale in Arabic is a different document and must
 * not be answered from the French one already in the cache. */
export function saleTicketQueryKey(
  id: number,
  lang: PrintLang,
): readonly (string | number)[] {
  return ["sale-ticket", id, lang];
}

/** The facture is keyed by its sheet as well as its language: A4 and A5 are
 * two pages the core renders, and a cached A4 must not be handed back when
 * the cashier asked for the half sheet. */
export function saleFactureQueryKey(
  id: number,
  lang: PrintLang,
  paper: PrintPaper,
): readonly (string | number)[] {
  return ["sale-facture", id, lang, paper];
}

/** Every rendered page of one document, whatever language and whatever sheet
 * it was asked for in. A cancellation changes the paper itself, so all of
 * them are stale at once and naming each language and each sheet to invalidate
 * them would be a list that goes wrong the day a fourth paper size exists.
 *
 * Two prefixes because the two papers are two keys: an 80 mm ticket and a
 * sheet are rendered by different routes. */
export function saleSheetPrefixes(id: number): readonly (readonly (string | number)[])[] {
  return [
    ["sale-ticket", id],
    ["sale-facture", id],
  ];
}

/** One document read on its own, which is what the documents screen opens a
 * row into. A factory beside the list's, so the id is always the second
 * element and no call site spells the key out. */
export function saleQueryKey(id: number): readonly (string | number)[] {
  return ["sale", id];
}

/** The document list, keyed by the kind filter so the narrowed list and the
 * whole one are two entries and a filter change is not answered from the
 * other's cache. `undefined` is every kind, which is what the route reads a
 * missing `kind` as. */
export function salesQueryKey(kind?: string): readonly (string | undefined)[] {
  return ["sales", kind];
}

/** Every list entry whatever its filter. A write that changes a document's
 * status changes what each of them holds, and invalidating one filter's key
 * leaves the others answering from a cache the write made wrong: react-query
 * matches a filter key element by element, so `["sales", undefined]` misses
 * `["sales", "facture"]` rather than covering it. */
export function salesQueryPrefix(): readonly string[] {
  return ["sales"];
}

/** Every avoir written against one facture. Its own key rather than a slice
 * of the document's: writing one changes both, and the detail panel reads
 * the two shapes separately. */
export function saleAvoirsQueryKey(id: number): readonly (string | number)[] {
  return ["sale-avoirs", id];
}
