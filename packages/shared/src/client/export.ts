// The four workbooks a shop takes away.

import type { Download, ExportKind, PrintLang } from "../client-response";
import type { Transport } from "../client";

export function exportClient({ sendFile }: Transport) {
  return {
    /** The four workbooks a shop takes away. The range is the sales
     * workbook's alone; the other three are the rows as they stand today,
     * because a product is a current row and not an event. */
    async exportWorkbook(
      kind: ExportKind,
      lang: PrintLang,
      range?: { from?: string; to?: string },
    ): Promise<Download> {
      const query = new URLSearchParams({ lang });
      if (range?.from !== undefined && range.from !== "") query.set("from", range.from);
      if (range?.to !== undefined && range.to !== "") query.set("to", range.to);
      return sendFile(`/export/${kind}?${query.toString()}`);
    },
  };
}
