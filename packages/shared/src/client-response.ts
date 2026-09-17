// What comes back off the wire, and what to do when it is a refusal.
//
// Apart from client.ts because it is the half that has no idea which routes
// exist: an envelope, an error built from it, and three unwrappers for the
// three shapes a route answers in (JSON, a document, a file). client.ts is
// the list of calls; this is what every one of them goes through on the way
// back.

import { z } from "zod";

import { apiErrorSchema } from "./schemas/error";
import type { PermissionDto } from "./generated/PermissionDto";

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
  /** Only on `locked_out`: how long before the till will look at a PIN
   * again. The sign-in screen counts it down rather than working it out. */
  readonly retryAfterSeconds?: number;
  /** Only on `forbidden`: the permission the route wanted. The screen says
   * which thing this role may not do without deciding that for itself
   * (architecture.md rule 2). */
  readonly permission?: PermissionDto;

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
      retryAfterSeconds?: number;
      permission?: PermissionDto;
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
    this.retryAfterSeconds = figures?.retryAfterSeconds;
    this.permission = figures?.permission;
  }
}

/** The error the envelope described, with the figures and the party fields
 * when it carried them. One place builds it, so both callers of `unwrap`
 * read a refusal the same way. */
export function apiError(body: z.output<typeof apiErrorSchema>, status: number): ApiError {
  return new ApiError(body.error.code, body.error.message, status, {
    balanceAfterCentimes: body.error.balance_after_centimes,
    creditLimitCentimes: body.error.credit_limit_centimes,
    field: body.error.field,
    outstandingCentimes: body.error.outstanding_centimes,
    partySide: body.error.party_side,
    missingIds: body.error.missing_ids,
    retryAfterSeconds: body.error.retry_after_seconds,
    permission: body.error.permission,
  });
}

export async function unwrap(res: Response): Promise<unknown> {
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
  const refusal = apiErrorSchema.safeParse(body);
  if (refusal.success) {
    throw apiError(refusal.data, res.status);
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

/** The four workbooks. Not a generated DTO either: the kind is a path
 * segment, so `crates/api/src/lib.rs` is the other half and an unknown one
 * answers 404. */
export type ExportKind = "products" | "sales" | "customers" | "suppliers";

/** A body that is a page, not JSON. Only the error path is JSON, and it is
 * the same envelope every other call answers with. */
export async function unwrapText(res: Response): Promise<string> {
  const text = await res.text();
  if (res.ok) return text;
  let body: unknown = null;
  try {
    body = JSON.parse(text);
  } catch {
    body = null;
  }
  const refusal = apiErrorSchema.safeParse(body);
  if (refusal.success) {
    throw apiError(refusal.data, res.status);
  }
  throw new ApiError("unreachable", `HTTP ${res.status}`, res.status);
}

/** A file the shop saves: the bytes and the name the server gave them. */
export interface Download {
  readonly blob: Blob;
  readonly filename: string;
}

/** The name out of `content-disposition`, or the fallback the caller gives.
 * A filename with a quote or a path separator in it is dropped rather than
 * cleaned: nothing this server sends has one, and a name that reached the
 * save dialog with a `/` in it would be somebody else's bug arriving here. */
export function filenameOf(header: string | null, fallback: string): string {
  if (header === null) return fallback;
  const match = /filename="([^"\/\\]+)"/.exec(header);
  return match === null ? fallback : match[1];
}

/** A body that is a file. The error path is the same JSON envelope every
 * other call answers with, so a refusal is read out of the blob as text. */
export async function unwrapFile(res: Response): Promise<Download> {
  if (res.ok) {
    return {
      blob: await res.blob(),
      filename: filenameOf(res.headers.get("content-disposition"), "export.xlsx"),
    };
  }
  let body: unknown = null;
  try {
    body = JSON.parse(await res.text());
  } catch {
    body = null;
  }
  const refusal = apiErrorSchema.safeParse(body);
  if (refusal.success) {
    throw apiError(refusal.data, res.status);
  }
  throw new ApiError("unreachable", `HTTP ${res.status}`, res.status);
}

/** The answer, parsed by the schema of the DTO the route promises. A body the
 * schema refuses raises the same `bad_response` the hand guards raised: the
 * screens read that code and a half-typed object never reaches them. */
export function narrow<Schema extends z.ZodType>(
  body: unknown,
  schema: Schema,
  what: string,
): z.output<Schema> {
  const parsed = schema.safeParse(body);
  if (parsed.success) return parsed.data;
  throw new ApiError("bad_response", `the server sent an unexpected ${what}`, 0);
}
