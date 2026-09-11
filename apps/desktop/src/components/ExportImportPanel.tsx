// The export and import block on the settings screen (features.md §1). Four
// workbooks out, one filled workbook back in.
//
// The import is two calls and never one: the dry run says what the file
// would do and writes nothing, and apply is a second decision the shop takes
// after reading the table. A file with one refusal in it cannot be applied
// at all, so the button is not offered; that rule lives in the core and is
// only mirrored here, because a button that can only fail is a button worth
// not showing.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { ApiError } from "@dzpos/shared";
import type { ExportKind, ImportDryRunDto, ImportRowDto } from "@dzpos/shared";
import { api, categoriesQueryKey, productsQueryKey } from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { FormField } from "@/components/FormField";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { saveBlob } from "@/lib/download";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";



const EXPORTS: { kind: ExportKind; label: Key; testId: string }[] = [
  { kind: "products", label: "export_products", testId: "export-products" },
  { kind: "sales", label: "export_sales", testId: "export-sales" },
  { kind: "customers", label: "export_customers", testId: "export-customers" },
  { kind: "suppliers", label: "export_suppliers", testId: "export-suppliers" },
];

/** The refusal keys the core sends, as the sentences a shop reads. The core
 * sends a key and never a sentence, the same way an error code crosses
 * (architecture.md, error policy). An unknown one is shown as itself rather
 * than swallowed: a rule added to the core must not become a blank cell
 * here while the three translations catch up. */
const REASON_KEY: Record<string, Key> = {
  missing_name: "import_reason_missing_name",
  unknown_unit: "import_reason_unknown_unit",
  missing_amount: "import_reason_missing_amount",
  negative_amount: "import_reason_negative_amount",
  negative_quantity: "import_reason_negative_quantity",
  not_a_number: "import_reason_not_a_number",
  too_many_decimals: "import_reason_too_many_decimals",
  negative_rate: "import_reason_negative_rate",
  rate_not_allowed: "import_reason_rate_not_allowed",
  rate_missing: "import_reason_rate_missing",
  duplicate_in_file: "import_reason_duplicate_in_file",
  bad_barcode: "import_reason_bad_barcode",
};

