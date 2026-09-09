// The one HTTP client. The desktop webview, the browser preview and, later,
// the phone all use it, so none of them knows which mode it is in (rule 1).
//
// No `as` casts: every response is narrowed by a guard, so a server that
// answers the wrong shape raises a translatable error instead of leaking a
// half-typed object into the UI.

import type { ApiErrorDto } from "./generated/ApiErrorDto";
import type { BackupDto } from "./generated/BackupDto";
import type { BackupsDto } from "./generated/BackupsDto";
import type { AdjustmentDto } from "./generated/AdjustmentDto";
import type { CategoryDto } from "./generated/CategoryDto";
import type { CustomerDto } from "./generated/CustomerDto";
import type { CustomerLedgerDto } from "./generated/CustomerLedgerDto";
import type { CustomerWriteDto } from "./generated/CustomerWriteDto";
import type { DebtEntryDto } from "./generated/DebtEntryDto";
import type { DebtKindDto } from "./generated/DebtKindDto";
import type { CustomerPaymentsDto } from "./generated/CustomerPaymentsDto";
import type { NewCustomerDto } from "./generated/NewCustomerDto";
import type { NewPaymentDto } from "./generated/NewPaymentDto";
import type { PaymentAllocationDto } from "./generated/PaymentAllocationDto";
import type { PaymentDto } from "./generated/PaymentDto";
import type { PaymentMethodDto } from "./generated/PaymentMethodDto";
import type { PartyKindDto } from "./generated/PartyKindDto";
import type { HealthDto } from "./generated/HealthDto";
import type { DocumentKindDto } from "./generated/DocumentKindDto";
import type { DocumentStatusDto } from "./generated/DocumentStatusDto";
import type { NewProductDto } from "./generated/NewProductDto";
import type { NewSaleDto } from "./generated/NewSaleDto";
import type { PaymentModeDto } from "./generated/PaymentModeDto";
import type { SaleWarningDto } from "./generated/SaleWarningDto";
import type { SaleDto } from "./generated/SaleDto";
import type { SaleKindDto } from "./generated/SaleKindDto";
import type { SaleLineDto } from "./generated/SaleLineDto";
import type { SaleCancelEffectDto } from "./generated/SaleCancelEffectDto";
import type { SaleCancellationDto } from "./generated/SaleCancellationDto";
import type { NewAvoirDto } from "./generated/NewAvoirDto";
import type { CancelDocumentDto } from "./generated/CancelDocumentDto";
import type { SaleBalanceDto } from "./generated/SaleBalanceDto";
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

/** An error the server described. `code` is a translation key.
 *
 * `balanceAfterCentimes` and `creditLimitCentimes` are on a `credit_limit`
 * refusal and on nothing else: the till has to say by how much a limit was
 * passed, and working that out on the screen would be a second answer to
 * what a customer owes. `field` and `outstandingCentimes` are the same
 * bargain on a payment above the debt: the fiche says what is actually owed
 * because the server said it. Undefined everywhere else, never zero. */
export class ApiError extends Error {
  readonly code: string;
  readonly status: number;
  readonly balanceAfterCentimes?: number;
  readonly creditLimitCentimes?: number;
  readonly field?: string;
  readonly outstandingCentimes?: number;
  /** Only on `party_ids`: which half of the facture is short and of which
   * identifiers. The server decides both; a screen shows them and works out
   * neither (architecture.md rule 2). */
  readonly partySide?: string;
  readonly missingIds?: readonly string[];

  constructor(
    code: string,
    message: string,
    status: number,
    figures?: {
      balanceAfterCentimes?: number;
      creditLimitCentimes?: number;
      field?: string;
      outstandingCentimes?: number;
      partySide?: string;
      missingIds?: readonly string[];
    },
  ) {
    super(message);
    this.name = "ApiError";
    this.code = code;
    this.status = status;
    this.balanceAfterCentimes = figures?.balanceAfterCentimes;
    this.creditLimitCentimes = figures?.creditLimitCentimes;
    this.field = figures?.field;
    this.outstandingCentimes = figures?.outstandingCentimes;
    this.partySide = figures?.partySide;
    this.missingIds = figures?.missingIds;
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
    isRecord(error) &&
    typeof error.code === "string" &&
    typeof error.message === "string" &&
    isOptionalExactInteger(error.balance_after_centimes) &&
    isOptionalExactInteger(error.credit_limit_centimes) &&
    (error.field === undefined || typeof error.field === "string") &&
    isOptionalExactInteger(error.outstanding_centimes) &&
    (error.party_side === undefined || typeof error.party_side === "string") &&
    (error.missing_ids === undefined ||
      (Array.isArray(error.missing_ids) && error.missing_ids.every((v) => typeof v === "string")))
  );
}

