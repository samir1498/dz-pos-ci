// The backups block on the settings screen (features.md §1). The server
// owns the folder, the names and the thirty it keeps; this file lists what
// the server reports, asks for one more, and asks the owner twice before it
// puts one back.
//
// The second ask is the kit's dialog rather than the browser's `confirm`.
// The native box wears the operating system's colours, cannot be read by the
// shop in Arabic on a French Windows, and reduces the most destructive action
// in the app to a sentence in a grey rectangle. The dialog says what is lost
// and puts the confirmation on a destructive button of its own.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Archive, RotateCcw } from "lucide-react";
import { useState } from "react";
import { ApiError } from "@dzpos/shared";
import type { BackupDto } from "@dzpos/shared";
import { api, backupsQueryKey } from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";



const BYTES_PER_KB = 1024;
const BYTES_PER_MB = BYTES_PER_KB * 1024;

/** Which of the three lists a copy came out of. The three are kept under
 *  three rules and put back three different shops, so the confirmation says
 *  which one it is holding rather than only when it was taken: two lists can
 *  each hold a copy from the same minute. */
type Kind = "daily" | "safety" | "upgrade";

const KIND_LABEL: Record<Kind, Key> = {
  daily: "settings_backups",
  safety: "settings_safety_copies",
  upgrade: "settings_upgrade_copies",
};

/** The copy the confirmation is about, and the list it came from. */
interface Asked {
  copy: BackupDto;
  kind: Kind;
}

/** `2026-09-08T09:30:00` as `2026-09-08 09:30`. The server already wrote the
 * shop's own calendar into it, so nothing here re-reads a clock. */
function readableTime(takenAt: string): string {
  return takenAt.replace("T", " ").slice(0, 16);
}

/** A file size, not an amount: this is the one place a fraction is fine, and
 * it is display only (architecture.md rule 6 is about money). The separator
 * comes from the dictionary like every other number on a screen
 * (`lib/rate.ts` says why it is a key and not a comma written here). */
export function readableSize(
  bytes: number,
  kb: string,
  mb: string,
  decimalSeparator: string,
): string {
  if (bytes >= BYTES_PER_MB) {
    return `${(bytes / BYTES_PER_MB).toFixed(1).replace(".", decimalSeparator)} ${mb}`;
  }
  return `${Math.round(bytes / BYTES_PER_KB)} ${kb}`;
}

