// Purchase orders: listing, one with its lines, writing one, delivering
// against it, returns, and the two ways an order closes short.

import { z } from "zod";

import type { CloseOrderDto } from "../generated/CloseOrderDto";
import type { NewPurchaseDto } from "../generated/NewPurchaseDto";
import type { NewReceiptDto } from "../generated/NewReceiptDto";
import type { PurchaseDetailDto } from "../generated/PurchaseDetailDto";
import type { PurchaseDto } from "../generated/PurchaseDto";
import type { PurchaseStatusDto } from "../generated/PurchaseStatusDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { purchaseDetailSchema, purchaseSchema } from "../schemas/purchase";

export function purchasesClient({ send }: Transport) {
  return {
    /** The shop's orders, newest first, narrowed to one state or one
     * supplier when the screen asks for it. A row per order and no lines:
     * the lines are what `getPurchase` answers. */
    async listPurchases(filters?: {
      status?: PurchaseStatusDto;
      supplierId?: number;
    }): Promise<PurchaseDto[]> {
      const query = new URLSearchParams();
      if (filters?.status !== undefined) query.set("status", filters.status);
      if (filters?.supplierId !== undefined) query.set("supplier_id", String(filters.supplierId));
      const suffix = query.toString() === "" ? "" : `?${query.toString()}`;
      return narrow(await send(`/purchases${suffix}`), z.array(purchaseSchema), "purchase list");
    },

    /** One order with its lines and every delivery against it. */
    async getPurchase(id: number): Promise<PurchaseDetailDto> {
      return narrow(await send(`/purchases/${id}`), purchaseDetailSchema, "purchase");
    },

    /** Writes the order. With `receive_now` the whole delivery is written in
     * the same transaction, which is the common case: the goods came with
     * the paper. Stock and the supplier's debt move on the delivery and
     * never on the order alone. */
    async createPurchase(input: NewPurchaseDto): Promise<PurchaseDetailDto> {
      const body = await send("/purchases", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, purchaseDetailSchema, "purchase");
    },

    /** A delivery against an order: the stock rises and the supplier's
     * account with it, at the cost the goods landed at. */
    async receivePurchase(id: number, input: NewReceiptDto): Promise<PurchaseDetailDto> {
      const body = await send(`/purchases/${id}/receipts`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, purchaseDetailSchema, "purchase");
    },

    /** Goods handed back. No document is written: the stock movement out and
     * the credit on the ledger are the record. */
    async returnPurchase(id: number, input: NewReceiptDto): Promise<PurchaseDetailDto> {
      const body = await send(`/purchases/${id}/returns`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, purchaseDetailSchema, "purchase");
    },

    /** An order that never happened, only while nothing has arrived. */
    async cancelPurchase(id: number, input: CloseOrderDto): Promise<PurchaseDetailDto> {
      const body = await send(`/purchases/${id}/cancel`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, purchaseDetailSchema, "purchase");
    },

    /** An order the rest of which will never come. What arrived stays. */
    async closeShortPurchase(id: number, input: CloseOrderDto): Promise<PurchaseDetailDto> {
      const body = await send(`/purchases/${id}/close-short`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, purchaseDetailSchema, "purchase");
    },
  };
}
