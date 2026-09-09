// The backups block on the settings screen (features.md §1). The server
// owns the folder, the names and the thirty it keeps; this file lists what
// the server reports, asks for one more, and asks the owner twice before it
// puts one back.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { ApiError } from "@dzpos/shared";
import type { BackupDto } from "@dzpos/shared";
import { api, backupsQueryKey } from "@/api";
import { useTranslation, type Key } from "@/i18n";

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  not_found: "error_not_found",
  storage: "error_storage",
  restart_needed: "error_restart_needed",
  restore_failed_restart_needed: "error_restore_failed_restart_needed",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unauthorized: "error_unauthorized",
  unreachable: "error_unreachable",
};

function errorKey(error: unknown): Key {
  if (error instanceof ApiError) return ERROR_KEY[error.code] ?? "error_unknown";
  return "error_unknown";
}

const BYTES_PER_KB = 1024;
const BYTES_PER_MB = BYTES_PER_KB * 1024;

/** `2026-09-08T09:30:00` as `2026-09-08 09:30`. The server already wrote the
 * shop's own calendar into it, so nothing here re-reads a clock. */
function readableTime(takenAt: string): string {
  return takenAt.replace("T", " ").slice(0, 16);
}

/** A file size, not an amount: this is the one place a fraction is fine, and
 * it is display only (architecture.md rule 6 is about money). */
function readableSize(bytes: number, kb: string, mb: string): string {
  if (bytes >= BYTES_PER_MB) {
    return `${(bytes / BYTES_PER_MB).toFixed(1).replace(".", ",")} ${mb}`;
  }
  return `${Math.round(bytes / BYTES_PER_KB)} ${kb}`;
}

export function BackupsPanel() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const backups = useQuery({ queryKey: backupsQueryKey, queryFn: () => api.listBackups() });
  const [done, setDone] = useState<Key | null>(null);
  const [serverError, setServerError] = useState<Key | null>(null);

  const create = useMutation({
    mutationFn: () => api.createBackup(),
    onSuccess: async () => {
      setServerError(null);
      setDone("backups_created");
      await queryClient.invalidateQueries({ queryKey: backupsQueryKey });
    },
    onError: (error: unknown) => {
      setDone(null);
      setServerError(errorKey(error));
    },
  });

  const restore = useMutation({
    mutationFn: (name: string) => api.restoreBackup(name),
    onSuccess: async () => {
      setServerError(null);
      setDone("backups_restored");
      // The whole file was replaced: products, categories and settings are
      // all someone else's rows now, so nothing cached survives.
      await queryClient.invalidateQueries();
    },
    onError: (error: unknown) => {
      setDone(null);
      setServerError(errorKey(error));
    },
  });

  const busy = create.isPending || restore.isPending;
  const rows: BackupDto[] = backups.data?.backups ?? [];
  const safetyCopies: BackupDto[] = backups.data?.safety_copies ?? [];
  const newest = rows[0];

  function askThenRestore(name: string) {
    setDone(null);
    setServerError(null);
    // A confirm dialog is the M1 answer, translated like every other string.
    // A restore throws away everything since the copy, so it is never one
    // click away.
    if (!window.confirm(t("backups_confirm_restore"))) return;
    restore.mutate(name);
  }

  return (
    <section aria-labelledby="settings-backups" className="flex flex-col gap-3 rounded border p-4">
      <h2 id="settings-backups" className="font-semibold">
        {t("settings_backups")}
      </h2>
      <p className="text-sm opacity-80">{t("settings_backups_hint")}</p>

      {backups.isPending ? <p>{t("products_loading")}</p> : null}
      {backups.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(backups.error))}
        </p>
      ) : null}

      {backups.isSuccess ? (
        <>
          <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
            <dt>{t("backups_newest_label")}</dt>
            <dd data-testid="backups-newest">
              {newest === undefined ? t("backups_none") : readableTime(newest.taken_at)}
            </dd>
          </dl>

          {rows.length === 0 ? null : (
            <ul className="flex flex-col divide-y rounded border">
              {rows.map((row) => (
                <li
                  key={row.name}
                  data-testid="backup-row"
                  className="flex items-center justify-between gap-4 px-3 py-2"
                >
                  <span>{readableTime(row.taken_at)}</span>
                  <span className="text-sm opacity-80">
                    {readableSize(row.bytes, t("unit_kb"), t("unit_mb"))}
                  </span>
                  <button
                    type="button"
                    className="rounded border px-3 py-1.5 disabled:opacity-50"
                    disabled={busy}
                    onClick={() => askThenRestore(row.name)}
                  >
                    {restore.isPending && restore.variables === row.name
                      ? t("action_restoring")
                      : t("action_restore")}
                  </button>
                </li>
              ))}
            </ul>
          )}

          {/* Kept under their own heading because they are kept under their
              own rule: the daily copies are pruned to thirty, these are
              never deleted, and they are the only record of a state the
              owner replaced. No restore button: restoring one is a decision
              that needs a person who knows the file, not one more click. */}
          {safetyCopies.length === 0 ? null : (
            <section aria-labelledby="settings-safety-copies" className="flex flex-col gap-2">
              <h3 id="settings-safety-copies" className="font-semibold">
                {t("settings_safety_copies")}
              </h3>
              <p className="text-sm opacity-80">{t("settings_safety_copies_hint")}</p>
              <ul className="flex flex-col divide-y rounded border">
                {safetyCopies.map((row) => (
                  <li
                    key={row.name}
                    data-testid="safety-copy-row"
                    className="flex items-center justify-between gap-4 px-3 py-2"
                  >
                    <span>{readableTime(row.taken_at)}</span>
                    <span className="text-sm opacity-80">
                      {readableSize(row.bytes, t("unit_kb"), t("unit_mb"))}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          )}
        </>
      ) : null}

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}
      {done !== null && serverError === null ? <p role="status">{t(done)}</p> : null}

      <div>
        <button
          type="button"
          className="rounded border px-3 py-1.5 disabled:opacity-50"
          disabled={busy}
          onClick={() => {
            setDone(null);
            setServerError(null);
            create.mutate();
          }}
        >
          {create.isPending ? t("action_saving") : t("action_backup_now")}
        </button>
      </div>
    </section>
  );
}