export function BackupsPanel() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const backups = useQuery({ queryKey: backupsQueryKey, queryFn: () => api.listBackups() });
  const [done, setDone] = useState<Key | null>(null);
  const [serverError, setServerError] = useState<Key | null>(null);
  // The copy the owner asked about, and the whole of the dialog's state: an
  // open dialog with nothing in it would have nothing to restore.
  const [asking, setAsking] = useState<Asked | null>(null);

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
  const upgradeCopies: BackupDto[] = backups.data?.upgrade_copies ?? [];
  const newest = rows[0];

  const columns: readonly Column<BackupDto>[] = [
    {
      id: "taken",
      header: t("backups_taken_header"),
      cell: (row) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {readableTime(row.taken_at)}
        </span>
      ),
    },
    {
      id: "size",
      header: t("backups_size_header"),
      numeric: true,
      cell: (row) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {readableSize(row.bytes, t("unit_kb"), t("unit_mb"), t("decimal_separator"))}
        </span>
      ),
    },
  ];

  const askToRestore = (kind: Kind) => (copy: BackupDto) => (
    <Button
      type="button"
      variant="outline"
      size="sm"
      disabled={busy}
      onClick={() => {
        setDone(null);
        setServerError(null);
        setAsking({ copy, kind });
      }}
    >
      {/* An undo, which the kit mirrors with the page: on the Arabic screen
          "back" is the other way round. */}
      <Icon as={RotateCcw} size={18} flip />
      {restore.isPending && restore.variables === copy.name
        ? t("action_restoring")
        : t("action_restore")}
    </Button>
  );

  return (
    <section aria-labelledby="settings-backups">
      <Card>
        <CardHeader>
          <h3 id="settings-backups" className="font-semibold text-foreground">
            {t("settings_backups")}
          </h3>
          <CardDescription>{t("settings_backups_hint")}</CardDescription>
        </CardHeader>

        <CardContent className="flex min-w-0 flex-col gap-4">
          {backups.isPending ? (
            <p className="text-sm text-muted-foreground">{t("settings_loading")}</p>
          ) : null}
          {backups.isError ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(errorKey(backups.error))}
            </p>
          ) : null}

          {backups.isSuccess ? (
            <>
              <dl className="grid min-w-0 grid-cols-[auto_1fr] items-baseline gap-x-4 gap-y-1">
                <dt className="text-sm text-muted-foreground">{t("backups_newest_label")}</dt>
                <dd data-testid="backups-newest" className="min-w-0 font-medium break-words text-foreground">
                  {newest === undefined ? (
                    t("backups_none")
                  ) : (
                    // A date and a time read left to right with Western
                    // digits on the Arabic screen too; without this the hour
                    // swaps in front of the day.
                    <span dir="ltr" className="font-numeric tabular-nums">
                      {readableTime(newest.taken_at)}
                    </span>
                  )}
                </dd>
              </dl>

              <DataTable
                data-testid="backups-table"
                caption={t("settings_backups")}
                columns={columns}
                rows={rows}
                rowKey={(row) => row.name}
                empty={
                  <EmptyState
                    icon={Archive}
                    title={t("backups_none")}
                    description={t("backups_empty_hint")}
                  />
                }
                actions={askToRestore("daily")}
              />

              {/* Kept under their own heading because they are kept under
                  their own rule: the daily copies are pruned to thirty, these
                  are never deleted, and they are the only record of a state
                  the owner replaced. They carry the same restore button as
                  the daily ones, and the same confirmation, which is what
                  makes putting one back a decision rather than a click. Until
                  2026-09-12 the route took a daily copy's name and no other,
                  so undoing a restore meant swapping files by hand on a
                  machine in a shop. */}
              {safetyCopies.length === 0 ? null : (
                <section aria-labelledby="settings-safety-copies" className="flex flex-col gap-2">
                  <h4 id="settings-safety-copies" className="font-semibold text-foreground">
                    {t("settings_safety_copies")}
                  </h4>
                  <p className="text-sm text-muted-foreground">{t("settings_safety_copies_hint")}</p>
                  <DataTable
                    data-testid="safety-copies-table"
                    caption={t("settings_safety_copies")}
                    columns={columns}
                    rows={safetyCopies}
                    rowKey={(row) => row.name}
                    actions={askToRestore("safety")}
                  />
                </section>
              )}

              {/* The third kind, and the one nothing listed at all until
                  2026-09-12: an owner found one by knowing how this app
                  names a file. Restoring one puts the shop back on the data
                  the older version left, which the app then migrates forward
                  on its next open, taking one of these copies again. */}
              {upgradeCopies.length === 0 ? null : (
                <section aria-labelledby="settings-upgrade-copies" className="flex flex-col gap-2">
                  <h4 id="settings-upgrade-copies" className="font-semibold text-foreground">
                    {t("settings_upgrade_copies")}
                  </h4>
                  <p className="text-sm text-muted-foreground">
                    {t("settings_upgrade_copies_hint")}
                  </p>
                  <DataTable
                    data-testid="upgrade-copies-table"
                    caption={t("settings_upgrade_copies")}
                    columns={columns}
                    rows={upgradeCopies}
                    rowKey={(row) => row.name}
                    actions={askToRestore("upgrade")}
                  />
                </section>
              )}
            </>
          ) : null}
        </CardContent>

        <CardFooter className="flex flex-wrap items-center gap-3">
          <Button
            type="button"
            disabled={busy}
            onClick={() => {
              setDone(null);
              setServerError(null);
              create.mutate();
            }}
          >
            {create.isPending ? t("action_saving") : t("action_backup_now")}
          </Button>
          {serverError !== null ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(serverError)}
            </p>
          ) : null}
          {done !== null && serverError === null ? (
            <p role="status" className="text-sm text-fg-success">
              {t(done)}
            </p>
          ) : null}
        </CardFooter>
      </Card>

      <Dialog
        open={asking !== null}
        onOpenChange={(open) => {
          if (!open) setAsking(null);
        }}
      >
        <DialogContent data-testid="backup-restore-dialog">
          <DialogHeader>
            <DialogTitle>{t("backups_restore_title")}</DialogTitle>
            <DialogDescription>{t("backups_confirm_restore")}</DialogDescription>
          </DialogHeader>
          {asking === null ? null : (
            <>
              {/* Which list the copy came from, not just when it was taken:
                  three lists can each hold a copy from the same minute, and
                  the three do different things to the shop. */}
              <p className="text-sm text-muted-foreground">
                {t(KIND_LABEL[asking.kind])}{" "}
                <span dir="ltr" className="font-numeric tabular-nums text-foreground">
                  {readableTime(asking.copy.taken_at)}
                </span>
              </p>
              {asking.kind === "upgrade" ? (
                <p className="text-sm text-fg-danger">{t("backups_restore_older_shape")}</p>
              ) : null}
            </>
          )}
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => setAsking(null)}>
              {t("action_cancel")}
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={restore.isPending}
              onClick={() => {
                if (asking === null) return;
                const name = asking.copy.name;
                setAsking(null);
                restore.mutate(name);
              }}
            >
              {t("backups_restore_confirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
