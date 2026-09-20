// Customer fiches, their ledgers, payments and the two printed statements.

import { z } from "zod";

import type { AdjustmentDto } from "../generated/AdjustmentDto";
import type { CustomerDto } from "../generated/CustomerDto";
import type { CustomerLedgerDto } from "../generated/CustomerLedgerDto";
import type { CustomerPaymentsDto } from "../generated/CustomerPaymentsDto";
import type { CustomerWriteDto } from "../generated/CustomerWriteDto";
import type { NewCustomerDto } from "../generated/NewCustomerDto";
import type { NewPaymentDto } from "../generated/NewPaymentDto";
import { narrow, type PrintLang } from "../client-response";
import type { Transport } from "../client";
import {
  customerLedgerSchema,
  customerPaymentsSchema,
  customerSchema,
} from "../schemas/customer";

export function customersClient({ send, sendText }: Transport) {
  return {
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
  };
}
