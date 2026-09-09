// The one HTTP client. The desktop webview, the browser preview and, later,
// the phone all use it, so none of them knows which mode it is in (rule 1).
//
// No `as` casts: every response is narrowed by a guard, so a server that
// answers the wrong shape raises a translatable error instead of leaking a
// half-typed object into the UI.

import type { ApiErrorDto } from "./generated/ApiErrorDto";
import type { BackupDto } from "./generated/BackupDto";
import type { BackupsDto } from "./generated/BackupsDto";
import type { CategoryDto } from "./generated/CategoryDto";
import type { HealthDto } from "./generated/HealthDto";
import type { DocumentKindDto } from "./generated/DocumentKindDto";
import type { DocumentStatusDto } from "./generated/DocumentStatusDto";
import type { NewProductDto } from "./generated/NewProductDto";
import type { NewSaleDto } from "./generated/NewSaleDto";
import type { PaymentModeDto } from "./generated/PaymentModeDto";
import type { SaleDto } from "./generated/SaleDto";
import type { SaleLineDto } from "./generated/SaleLineDto";
import type { SaleTotalsDto } from "./generated/SaleTotalsDto";
import type { SaleTvaDto } from "./generated/SaleTvaDto";
import type { ProductDto } from "./generated/ProductDto";
import type { RegimeChangeDto } from "./generated/RegimeChangeDto";
import type { RestoreDto } from "./generated/RestoreDto";
import type { DatedRegimeDto } from "./generated/DatedRegimeDto";
import type { RegimeDto } from "./generated/RegimeDto";
import type { SettingsDto } from "./generated/SettingsDto";
import type { StoreDto } from "./generated/StoreDto";
import type { UnitDto } from "./generated/UnitDto";

/** An error the server described. `code` is a translation key. */
export class ApiError extends Error {
  readonly code: string;
  readonly status: number;

  constructor(code: string, message: string, status: number) {
    super(message);
    this.name = "ApiError";
    this.code = code;
    this.status = status;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isNullableNumber(value: unknown): value is number | null {
  return value === null || typeof value === "number";
}

const UNITS: readonly UnitDto[] = ["piece", "kg", "litre", "box"];

function isUnit(value: unknown): value is UnitDto {
  return typeof value === "string" && UNITS.some((u) => u === value);
}

export function isApiErrorBody(value: unknown): value is ApiErrorDto {
  if (!isRecord(value)) return false;
  const { error } = value;
  return (
    isRecord(error) && typeof error.code === "string" && typeof error.message === "string"
  );
}

export function isCategory(value: unknown): value is CategoryDto {
  return (
    isRecord(value) &&
    typeof value.id === "number" &&
    typeof value.shop_id === "number" &&
    typeof value.name === "string" &&
    typeof value.default_rate_bps === "number"
  );
}

function isCategoryList(value: unknown): value is CategoryDto[] {
  return Array.isArray(value) && value.every(isCategory);
}

export function isHealth(value: unknown): value is HealthDto {
  return isRecord(value) && typeof value.status === "string" && typeof value.shop_id === "number";
}

/** An amount or a quantity: an integer JSON.parse did not have to round. */
function isExactInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value);
}

function isNullableExactInteger(value: unknown): value is number | null {
  return value === null || isExactInteger(value);
}

export function isProduct(value: unknown): value is ProductDto {
  return (
    isRecord(value) &&
    typeof value.id === "number" &&
    typeof value.shop_id === "number" &&
    typeof value.name === "string" &&
    isNullableString(value.barcode) &&
    isNullableNumber(value.category_id) &&
    isUnit(value.unit) &&
    isExactInteger(value.cost_centimes) &&
    isExactInteger(value.selling_centimes) &&
    isNullableExactInteger(value.wholesale_centimes) &&
    isExactInteger(value.qty_on_hand_milli) &&
    isExactInteger(value.low_stock_at_milli) &&
    typeof value.rate_bps === "number" &&
    typeof value.active === "boolean"
  );
}

