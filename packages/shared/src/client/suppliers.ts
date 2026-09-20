// Supplier fiches, their ledgers, payments and the JSON statement.

import { z } from "zod";

import type { AdjustmentDto } from "../generated/AdjustmentDto";
import type { CloseSupplierDto } from "../generated/CloseSupplierDto";
import type { NewPaymentDto } from "../generated/NewPaymentDto";
import type { NewSupplierDto } from "../generated/NewSupplierDto";
import type { SupplierDto } from "../generated/SupplierDto";
import type { SupplierLedgerDto } from "../generated/SupplierLedgerDto";
import type { SupplierStatementDto } from "../generated/SupplierStatementDto";
import type { SupplierWriteDto } from "../generated/SupplierWriteDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import {
  supplierLedgerSchema,
  supplierSchema,
  supplierStatementSchema,
} from "../schemas/supplier";

export function suppliersClient({ send }: Transport) {
  return {
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
  };
}
