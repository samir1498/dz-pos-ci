// The product import: the empty template, what a file would do, and
// applying it.

import type { ImportAppliedDto } from "../generated/ImportAppliedDto";
import type { ImportDryRunDto } from "../generated/ImportDryRunDto";
import { narrow, type Download, type PrintLang } from "../client-response";
import type { Transport } from "../client";
import { importAppliedSchema, importDryRunSchema } from "../schemas/import";

export function importClient({ send, sendFile }: Transport) {
  return {
    /** The empty workbook a shop fills in and posts back. */
    async importTemplate(lang: PrintLang): Promise<Download> {
      return sendFile(`/import/products/template?lang=${lang}`);
    },

    /** What the file would do, with nothing written. A file with refusals in
     * it still answers 200: the refusals are the answer. */
    async dryRunProductImport(file: Blob): Promise<ImportDryRunDto> {
      const body = await send("/import/products/dry-run", { method: "POST", body: file });
      return narrow(body, importDryRunSchema, "import dry run");
    },

    /** The file, written, or nothing at all. */
    async applyProductImport(file: Blob): Promise<ImportAppliedDto> {
      const body = await send("/import/products", { method: "POST", body: file });
      return narrow(body, importAppliedSchema, "import result");
    },
  };
}