export function ExportImportPanel() {
  const { t, lang } = useTranslation();
  const queryClient = useQueryClient();
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [dryRun, setDryRun] = useState<ImportDryRunDto | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [serverError, setServerError] = useState<Key | null>(null);
  const picker = useRef<HTMLInputElement>(null);

  const download = useMutation({
    mutationFn: async (kind: ExportKind) =>
      kind === "sales"
        ? api.exportWorkbook(kind, lang, { from, to })
        : api.exportWorkbook(kind, lang),
    onSuccess: (got) => {
      setServerError(null);
      saveBlob(got.blob, got.filename);
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  const template = useMutation({
    mutationFn: () => api.importTemplate(lang),
    onSuccess: (got) => {
      setServerError(null);
      saveBlob(got.blob, got.filename);
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  const check = useMutation({
    mutationFn: (chosen: File) => api.dryRunProductImport(chosen),
    onSuccess: (report) => {
      setServerError(null);
      setDone(null);
      setDryRun(report);
    },
    onError: (error: unknown) => {
      setDryRun(null);
      setServerError(errorKey(error));
    },
  });

  const apply = useMutation({
    mutationFn: (chosen: File) => api.applyProductImport(chosen),
    onSuccess: async (result) => {
      setServerError(null);
      setDryRun(null);
      setFile(null);
      if (picker.current !== null) picker.current.value = "";
      setDone(
        t("import_done")
          .replace("{created}", String(result.created))
          .replace("{updated}", String(result.updated)),
      );
      // The file wrote products and may have made categories on the way.
      await queryClient.invalidateQueries({ queryKey: productsQueryKey });
      await queryClient.invalidateQueries({ queryKey: categoriesQueryKey });
    },
    onError: (error: unknown) => {
      setDone(null);
      setServerError(errorKey(error));
    },
  });

  const busy = download.isPending || template.isPending || check.isPending || apply.isPending;
  // A range the wrong way round is a typing mistake, and a call that can
  // only be refused is a call not worth making.
  const backwards = from !== "" && to !== "" && from > to;
  const clean = dryRun !== null && dryRun.refused === 0 && dryRun.accepted > 0;

  return (
    <Card aria-labelledby="settings-export-import" data-testid="export-import">
      <CardHeader>
        {/* A heading and not `CardTitle`, which is a div: this block is a
            section of the settings screen and a screen reader's outline has
            to be able to jump to it. */}
        <h2 id="settings-export-import" className="text-md leading-none font-semibold">
          {t("settings_export_import")}
        </h2>
        <CardDescription>{t("settings_export_import_hint")}</CardDescription>
      </CardHeader>

      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-wrap items-end gap-2">
          {EXPORTS.map((one) => (
            <Button
              key={one.kind}
              variant="outline"
              data-testid={one.testId}
              disabled={busy || (one.kind === "sales" && backwards)}
              onClick={() => {
                setServerError(null);
                download.mutate(one.kind);
              }}
            >
              {t(one.label)}
            </Button>
          ))}
        </div>

        <div className="flex flex-wrap items-end gap-4">
          <FormField label={t("field_from")}>
            {(parts) => (
              <Input
                {...parts}
                type="date"
                dir="ltr"
                className="font-numeric tabular-nums"
                value={from}
                onChange={(e) => setFrom(e.target.value)}
              />
            )}
          </FormField>
          <FormField label={t("field_to")}>
            {(parts) => (
              <Input
                {...parts}
                type="date"
                dir="ltr"
                className="font-numeric tabular-nums"
                value={to}
                onChange={(e) => setTo(e.target.value)}
              />
            )}
          </FormField>
          <p className="text-sm text-muted-foreground">{t("export_range_hint")}</p>
        </div>
        {backwards ? (
          <p role="alert" className="text-fg-danger">
            {t("error_statement_range_invalid")}
          </p>
        ) : null}

        <Separator />

        <h3 className="text-md font-semibold">{t("import_title")}</h3>
        <p className="text-sm text-muted-foreground">{t("import_hint")}</p>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            data-testid="import-template"
            disabled={busy}
            onClick={() => {
              setServerError(null);
              template.mutate();
            }}
          >
            {t("action_download_template")}
          </Button>
          <Input
            ref={picker}
            type="file"
            accept=".xlsx"
            aria-label={t("import_pick_file")}
            data-testid="import-file"
            className="w-auto"
            onChange={(e) => {
              const chosen = e.target.files?.[0] ?? null;
              setDryRun(null);
              setDone(null);
              setServerError(null);
              setFile(chosen);
            }}
          />
          <Button
            data-testid="import-dry-run"
            disabled={busy || file === null}
            onClick={() => {
              if (file !== null) check.mutate(file);
            }}
          >
            {check.isPending ? t("action_checking") : t("action_check_file")}
          </Button>
        </div>

        {dryRun !== null ? (
          <DryRunReport report={dryRun} reasonKey={(reason) => REASON_KEY[reason]} />
        ) : null}

        {dryRun !== null ? (
          <div className="flex flex-wrap items-center gap-2">
            <Button
              data-testid="import-apply"
              disabled={busy || !clean || file === null}
              onClick={() => {
                if (file !== null) apply.mutate(file);
              }}
            >
              {apply.isPending ? t("action_importing") : t("action_import")}
            </Button>
            {clean ? null : (
              <p className="text-sm text-muted-foreground">{t("import_fix_first")}</p>
            )}
          </div>
        ) : null}

        {serverError !== null ? (
          <p role="alert" className="text-fg-danger">
            {t(serverError)}
          </p>
        ) : null}
        {done !== null && serverError === null ? (
          <p role="status" data-testid="import-done">
            {done}
          </p>
        ) : null}
      </CardContent>
    </Card>
  );
}

/**
 * What the file would do, read before anything is written. The outcome wears
 * a badge rather than a colour alone, and the refusal carries the field the
 * core named beside it: "unknown_unit" on line 4 is a cell to go and fix, and
 * a row that only said "refused" sends the shop back through the whole file.
 */
function DryRunReport({
  report,
  reasonKey,
}: {
  report: ImportDryRunDto;
  reasonKey: (reason: string) => Key | undefined;
}) {
  const { t } = useTranslation();
  const outcomeLabel: Record<ImportRowDto["outcome"], Key> = {
    created: "import_outcome_created",
    updated: "import_outcome_updated",
    refused: "import_outcome_refused",
  };
  const tone = (outcome: ImportRowDto["outcome"]) =>
    outcome === "refused" ? "destructive" : outcome === "updated" ? "secondary" : "default";

  const columns: readonly Column<ImportRowDto>[] = [
    {
      id: "row",
      header: t("col_row"),
      numeric: true,
      cell: (row) => (
        <span dir="ltr" data-testid="import-row-number">
          {row.row}
        </span>
      ),
    },
    { id: "name", header: t("col_name"), cell: (row) => row.name },
    {
      id: "outcome",
      header: t("col_outcome"),
      cell: (row) => {
        const known = row.reason === null ? undefined : reasonKey(row.reason);
        return (
          <span className="flex flex-wrap items-center gap-2">
            <Badge variant={tone(row.outcome)}>{t(outcomeLabel[row.outcome])}</Badge>
            {row.field === null ? null : (
              <span className="text-sm text-muted-foreground">
                {row.field}
                {": "}
                {known === undefined ? row.reason : t(known)}
              </span>
            )}
          </span>
        );
      },
    },
  ];

  return (
    <div className="flex flex-col gap-2">
      <p data-testid="import-counts">
        {t("import_counts")
          .replace("{accepted}", String(report.accepted))
          .replace("{refused}", String(report.refused))}
      </p>
      {/* Only when a row really is an update: the sentence answers a
          question nobody asked on a file that creates everything. */}
      {report.rows.some((row) => row.outcome === "updated") ? (
        <p className="text-sm text-muted-foreground" data-testid="import-keeps-stock">
          {t("import_update_keeps_stock")}
        </p>
      ) : null}
      <DataTable
        columns={columns}
        rows={report.rows}
        rowKey={(row) => row.row}
        caption={t("import_title")}
        data-testid="import-table"
      />
    </div>
  );
}
