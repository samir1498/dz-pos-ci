// The daily stock recount: when it last ran and forcing one now.

import type { LastStockRecountDto } from "../generated/LastStockRecountDto";
import type { StockRecountDto } from "../generated/StockRecountDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { lastStockRecountSchema, stockRecountSchema } from "../schemas/stock";

export function stockClient({ send }: Transport) {
  return {
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
  };
}
