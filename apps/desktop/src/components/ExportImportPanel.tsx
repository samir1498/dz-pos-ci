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
import { saveBlob } from "@/lib/download";
import { useTranslation, type Key } from "@/i18n";

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  duplicate_barcode: "error_duplicate_barcode",
  not_found: "error_not_found",
  money: "error_money",
  storage: "error_storage",
  restart_needed: "error_restart_needed",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unauthorized: "error_unauthorized",
  unreachable: "error_unreachable",
};

function errorKey(error: unknown): Key {
  if (error instanceof ApiError) return ERROR_KEY[error.code] ?? "error_unknown";
  return "error_unknown";
}

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
  negative_rate: "import_reason_negative_rate",
  rate_not_allowed: "import_reason_rate_not_allowed",
  rate_missing: "import_reason_rate_missing",
  duplicate_in_file: "import_reason_duplicate_in_file",
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
    <section
      aria-labelledby="settings-export-import"
      className="flex flex-col gap-3 rounded border p-4"
    >
      <h2 id="settings-export-import" className="font-semibold">
        {t("settings_export_import")}
      </h2>
      <p className="text-sm opacity-80">{t("settings_export_import_hint")}</p>

      <div className="flex flex-wrap items-end gap-2">
        {EXPORTS.map((one) => (
          <button
            key={one.kind}
            type="button"
            data-testid={one.testId}
            className="rounded border px-3 py-1.5 disabled:opacity-50"
            disabled={busy || (one.kind === "sales" && backwards)}
            onClick={() => {
              setServerError(null);
              download.mutate(one.kind);
            }}
          >
            {t(one.label)}
          </button>
        ))}
      </div>

      <div className="flex flex-wrap items-end gap-2">
        <label className="flex flex-col gap-1">
          <span>{t("field_from")}</span>
          <input
            type="date"
            dir="ltr"
            className="rounded border px-2 py-1 font-mono"
            value={from}
            onChange={(e) => setFrom(e.target.value)}
          />
        </label>
        <label className="flex flex-col gap-1">
          <span>{t("field_to")}</span>
          <input
            type="date"
            dir="ltr"
            className="rounded border px-2 py-1 font-mono"
            value={to}
            onChange={(e) => setTo(e.target.value)}
          />
        </label>
        <p className="text-sm opacity-80">{t("export_range_hint")}</p>
      </div>
      {backwards ? (
        <p role="alert" className="text-red-700">
          {t("error_statement_range_invalid")}
        </p>
      ) : null}

      <h3 className="font-semibold">{t("import_title")}</h3>
      <p className="text-sm opacity-80">{t("import_hint")}</p>
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          data-testid="import-template"
          className="rounded border px-3 py-1.5 disabled:opacity-50"
          disabled={busy}
          onClick={() => {
            setServerError(null);
            template.mutate();
          }}
        >
          {t("action_download_template")}
        </button>
        <input
          ref={picker}
          type="file"
          accept=".xlsx"
          aria-label={t("import_pick_file")}
          data-testid="import-file"
          onChange={(e) => {
            const chosen = e.target.files?.[0] ?? null;
            setDryRun(null);
            setDone(null);
            setServerError(null);
            setFile(chosen);
          }}
        />
        <button
          type="button"
          data-testid="import-dry-run"
          className="rounded border px-3 py-1.5 disabled:opacity-50"
          disabled={busy || file === null}
          onClick={() => {
            if (file !== null) check.mutate(file);
          }}
        >
          {check.isPending ? t("action_checking") : t("action_check_file")}
        </button>
      </div>

      {dryRun !== null ? (
        <DryRunTable report={dryRun} reasonKey={(reason) => REASON_KEY[reason]} />
      ) : null}

      {dryRun !== null ? (
        <div className="flex items-center gap-2">
          <button
            type="button"
            data-testid="import-apply"
            className="rounded border px-3 py-1.5 disabled:opacity-50"
            disabled={busy || !clean || file === null}
            onClick={() => {
              if (file !== null) apply.mutate(file);
            }}
          >
            {apply.isPending ? t("action_importing") : t("action_import")}
          </button>
          {clean ? null : <p className="text-sm opacity-80">{t("import_fix_first")}</p>}
        </div>
      ) : null}

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}
      {done !== null && serverError === null ? (
        <p role="status" data-testid="import-done">
          {done}
        </p>
      ) : null}
    </section>
  );
}

function DryRunTable({
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
  return (
    <>
      <p data-testid="import-counts">
        {t("import_counts")
          .replace("{accepted}", String(report.accepted))
          .replace("{refused}", String(report.refused))}
      </p>
      <table className="w-full text-start">
        <caption className="sr-only">{t("import_title")}</caption>
        <thead>
          <tr>
            <th scope="col" className="pb-2 text-start">
              {t("col_row")}
            </th>
            <th scope="col" className="pb-2 text-start">
              {t("col_name")}
            </th>
            <th scope="col" className="pb-2 text-start">
              {t("col_outcome")}
            </th>
          </tr>
        </thead>
        <tbody>
          {report.rows.map((row) => {
            const known = row.reason === null ? undefined : reasonKey(row.reason);
            return (
              <tr key={row.row} className="border-t" data-testid="import-row">
                <td className="py-1.5 pe-3 font-mono" dir="ltr">
                  {row.row}
                </td>
                <td className="py-1.5 pe-3">{row.name}</td>
                <td className="py-1.5 pe-3">
                  {t(outcomeLabel[row.outcome])}
                  {row.field === null ? null : (
                    <span className="ms-2 text-sm opacity-80">
                      {row.field}
                      {": "}
                      {known === undefined ? row.reason : t(known)}
                    </span>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </>
  );
}
