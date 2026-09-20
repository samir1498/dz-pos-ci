// The shop's rows going out and coming in, and the nightly recount that
// keeps the quantity on a fiche honest against its movements.
//
// The room itself is open to every role because the recount is. The four
// exports and the two import routes are `ExportAndImport` in
// `crates/api/src/gates/`, so that block alone is hidden from a role that
// does not hold it — the server refuses those routes either way.

import { createFileRoute } from "@tanstack/react-router";

import { ExportImportPanel } from "@/components/ExportImportPanel";
import { StockRecountPanel } from "@/components/StockRecountPanel";
import { useHasPermission } from "@/lib/session";

export const Route = createFileRoute("/settings/data")({ component: DataRoom });

export function DataRoom() {
  const exportAndImport = useHasPermission("export_and_import");
  return (
    <>
      {exportAndImport ? <ExportImportPanel /> : null}
      <StockRecountPanel />
    </>
  );
}
