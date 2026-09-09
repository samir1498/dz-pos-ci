// The one HTTP client. The desktop webview, the browser preview and, later,
// the phone all use it, so none of them knows which mode it is in (rule 1).
//
// No `as` casts: every response is narrowed by a guard, so a server that
// answers the wrong shape raises a translatable error instead of leaking a
// half-typed object into the UI.

import type { ApiErrorDto } from "./generated/ApiErrorDto";
import type { CategoryDto } from "./generated/CategoryDto";
import type { HealthDto } from "./generated/HealthDto";
import type { NewProductDto } from "./generated/NewProductDto";
import type { ProductDto } from "./generated/ProductDto";
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
