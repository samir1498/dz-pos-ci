// The till: ringing up a sale, its two prints, listing sales, avoirs and
// cancelling a document.

import { z } from "zod";

import type { CancelDocumentDto } from "../generated/CancelDocumentDto";
import type { NewAvoirDto } from "../generated/NewAvoirDto";
import type { NewSaleDto } from "../generated/NewSaleDto";
import type { SaleDto } from "../generated/SaleDto";
import type { SaleKindDto } from "../generated/SaleKindDto";
import { narrow, type PrintLang, type PrintPaper } from "../client-response";
import type { Transport } from "../client";
import { saleSchema } from "../schemas/sale";

export function salesClient({ send, sendText }: Transport) {
  return {
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
  };
}