function isProductList(value: unknown): value is ProductDto[] {
  return Array.isArray(value) && value.every(isProduct);
}

const REGIMES: readonly RegimeDto[] = ["ifu", "reel"];

function isRegime(value: unknown): value is RegimeDto {
  return typeof value === "string" && REGIMES.some((r) => r === value);
}

/** `YYYY-MM-DD`, the only shape the API writes a day in. */
const DAY = /^\d{4}-\d{2}-\d{2}$/;

function isDay(value: unknown): value is string {
  return typeof value === "string" && DAY.test(value);
}

export function isStore(value: unknown): value is StoreDto {
  return (
    isRecord(value) &&
    typeof value.name === "string" &&
    isNullableString(value.rc) &&
    isNullableString(value.nif) &&
    isNullableString(value.nis) &&
    isNullableString(value.ai) &&
    isNullableString(value.address) &&
    isNullableString(value.phone)
  );
}

function isDatedRegime(value: unknown): value is DatedRegimeDto {
  return isRecord(value) && isRegime(value.regime) && isDay(value.valid_from);
}

export function isSettings(value: unknown): value is SettingsDto {
  return (
    isRecord(value) &&
    isStore(value.store) &&
    isDatedRegime(value.regime) &&
    (value.regime_planned === null || isDatedRegime(value.regime_planned))
  );
}

export function isBackup(value: unknown): value is BackupDto {
  return (
    isRecord(value) &&
    typeof value.name === "string" &&
    typeof value.taken_at === "string" &&
    // A file size, so an integer: a fractional byte count means the server
    // is not the one this client was generated against.
    isExactInteger(value.bytes)
  );
}

function isBackupList(value: unknown): value is BackupDto[] {
  return Array.isArray(value) && value.every(isBackup);
}

export function isBackups(value: unknown): value is BackupsDto {
  return (
    isRecord(value) && isBackupList(value.backups) && isBackupList(value.safety_copies)
  );
}

export function isRestore(value: unknown): value is RestoreDto {
  return (
    isRecord(value) &&
    typeof value.restored_from === "string" &&
    typeof value.safety_copy === "string" &&
    isExactInteger(value.products) &&
    isNullableExactInteger(value.documents)
  );
}

const PAYMENT_MODES: readonly PaymentModeDto[] = ["cash", "card", "credit"];

function isPaymentMode(value: unknown): value is PaymentModeDto {
  return typeof value === "string" && PAYMENT_MODES.some((m) => m === value);
}

const DOCUMENT_KINDS: readonly DocumentKindDto[] = [
  "ticket",
  "facture",
  "proforma",
  "bon_de_livraison",
  "avoir",
  "bon_de_reception",
];

function isDocumentKind(value: unknown): value is DocumentKindDto {
  return typeof value === "string" && DOCUMENT_KINDS.some((k) => k === value);
}

const DOCUMENT_STATUSES: readonly DocumentStatusDto[] = ["issued", "cancelled"];

function isDocumentStatus(value: unknown): value is DocumentStatusDto {
  return typeof value === "string" && DOCUMENT_STATUSES.some((s) => s === value);
}

function isSaleLine(value: unknown): value is SaleLineDto {
  return (
    isRecord(value) &&
    typeof value.id === "number" &&
    typeof value.position === "number" &&
    isNullableNumber(value.product_id) &&
    typeof value.name === "string" &&
    isNullableString(value.barcode) &&
    isExactInteger(value.qty_milli) &&
    isExactInteger(value.unit_price_centimes) &&
    isExactInteger(value.line_discount_centimes) &&
    typeof value.rate_bps === "number" &&
    isExactInteger(value.line_total_centimes)
  );
}

function isSaleTva(value: unknown): value is SaleTvaDto {
  return (
    isRecord(value) &&
    typeof value.rate_bps === "number" &&
    isExactInteger(value.base_centimes) &&
    isExactInteger(value.amount_centimes)
  );
}

/** Every column of the totals table, each an exact integer of centimes: a
 *  total JSON.parse had to round is refused rather than printed. */