/** The error the envelope described, with the figures and the party fields
 * when it carried them. One place builds it, so both callers of `unwrap`
 * read a refusal the same way. */
function apiError(body: ApiErrorDto, status: number): ApiError {
  return new ApiError(body.error.code, body.error.message, status, {
    balanceAfterCentimes: body.error.balance_after_centimes,
    creditLimitCentimes: body.error.credit_limit_centimes,
    field: body.error.field,
    outstandingCentimes: body.error.outstanding_centimes,
    partySide: body.error.party_side,
    missingIds: body.error.missing_ids,
  });
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

/** A field the server leaves out rather than sending as null. Absent is an
 * answer here: only a credit refusal carries the two amounts. */
function isOptionalExactInteger(value: unknown): value is number | undefined {
  return value === undefined || isExactInteger(value);
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

const SALE_WARNINGS: readonly SaleWarningDto[] = ["near_limit"];

function isNullableSaleWarning(value: unknown): value is SaleWarningDto | null {
  return value === null || (typeof value === "string" && SALE_WARNINGS.some((w) => w === value));
}

const DOCUMENT_KINDS: readonly DocumentKindDto[] = [
  "ticket",
  "facture",
  "proforma",
  "bon_de_livraison",
  "avoir",
  "bon_de_reception",
  "quittance",
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
    isExactInteger(value.line_total_centimes) &&
    isNullableNumber(value.ref_line_id)
  );
}

/** What a cancellation left on the document it annulled, or nothing at all
 *  on one that still stands. Whole or absent: a screen never has to ask
 *  whether the date is there while the reason is not. */
function isSaleCancellation(value: unknown): value is SaleCancellationDto | null {
  return (
    value === null ||
    (isRecord(value) &&
      typeof value.cancelled_at === "string" &&
      typeof value.cancelled_by === "number" &&
      typeof value.reason === "string" &&
      isNullableNumber(value.avoir_document_id))
  );
}

/** What cancelling this document would do, or nothing at all when the answer
 *  was a list rather than a read of one document. Checked shape by shape,
 *  because the amount belongs to exactly one of them: a screen that read an
 *  amount off `stock_back` would be showing a figure the server never sent. */
function isSaleCancelEffect(value: unknown): value is SaleCancelEffectDto | null {
  if (value === null) return true;
  if (!isRecord(value)) return false;
  switch (value.effect) {
    case "nothing_to_reverse":
    case "stock_back":
      return true;
    case "stock_back_and_avoir":
      return isExactInteger(value.amount_centimes);
    default:
      return false;
  }
}

function isSaleTva(value: unknown): value is SaleTvaDto {
  return (
    isRecord(value) &&
    typeof value.rate_bps === "number" &&
    isExactInteger(value.base_centimes) &&
    isExactInteger(value.amount_centimes)
  );
}

/** The balance triple, or null on a document that names no customer. Three
 *  exact integers or nothing: two of three would be a closing balance its
 *  own opening balance does not explain. */
function isSaleBalance(value: unknown): value is SaleBalanceDto | null {
  return (
    value === null ||
    (isRecord(value) &&
      isExactInteger(value.old_balance_centimes) &&
      isExactInteger(value.remaining_debt_centimes) &&
      isExactInteger(value.total_debt_centimes))
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
    typeof value.printed_number === "string" &&
    typeof value.issued_at === "string" &&
    typeof value.user_id === "number" &&
    isRegime(value.regime) &&
    isPaymentMode(value.payment_mode) &&
    isStore(value.seller) &&
    isNullableNumber(value.customer_id) &&
    isNullableNumber(value.ref_document_id) &&
    isNullableString(value.buyer_name) &&
    isSaleBalance(value.balance) &&
    isSaleTotals(value.totals) &&
    Array.isArray(value.tva) &&
    value.tva.every(isSaleTva) &&
    isNullableExactInteger(value.tendered_centimes) &&
    isNullableExactInteger(value.change_centimes) &&
    isDocumentStatus(value.status) &&
    isSaleCancellation(value.cancellation) &&
    isSaleCancelEffect(value.cancel_effect) &&
    Array.isArray(value.lines) &&
    value.lines.every(isSaleLine) &&
    isNullableSaleWarning(value.warning)
  );
}

function isSaleList(value: unknown): value is SaleDto[] {
  return Array.isArray(value) && value.every(isSale);
}

const PARTY_KINDS: readonly PartyKindDto[] = ["company", "consumer"];

function isPartyKind(value: unknown): value is PartyKindDto {
  return typeof value === "string" && PARTY_KINDS.some((k) => k === value);
}

const DEBT_KINDS: readonly DebtKindDto[] = [
  "opening",
  "sale",
  "payment",
  "avoir",
  "adjustment",
];

function isDebtKind(value: unknown): value is DebtKindDto {
  return typeof value === "string" && DEBT_KINDS.some((k) => k === value);
}

/** A fiche, with the balance the core summed. Every amount is checked as an
 *  exact integer: a debt JSON.parse had to round is refused rather than
 *  shown to a shop. */
export function isCustomer(value: unknown): value is CustomerDto {
  return (
    isRecord(value) &&
    typeof value.id === "number" &&
    typeof value.shop_id === "number" &&
    typeof value.name === "string" &&
    isPartyKind(value.party_kind) &&
    isNullableString(value.phone) &&
    isNullableString(value.address) &&
    isNullableString(value.rc) &&
    isNullableString(value.nif) &&
    isNullableString(value.nis) &&
    isNullableString(value.ai) &&
    isNullableExactInteger(value.credit_limit_centimes) &&
    isNullableExactInteger(value.warn_threshold_centimes) &&
    isNullableString(value.notes) &&
    typeof value.active === "boolean" &&
    isExactInteger(value.balance_centimes)
  );
}

function isCustomerList(value: unknown): value is CustomerDto[] {
  return Array.isArray(value) && value.every(isCustomer);
}

export function isDebtEntry(value: unknown): value is DebtEntryDto {
  return (
    isRecord(value) &&
    typeof value.id === "number" &&
    typeof value.customer_id === "number" &&
    isNullableNumber(value.document_id) &&
    isDebtKind(value.kind) &&
    isExactInteger(value.debit_centimes) &&
    isExactInteger(value.credit_centimes) &&
    isExactInteger(value.balance_after_centimes) &&
    typeof value.user_id === "number" &&
    isNullableString(value.note) &&
    typeof value.created_at === "string"
  );
}

const PAYMENT_METHODS: readonly PaymentMethodDto[] = ["cash", "card"];

function isPaymentMethod(value: unknown): value is PaymentMethodDto {
  return typeof value === "string" && PAYMENT_METHODS.some((m) => m === value);
}

function isPaymentAllocation(value: unknown): value is PaymentAllocationDto {
  return (
    isRecord(value) &&
    typeof value.document_id === "number" &&
    isExactInteger(value.amount_centimes)
  );
}

export function isPayment(value: unknown): value is PaymentDto {
  return (
    isRecord(value) &&
    typeof value.ledger_id === "number" &&
    typeof value.customer_id === "number" &&
    isExactInteger(value.amount_centimes) &&
    (value.payment_mode === null || isPaymentMethod(value.payment_mode)) &&
    isNullableString(value.note) &&
    isExactInteger(value.balance_after_centimes) &&
    Array.isArray(value.allocations) &&
    value.allocations.every(isPaymentAllocation) &&
    typeof value.created_at === "string"
  );
}

export function isCustomerPayments(value: unknown): value is CustomerPaymentsDto {
  return (
    isRecord(value) &&
    typeof value.customer_id === "number" &&
    isExactInteger(value.balance_centimes) &&
    Array.isArray(value.payments) &&
    value.payments.every(isPayment)
  );
}

export function isCustomerLedger(value: unknown): value is CustomerLedgerDto {
  return (
    isRecord(value) &&
    typeof value.customer_id === "number" &&
    isExactInteger(value.balance_centimes) &&
    Array.isArray(value.entries) &&
    value.entries.every(isDebtEntry)
  );
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
    throw apiError(body, res.status);
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

/** The sheet a facture is laid out for. It changes the `@page size` of the
 * page the core renders and nothing else, so an A5 facture is the same
 * facture on a smaller sheet (features.md §4). */
export type PrintPaper = "a4" | "a5";

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
    throw apiError(body, res.status);
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

    /** The A4 or A5 facture for a sale, as the HTML page the core rendered.
     * The same contract as the ticket, plus the sheet: the UI prints these
     * bytes and never lays a document out itself. The id has to name a
     * facture; a ticket's id is a 404, because a ticket is its own paper. */
    async getSaleFacture(id: number, lang: PrintLang, paper: PrintPaper): Promise<string> {
      return sendText(`/sales/${id}/facture?lang=${lang}&paper=${paper}`);
    },

    /** Newest first, every kind the till issues. `kind` narrows it to one
     * series: the day's till roll asks for `ticket`, a documents screen for
     * `facture`, and a screen that wants both asks for neither. */
    async listSales(kind?: SaleKindDto): Promise<SaleDto[]> {
      const query = kind === undefined ? "" : `?kind=${kind}`;
      return narrow(await send(`/sales${query}`), isSaleList, "sale list");
    },

    /** Writes a credit note against the facture named. `lines` left out is
     * the whole of what is left on it, which is what the "avoir the lot"
     * button sends; a list credits the lines it names and no more of each
     * than the facture has left. Every rule is the core's. */
    async createAvoir(id: number, input: NewAvoirDto): Promise<SaleDto> {
      const body = await send(`/sales/${id}/avoir`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isSale, "avoir");
    },

    /** Every avoir written against one facture, oldest first. A ticket's id
     * is a 404 rather than an empty list: an empty list would read as "this
     * facture has no credit notes". */
    async listAvoirs(id: number): Promise<SaleDto[]> {
      return narrow(await send(`/sales/${id}/avoirs`), isSaleList, "avoir list");
    },

    /** Annuls a document and hands it back carrying the block that says
     * when, by whom, why and with which avoir. It keeps its number. */
    async cancelSale(id: number, input: CancelDocumentDto): Promise<SaleDto> {
      const body = await send(`/sales/${id}/cancel`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isSale, "sale");
    },

    /** The shop's customers, the active ones first. `search` is a piece of a
     * name or a phone number; blank asks for the whole list, which is what an
     * emptied search box means. */
    async listCustomers(search?: string): Promise<CustomerDto[]> {
      const trimmed = search === undefined ? "" : search.trim();
      const query = trimmed === "" ? "" : `?q=${encodeURIComponent(trimmed).replace(/%20/g, "+")}`;
      return narrow(await send(`/customers${query}`), isCustomerList, "customer list");
    },

    async getCustomer(id: number): Promise<CustomerDto> {
      return narrow(await send(`/customers/${id}`), isCustomer, "customer");
    },

    /** Opens a fiche, and with it the opening debt when the shop is carrying
     * one over. The opening debt is only on the create: a wrong one is
     * corrected by an adjustment, never by editing the fiche. */
    async createCustomer(input: NewCustomerDto): Promise<CustomerDto> {
      const body = await send("/customers", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isCustomer, "customer");
    },

    /** The whole fiche; a null clears that field. */
    async updateCustomer(id: number, input: CustomerWriteDto): Promise<CustomerDto> {
      const body = await send(`/customers/${id}`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isCustomer, "customer");
    },

    /** The movements newest first, each with the balance as of itself, and
     * the balance they sum to. Both are the core's; nothing here adds a
     * column up. */
    async customerLedger(id: number): Promise<CustomerLedgerDto> {
      return narrow(await send(`/customers/${id}/ledger`), isCustomerLedger, "customer ledger");
    },

    /** Corrects a balance by writing a movement: positive raises the debt,
     * negative lowers it. The answer is the whole ledger again. */
    async adjustCustomerDebt(id: number, input: AdjustmentDto): Promise<CustomerLedgerDto> {
      const body = await send(`/customers/${id}/adjustments`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isCustomerLedger, "customer ledger");
    },

    /** The customer's payments, newest first, each with the documents it
     * settled. The balance in the envelope is the whole ledger's, not the
     * newest payment's: a sale written after the last payment moved it. */
    async customerPayments(id: number): Promise<CustomerPaymentsDto> {
      return narrow(
        await send(`/customers/${id}/payments`),
        isCustomerPayments,
        "customer payments",
      );
    },

    /** Money against a debt. The server settles the oldest documents first
     * and refuses a payment above what the customer owes; the answer is the
     * whole list of payments again. */
    async payCustomer(id: number, input: NewPaymentDto): Promise<CustomerPaymentsDto> {
      const body = await send(`/customers/${id}/payments`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, isCustomerPayments, "customer payments");
    },

    /** The statement of account over a range of days, as the HTML page the
     * core rendered. The UI prints these bytes and never builds a document of
     * its own (features.md §4). The days are `YYYY-MM-DD`. */
    async customerStatement(
      id: number,
      from: string,
      to: string,
      lang: PrintLang,
    ): Promise<string> {
      const query = new URLSearchParams({ from, to, lang });
      return sendText(`/customers/${id}/statement?${query.toString()}`);
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
