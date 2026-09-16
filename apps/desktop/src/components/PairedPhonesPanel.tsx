// The paired phones block on the settings screen (M6 T2, M6 T4).
//
// Nothing rendered this before, which is the whole reason a cashier was being
// asked to type sixty-four hexadecimal characters off a terminal: the API has
// had `/pairing/qr` since M6 and no screen ever called it, so there was no QR
// anywhere for a phone to scan.
//
// Two things here are deliberate.
//
// The countdown is not decoration. A pairing token lives sixty seconds and is
// single use, so an owner who mints one and then walks off to find the
// cashier has already wasted it. The screen says that while it is still true
// rather than after, and the QR is replaced by "ask for another" the instant
// it lapses instead of staying on screen looking valid.
//
// The token is also printed underneath in a mono line. A QR is the fast path,
// not the only one: a cracked camera lens or a phone that will not focus
// under a shop's lighting is a Tuesday, and the phone keeps a typed box for
// exactly that.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Smartphone } from "lucide-react";
import { useEffect, useState } from "react";
import QRCode from "react-qr-code";
import { ApiError } from "@dzpos/shared";
import type { PairedDeviceDto } from "@dzpos/shared";
import { api, pairedDevicesQueryKey } from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
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
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

/** How wide the QR is drawn, in CSS pixels. Big enough that a phone reads it
 *  across a counter; a QR that has to be leaned into is a QR nobody scans. */
const QR_SIZE = 200;

export function PairedPhonesPanel() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const devices = useQuery({
    queryKey: pairedDevicesQueryKey,
    queryFn: () => api.listPairedDevices(),
  });
  const [token, setToken] = useState<string | null>(null);
  const [secondsLeft, setSecondsLeft] = useState(0);
  const [revoking, setRevoking] = useState<PairedDeviceDto | null>(null);

  const mint = useMutation({
    mutationFn: () => api.createPairingQr(),
    onSuccess: (qr) => {
      setToken(qr.pairing_token);
      setSecondsLeft(qr.expires_in_seconds);
    },
  });

  const revoke = useMutation({
    mutationFn: (id: number) => api.revokePairedDevice(id),
    onSuccess: async () => {
      setRevoking(null);
      await queryClient.invalidateQueries({ queryKey: pairedDevicesQueryKey });
    },
  });

  // One interval while a token is live, cleared the moment it lapses. The
  // list is refetched at the same time: a phone that scanned the QR is in it
  // now, and nothing else on this screen would have gone looking.
  useEffect(() => {
    if (token === null) return;
    const timer = setInterval(() => {
      setSecondsLeft((left) => {
        if (left > 1) return left - 1;
        setToken(null);
        void queryClient.invalidateQueries({ queryKey: pairedDevicesQueryKey });
        return 0;
      });
    }, 1000);
    return () => clearInterval(timer);
  }, [token, queryClient]);

  const columns: readonly Column<PairedDeviceDto>[] = [
    { id: "name", header: t("phones_name_header"), cell: (row) => row.name },
    { id: "paired", header: t("phones_paired_header"), cell: (row) => row.created_at },
    {
      id: "state",
      header: t("phones_state_header"),
      cell: (row) =>
        row.revoked_at === null ? t("phones_state_trusted") : t("phones_state_revoked"),
    },
  ];

  const live = devices.data?.filter((row) => row.revoked_at === null) ?? [];

  return (
    <Card>
      <CardHeader>
        <CardDescription>{t("settings_phones_hint")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {token !== null ? (
          <div className="flex flex-col items-start gap-3">
            {/* White behind the QR whatever the theme: a dark surface under a
                dark-on-transparent QR is one a camera cannot read. */}
            <div className="rounded-md bg-white p-3">
              <QRCode value={token} size={QR_SIZE} />
            </div>
            <p className="text-sm text-muted-foreground">
              {t("phones_qr_expires")} <span dir="ltr">{secondsLeft}s</span>
            </p>
            <code dir="ltr" className="break-all font-mono text-xs text-muted-foreground">
              {token}
            </code>
          </div>
        ) : null}

        {mint.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(mint.error))}
          </p>
        ) : null}

        {devices.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(devices.error))}
          </p>
        ) : devices.isSuccess && devices.data.length === 0 ? (
          <EmptyState
            icon={Smartphone}
            title={t("phones_none")}
            description={t("phones_none_hint")}
          />
        ) : devices.isSuccess ? (
          <DataTable
            columns={columns}
            rows={devices.data}
            rowKey={(row) => String(row.id)}
            caption={t("settings_phones")}
            empty={t("phones_none")}
            actions={(row) =>
              row.revoked_at === null ? (
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => setRevoking(row)}
                  aria-label={`${t("phones_revoke")} — ${row.name}`}
                >
                  {t("phones_revoke")}
                </Button>
              ) : null
            }
          />
        ) : null}
      </CardContent>
      <CardFooter className="flex items-center gap-3">
        <Button type="button" onClick={() => mint.mutate()} disabled={mint.isPending}>
          {t("phones_show_qr")}
        </Button>
        <p className="text-sm text-muted-foreground">
          <span dir="ltr">{live.length}</span> {t("phones_trusted_count")}
        </p>
      </CardFooter>

      <Dialog open={revoking !== null} onOpenChange={(open) => !open && setRevoking(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("phones_revoke_title")}</DialogTitle>
            <DialogDescription>{t("phones_revoke_confirm_hint")}</DialogDescription>
          </DialogHeader>
          {revoke.isError ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(errorKey(revoke.error))}
            </p>
          ) : null}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setRevoking(null)}>
              {t("action_cancel")}
            </Button>
            <Button
              type="button"
              variant="destructive"
              disabled={revoke.isPending}
              onClick={() => revoking !== null && revoke.mutate(revoking.id)}
            >
              {t("phones_revoke_confirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  );
}
