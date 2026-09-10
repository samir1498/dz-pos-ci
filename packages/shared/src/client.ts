// The one HTTP client. The desktop webview, the browser preview and, later,
// the phone all use it, so none of them knows which mode it is in (rule 1).
//
// No `as` casts: every answer is parsed by the zod schema of its DTO
// (`./schemas`), so a server that answers the wrong shape raises a
// translatable error instead of leaking a half-typed object into the UI. The
// schemas are pinned to `./generated` in both directions, so the check the
// client runs cannot drift from the Rust struct it is checking.

import { z } from "zod";

import type { AdjustmentDto } from "./generated/AdjustmentDto";
import type { BackupDto } from "./generated/BackupDto";
import type { BackupsDto } from "./generated/BackupsDto";
import type { CategoryDto } from "./generated/CategoryDto";
import type { ClockDto } from "./generated/ClockDto";
import type { CustomerDto } from "./generated/CustomerDto";
import type { CustomerLedgerDto } from "./generated/CustomerLedgerDto";
import type { CustomerPaymentsDto } from "./generated/CustomerPaymentsDto";
import type { CustomerWriteDto } from "./generated/CustomerWriteDto";
import type { CashPositionDto } from "./generated/CashPositionDto";
import type { ExpenseCategoryDto } from "./generated/ExpenseCategoryDto";
import type { ExpenseDto } from "./generated/ExpenseDto";
import type { ExpensesDto } from "./generated/ExpensesDto";
import type { NewExpenseDto } from "./generated/NewExpenseDto";
import type { HealthDto } from "./generated/HealthDto";
import type { NewAvoirDto } from "./generated/NewAvoirDto";
import type { CancelDocumentDto } from "./generated/CancelDocumentDto";
import type { NewCustomerDto } from "./generated/NewCustomerDto";
import type { NewPaymentDto } from "./generated/NewPaymentDto";
import type { NewProductDto } from "./generated/NewProductDto";
import type { NewSaleDto } from "./generated/NewSaleDto";
import type { ProductDto } from "./generated/ProductDto";
import type { RegimeChangeDto } from "./generated/RegimeChangeDto";
import type { RestoreDto } from "./generated/RestoreDto";
import type { SaleDto } from "./generated/SaleDto";
import type { SaleKindDto } from "./generated/SaleKindDto";
import type { SettingsDto } from "./generated/SettingsDto";
import type { StoreDto } from "./generated/StoreDto";
import type { CloseSupplierDto } from "./generated/CloseSupplierDto";
import type { LastStockRecountDto } from "./generated/LastStockRecountDto";
import type { StockRecountDto } from "./generated/StockRecountDto";
import type { NewSupplierDto } from "./generated/NewSupplierDto";
import type { SupplierDto } from "./generated/SupplierDto";
import type { SupplierLedgerDto } from "./generated/SupplierLedgerDto";
import type { SupplierStatementDto } from "./generated/SupplierStatementDto";
import type { SupplierWriteDto } from "./generated/SupplierWriteDto";
import { categorySchema, productSchema } from "./schemas/catalogue";
import {
  customerLedgerSchema,
  customerPaymentsSchema,
  customerSchema,
} from "./schemas/customer";
import { apiErrorSchema } from "./schemas/error";
import {
  cashPositionSchema,
  expenseCategorySchema,
  expenseSchema,
  expensesSchema,
} from "./schemas/expense";
import {
  supplierLedgerSchema,
  supplierSchema,
  supplierStatementSchema,
} from "./schemas/supplier";
import { saleSchema } from "./schemas/sale";
import { lastStockRecountSchema, stockRecountSchema } from "./schemas/stock";
import {
  backupSchema,
  backupsSchema,
  clockSchema,
  healthSchema,
  restoreSchema,
  settingsSchema,
  storeSchema,
} from "./schemas/settings";

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

/** The error the envelope described, with the figures and the party fields
 * when it carried them. One place builds it, so both callers of `unwrap`
 * read a refusal the same way. */
