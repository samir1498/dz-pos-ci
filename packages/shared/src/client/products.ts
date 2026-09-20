// The catalogue: the product list, writing one back, creating one, and the
// two shelf-label prints.

import { z } from "zod";

import type { NewProductDto } from "../generated/NewProductDto";
import type { ProductDto } from "../generated/ProductDto";
import { ApiError, narrow, type PrintLang } from "../client-response";
import type { Transport } from "../client";
import { productSchema } from "../schemas/catalogue";
import { labelSheetSchema } from "../schemas/import";

export function productsClient({ send, sendText }: Transport) {
  return {
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

    async createProduct(input: NewProductDto): Promise<ProductDto> {
      const body = await send("/products", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, productSchema, "product");
    },

    /** The 58 x 40 mm shelf label for one product, as a page to print. */
    async getProductLabel(id: number, lang: PrintLang): Promise<string> {
      return sendText(`/products/${id}/label?lang=${lang}`);
    },

    /** A sheet of those labels on A4, in the order the ids are given.
     *
     * The selection is checked against the same cap the API holds before
     * the call is made: a body the server will refuse is a call not worth
     * making, and the screen gets a `bad_request` it already translates
     * rather than a round trip. */
    async getLabelSheet(ids: readonly number[], lang: PrintLang): Promise<string> {
      const body = labelSheetSchema.safeParse({ ids: [...ids] });
      if (!body.success) {
        throw new ApiError("bad_request", "that is not a printable selection", 0);
      }
      return sendText(`/labels/sheet?lang=${lang}`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body.data),
      });
    },
  };
}
