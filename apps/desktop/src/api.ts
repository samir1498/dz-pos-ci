// Where the UI finds the API and what it shows to be let in. In the Tauri
// window the desktop process injects the port it bound and the launch
// token it made; in a browser (`pnpm desktop dev`) both come from the
// environment (`just api` writes the token to .dev/api-token and `just dev`
// passes it). Same routes either way, so no screen knows which mode it is
// in (architecture.md rule 1).

import { createClient } from "@dzpos/shared";
import type { PrintLang, PrintPaper } from "@dzpos/shared";

const FALLBACK = "http://127.0.0.1:4317";

function injected(name: "__DZPOS_API_URL__" | "__DZPOS_API_TOKEN__"): string | null {
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

/** Undefined in a browser started without one; the API then answers 401
 * and the screen shows the translated "unauthorized" error. */
export function apiToken(): string | undefined {
  const value = injected("__DZPOS_API_TOKEN__");
  if (value !== null) return value;
  const configured = import.meta.env.VITE_API_TOKEN;
  return typeof configured === "string" && configured !== "" ? configured : undefined;
}

export const api = createClient(apiBaseUrl(), { token: apiToken() });

export const productsQueryKey: readonly string[] = ["products"];
export const categoriesQueryKey: readonly string[] = ["categories"];
export const settingsQueryKey: readonly string[] = ["settings"];
export const backupsQueryKey: readonly string[] = ["backups"];
/** The customer list. The search text is appended by the screen, so an
 * invalidation of this key refreshes every search that is in the cache. */
export const customersQueryKey: readonly string[] = ["customers"];

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