function apiError(body: z.output<typeof apiErrorSchema>, status: number): ApiError {
  return new ApiError(body.error.code, body.error.message, status, {
    balanceAfterCentimes: body.error.balance_after_centimes,
    creditLimitCentimes: body.error.credit_limit_centimes,
    field: body.error.field,
    outstandingCentimes: body.error.outstanding_centimes,
    partySide: body.error.party_side,
    missingIds: body.error.missing_ids,
  });
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
  const refusal = apiErrorSchema.safeParse(body);
  if (refusal.success) {
    throw apiError(refusal.data, res.status);
  }
  throw new ApiError("unreachable", `HTTP ${res.status}`, res.status);
}

/** The answer, parsed by the schema of the DTO the route promises. A body the
 * schema refuses raises the same `bad_response` the hand guards raised: the
 * screens read that code and a half-typed object never reaches them. */
function narrow<Schema extends z.ZodType>(
  body: unknown,
  schema: Schema,
  what: string,
): z.output<Schema> {
  const parsed = schema.safeParse(body);
  if (parsed.success) return parsed.data;
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
      return narrow(await send("/health"), healthSchema, "health answer");
    },

    /** The day the shop is on. Asked for rather than read off the machine:
     * the core dates documents on Algeria's calendar and a browser in
     * another zone would be a day out either way. */
    async clock(): Promise<ClockDto> {
      return narrow(await send("/clock"), clockSchema, "clock answer");
    },

    async listCategories(): Promise<CategoryDto[]> {
      return narrow(await send("/categories"), z.array(categorySchema), "category list");
    },

    async listProducts(): Promise<ProductDto[]> {
      return narrow(await send("/products"), z.array(productSchema), "product list");
    },

    async updateProduct(id: number, input: NewProductDto): Promise<ProductDto> {
      const body = await send(`/products/${id}`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, productSchema, "product");
    },

    async getSettings(): Promise<SettingsDto> {
      return narrow(await send("/settings"), settingsSchema, "settings");
    },

    /** The whole store block; a null clears that field. */
    async updateStore(input: StoreDto): Promise<StoreDto> {
      const body = await send("/settings/store", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, storeSchema, "store block");
    },

    /** Appends a dated régime change; the answer is the whole settings page
     * again, since the change is current or planned depending on its day. */
    async changeRegime(input: RegimeChangeDto): Promise<SettingsDto> {
      const body = await send("/settings/regime", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, settingsSchema, "settings");
    },

    /** The copies of the shop file the server keeps, newest first: the daily
     * ones, and the copies taken on the way into a restore, which are kept
     * under different rules and so travel in their own list. */
    async listBackups(): Promise<BackupsDto> {
      return narrow(await send("/backups"), backupsSchema, "backup list");
    },

    /** One more copy, taken now. The server names it and prunes the folder. */
    async createBackup(): Promise<BackupDto> {
      return narrow(await send("/backups", { method: "POST" }), backupSchema, "backup");
    },

    /** Puts the shop file back from a copy. The name is the server's own, and
     * it is encoded rather than spliced, so a name that somehow carried a
     * separator reaches the server as one segment and is refused there. */
    async restoreBackup(name: string): Promise<RestoreDto> {
      const body = await send(`/backups/${encodeURIComponent(name)}/restore`, {
        method: "POST",
      });
      return narrow(body, restoreSchema, "restore answer");
    },

    /** Rings up the basket. The server dates the document and assigns the
     * number; neither is on the request. */
    async createSale(input: NewSaleDto): Promise<SaleDto> {
      const body = await send("/sales", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, saleSchema, "sale");
    },

    async getSale(id: number): Promise<SaleDto> {
      return narrow(await send(`/sales/${id}`), saleSchema, "sale");
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
      return narrow(await send(`/sales${query}`), z.array(saleSchema), "sale list");
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
      return narrow(body, saleSchema, "avoir");
    },

    /** Every avoir written against one facture, oldest first. A ticket's id
     * is a 404 rather than an empty list: an empty list would read as "this
     * facture has no credit notes". */
    async listAvoirs(id: number): Promise<SaleDto[]> {
      return narrow(await send(`/sales/${id}/avoirs`), z.array(saleSchema), "avoir list");
    },

    /** Annuls a document and hands it back carrying the block that says
     * when, by whom, why and with which avoir. It keeps its number. */
    async cancelSale(id: number, input: CancelDocumentDto): Promise<SaleDto> {
      const body = await send(`/sales/${id}/cancel`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, saleSchema, "sale");
    },

    /** The shop's customers, the active ones first. `search` is a piece of a
     * name or a phone number; blank asks for the whole list, which is what an
     * emptied search box means. */
    async listCustomers(search?: string): Promise<CustomerDto[]> {
      const trimmed = search === undefined ? "" : search.trim();
      const query = trimmed === "" ? "" : `?q=${encodeURIComponent(trimmed).replace(/%20/g, "+")}`;
      return narrow(await send(`/customers${query}`), z.array(customerSchema), "customer list");
    },

    async getCustomer(id: number): Promise<CustomerDto> {
      return narrow(await send(`/customers/${id}`), customerSchema, "customer");
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
      return narrow(body, customerSchema, "customer");
    },

    /** The whole fiche; a null clears that field. */
    async updateCustomer(id: number, input: CustomerWriteDto): Promise<CustomerDto> {
      const body = await send(`/customers/${id}`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, customerSchema, "customer");
    },

    /** The movements newest first, each with the balance as of itself, and
     * the balance they sum to. Both are the core's; nothing here adds a
     * column up. */
    async customerLedger(id: number): Promise<CustomerLedgerDto> {
      return narrow(await send(`/customers/${id}/ledger`), customerLedgerSchema, "customer ledger");
    },

    /** Corrects a balance by writing a movement: positive raises the debt,
     * negative lowers it. The answer is the whole ledger again. */
    async adjustCustomerDebt(id: number, input: AdjustmentDto): Promise<CustomerLedgerDto> {
      const body = await send(`/customers/${id}/adjustments`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, customerLedgerSchema, "customer ledger");
    },

    /** The customer's payments, newest first, each with the documents it
     * settled. The balance in the envelope is the whole ledger's, not the
     * newest payment's: a sale written after the last payment moved it. */
    async customerPayments(id: number): Promise<CustomerPaymentsDto> {
      return narrow(
        await send(`/customers/${id}/payments`),
        customerPaymentsSchema,
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
      return narrow(body, customerPaymentsSchema, "customer payments");
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

    /** The 80 mm debt slip, as the HTML page the core rendered: what the
     * customer owes now and the newest movements behind it. No range, because
     * the slip is about today rather than about a period, and the server's
     * clock dates it. */
    async customerDebtSlip(id: number, lang: PrintLang): Promise<string> {
      const query = new URLSearchParams({ lang });
      return sendText(`/customers/${id}/debt-slip?${query.toString()}`);
    },

    /** The shop's suppliers, the ones it still buys from first. `search` is a
     * piece of a name or a phone number; blank asks for the whole list, which
     * is what an emptied search box means. */
    async listSuppliers(search?: string): Promise<SupplierDto[]> {
      const trimmed = search === undefined ? "" : search.trim();
      const query = trimmed === "" ? "" : `?q=${encodeURIComponent(trimmed).replace(/%20/g, "+")}`;
      return narrow(await send(`/suppliers${query}`), z.array(supplierSchema), "supplier list");
    },

    async getSupplier(id: number): Promise<SupplierDto> {
      return narrow(await send(`/suppliers/${id}`), supplierSchema, "supplier");
    },

    /** Opens a fiche, and with it the debt the shop was already carrying to
     * this supplier. The opening debt is only on the create: a wrong one is
     * corrected by an adjustment, never by editing the fiche. */
    async createSupplier(input: NewSupplierDto): Promise<SupplierDto> {
      const body = await send("/suppliers", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, supplierSchema, "supplier");
    },

    /** The whole fiche; a null clears that field. */
    async updateSupplier(id: number, input: SupplierWriteDto): Promise<SupplierDto> {
      const body = await send(`/suppliers/${id}`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, supplierSchema, "supplier");
    },

    /** Stops the shop buying from this supplier. A fiche whose account is
     * still open is refused without a reason, and the reason goes into the
     * audit log beside the balance. */
    async closeSupplier(id: number, input: CloseSupplierDto): Promise<SupplierDto> {
      const body = await send(`/suppliers/${id}/close`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, supplierSchema, "supplier");
    },

    /** The movements newest first, each with the balance as of itself and,
     * on a payment, the orders it settled. Both figures are the core's. */
    async supplierLedger(id: number): Promise<SupplierLedgerDto> {
      return narrow(await send(`/suppliers/${id}/ledger`), supplierLedgerSchema, "supplier ledger");
    },

    /** Money to a supplier. The server settles the oldest orders first and
     * refuses a payment above what the shop owes; the answer is the whole
     * ledger again. */
    async paySupplier(id: number, input: NewPaymentDto): Promise<SupplierLedgerDto> {
      const body = await send(`/suppliers/${id}/payments`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, supplierLedgerSchema, "supplier ledger");
    },

    /** Corrects a balance by writing a movement: positive raises what the
     * shop owes, negative lowers it. The answer is the whole ledger again. */
    async adjustSupplierDebt(id: number, input: AdjustmentDto): Promise<SupplierLedgerDto> {
      const body = await send(`/suppliers/${id}/adjustments`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, supplierLedgerSchema, "supplier ledger");
    },

    /** The supplier's account between two days, both included. JSON and not
     * a rendered page: the printed statement is a paper a customer is
     * handed, and the shop's own copy of what it owes is a screen. */
    async supplierStatement(id: number, from: string, to: string): Promise<SupplierStatementDto> {
      const query = new URLSearchParams({ from, to });
      return narrow(
        await send(`/suppliers/${id}/statement?${query.toString()}`),
        supplierStatementSchema,
        "supplier statement",
      );
    },

    /** One month of expenses and what it came to, `YYYY-MM` on the shop's
     * calendar. The total is the server's: a screen that added the rows up
     * would be a second answer to the same question. */
    async listExpenses(month: string): Promise<ExpensesDto> {
      const query = new URLSearchParams({ month });
      return narrow(await send(`/expenses?${query.toString()}`), expensesSchema, "expenses");
    },

    /** The seven seeded categories, in the order the screen lists them. Each
     * carries an i18n key, and the label comes from the app's own language
     * files. */
    async listExpenseCategories(): Promise<ExpenseCategoryDto[]> {
      return narrow(
        await send("/expense-categories"),
        z.array(expenseCategorySchema),
        "expense categories",
      );
    },

    async createExpense(input: NewExpenseDto): Promise<ExpenseDto> {
      const body = await send("/expenses", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, expenseSchema, "expense");
    },

    /** The cash position over one day or one month, never both: the server
     * refuses a call that names the two, so the range a figure covers is
     * always the one that was asked for. */
    async cashPosition(period: { day: string } | { month: string }): Promise<CashPositionDto> {
      const query = new URLSearchParams(period);
      return narrow(await send(`/cash?${query.toString()}`), cashPositionSchema, "cash position");
    },

    /** The day the shop last recounted its stock and what that day put
     * right. Null day means it has never run: the daily loop marks the first
     * one on the first wake after the app is launched. */
    async lastStockRecount(): Promise<LastStockRecountDto> {
      return narrow(await send("/stock/recount"), lastStockRecountSchema, "last stock recount");
    },

    /** Recounts now, whatever the marker says. The server compares every
     * product's cached quantity with its ledger and writes the ledger back
     * over the ones that disagree, so the answer is what it corrected. */
    async recountStock(): Promise<StockRecountDto> {
      const body = await send("/stock/recount", { method: "POST" });
      return narrow(body, stockRecountSchema, "stock recount");
    },

    async createProduct(input: NewProductDto): Promise<ProductDto> {
      const body = await send("/products", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, productSchema, "product");
    },
  };
}