function isSaleTotals(value: unknown): value is SaleTotalsDto {
  return (
    isRecord(value) &&
    isExactInteger(value.total_ht_centimes) &&
    isExactInteger(value.discount_centimes) &&
    isExactInteger(value.subtotal_ht_centimes) &&
    isExactInteger(value.tva_centimes) &&
    isExactInteger(value.total_ttc_centimes) &&
    isExactInteger(value.stamp_centimes) &&
    isExactInteger(value.net_to_pay_centimes)
  );
}

export function isSale(value: unknown): value is SaleDto {
  return (
    isRecord(value) &&
    typeof value.id === "number" &&
    typeof value.shop_id === "number" &&
    isDocumentKind(value.kind) &&
    typeof value.series === "string" &&
    isExactInteger(value.number) &&
    typeof value.issued_at === "string" &&
    typeof value.user_id === "number" &&
    isRegime(value.regime) &&
    isPaymentMode(value.payment_mode) &&
    isStore(value.seller) &&
    isNullableNumber(value.customer_id) &&
    isSaleTotals(value.totals) &&
    Array.isArray(value.tva) &&
    value.tva.every(isSaleTva) &&
    isNullableExactInteger(value.tendered_centimes) &&
    isNullableExactInteger(value.change_centimes) &&
    isDocumentStatus(value.status) &&
    Array.isArray(value.lines) &&
    value.lines.every(isSaleLine)
  );
}

function isSaleList(value: unknown): value is SaleDto[] {
  return Array.isArray(value) && value.every(isSale);
}

async function unwrap(res: Response): Promise<unknown> {
  const text = await res.text();
  let body: unknown = null;
  if (text !== "") {
    try {
      body = JSON.parse(text);
    } catch {
      body = null;
    }
  }
  if (res.ok) return body;
  if (isApiErrorBody(body)) {
    throw new ApiError(body.error.code, body.error.message, res.status);
  }
  // The server always sends the shape above; anything else is the network
  // or a proxy, so the UI still gets a key it can translate.
  throw new ApiError("unreachable", `HTTP ${res.status}`, res.status);
}

/** The languages a document prints in. Not a generated DTO: the language
 * is a query parameter and never crosses in a body, so there is no Rust
 * struct to generate it from. `crates/core/src/lang.rs` is the other half
 * and `dzpos_core::lang::Lang` refuses anything else with a 422. */
export type PrintLang = "fr" | "en" | "ar";

/** A body that is a page, not JSON. Only the error path is JSON, and it is
 * the same envelope every other call answers with. */
async function unwrapText(res: Response): Promise<string> {
  const text = await res.text();
  if (res.ok) return text;
  let body: unknown = null;
  try {
    body = JSON.parse(text);
  } catch {
    body = null;
  }
  if (isApiErrorBody(body)) {
    throw new ApiError(body.error.code, body.error.message, res.status);
  }
  throw new ApiError("unreachable", `HTTP ${res.status}`, res.status);
}

function narrow<T>(body: unknown, guard: (v: unknown) => v is T, what: string): T {
  if (guard(body)) return body;
  throw new ApiError("bad_response", `the server sent an unexpected ${what}`, 0);
}

export type ApiClient = ReturnType<typeof createClient>;

export interface ClientOptions {
  /** The launch token the server was started with; sent as a bearer on
   * every call. The desktop injects it, the browser preview reads
   * VITE_API_TOKEN. Without it every route but /health answers 401. */
  readonly token?: string;
  /** A fetch to use instead of the global one (tests). */
  readonly fetch?: typeof fetch;
}

export function createClient(baseUrl: string, options: ClientOptions | typeof fetch = {}) {
  const base = baseUrl.replace(/\/+$/, "");
  const opts: ClientOptions = typeof options === "function" ? { fetch: options } : options;
  // Resolved on each call, not captured at module load: a test that stubs
  // globalThis.fetch after importing this module must still be seen.
  const send0: typeof fetch = opts.fetch ?? ((input, init) => globalThis.fetch(input, init));
  const token = opts.token;

  async function send(path: string, init?: RequestInit): Promise<unknown> {
    const headers = new Headers(init?.headers);
    if (token !== undefined && token !== "") headers.set("authorization", `Bearer ${token}`);
    let res: Response;
    try {
      res = await send0(`${base}${path}`, { ...init, headers });
    } catch (cause) {
      throw new ApiError("unreachable", `cannot reach ${base}`, 0);
    }
    return unwrap(res);
  }

  /** The same call, for a route that answers a document instead of JSON. */
  async function sendText(path: string, init?: RequestInit): Promise<string> {
    const headers = new Headers(init?.headers);
    if (token !== undefined && token !== "") headers.set("authorization", `Bearer ${token}`);
    let res: Response;
    try {
      res = await send0(`${base}${path}`, { ...init, headers });
    } catch {
      throw new ApiError("unreachable", `cannot reach ${base}`, 0);
    }
    return unwrapText(res);
  }

  return {
    baseUrl: base,

    async health(): Promise<HealthDto> {
      return narrow(await send("/health"), isHealth, "health answer");
    },

    async listCategories(): Promise<CategoryDto[]> {
      return narrow(await send("/categories"), isCategoryList, "category list");
    },

    async listProducts(): Promise<ProductDto[]> {
      return narrow(await send("/products"), isProductList, "product list");
    },

    async updateProduct(id: number, input: NewProductDto): Promise<ProductDto> {
      const body = await send(`/products/${id}`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isProduct, "product");
    },

    async getSettings(): Promise<SettingsDto> {
      return narrow(await send("/settings"), isSettings, "settings");
    },

    /** The whole store block; a null clears that field. */
    async updateStore(input: StoreDto): Promise<StoreDto> {
      const body = await send("/settings/store", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isStore, "store block");
    },

    /** Appends a dated régime change; the answer is the whole settings page
     * again, since the change is current or planned depending on its day. */
    async changeRegime(input: RegimeChangeDto): Promise<SettingsDto> {
      const body = await send("/settings/regime", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isSettings, "settings");
    },

    /** The copies of the shop file the server keeps, newest first: the daily
     * ones, and the copies taken on the way into a restore, which are kept
     * under different rules and so travel in their own list. */
    async listBackups(): Promise<BackupsDto> {
      return narrow(await send("/backups"), isBackups, "backup list");
    },

    /** One more copy, taken now. The server names it and prunes the folder. */
    async createBackup(): Promise<BackupDto> {
      return narrow(await send("/backups", { method: "POST" }), isBackup, "backup");
    },

    /** Puts the shop file back from a copy. The name is the server's own, and
     * it is encoded rather than spliced, so a name that somehow carried a
     * separator reaches the server as one segment and is refused there. */
    async restoreBackup(name: string): Promise<RestoreDto> {
      const body = await send(`/backups/${encodeURIComponent(name)}/restore`, {
        method: "POST",
      });
      return narrow(body, isRestore, "restore answer");
    },

    /** Rings up the basket. The server dates the document and assigns the
     * number; neither is on the request. */
    async createSale(input: NewSaleDto): Promise<SaleDto> {
      const body = await send("/sales", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isSale, "sale");
    },

    async getSale(id: number): Promise<SaleDto> {
      return narrow(await send(`/sales/${id}`), isSale, "sale");
    },

    /** The 80 mm ticket for a sale, as the HTML page the core rendered.
     * The UI prints these bytes and never builds a document of its own:
     * the desktop and a server print the same paper (features.md §4). The
     * language is the one the till is being used in. */
    async getSaleTicket(id: number, lang: PrintLang): Promise<string> {
      return sendText(`/sales/${id}/ticket?lang=${lang}`);
    },

    /** Newest first, tickets only in M1. */
    async listSales(): Promise<SaleDto[]> {
      return narrow(await send("/sales"), isSaleList, "sale list");
    },

    async createProduct(input: NewProductDto): Promise<ProductDto> {
      const body = await send("/products", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isProduct, "product");
    },
  };
}
