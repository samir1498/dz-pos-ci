// Whether the ticket also carries the seller's NIF, RC, NIS and AI.
//
// T55: the facture names the four identifiers whatever this says — the law
// asks nothing of a ticket de caisse, so the receipt is heavy for what it is
// when it repeats them. Off by default, for both a fresh shop and one that
// upgraded into this setting: the four identifiers are the fuller answer,
// not the safer one, and a shop that wants them back turns this on rather
// than losing something a build silently kept for it.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { SettingsDto } from "@dzpos/shared";

import { api, settingsQueryKey } from "@/api";
import { PanelHeading } from "@/components/settings/PanelHeading";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

export function TicketFiscalIdsPanel({ settings }: { settings: SettingsDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const choose = useMutation({
    mutationFn: (show: boolean) => api.setTicketFiscalIds(show),
    onSuccess: async (next) => {
      setServerError(null);
      queryClient.setQueryData<SettingsDto>(settingsQueryKey, next);
      await queryClient.invalidateQueries({ queryKey: settingsQueryKey });
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  return (
    <section aria-labelledby="settings-ticket-fiscal-ids">
      <Card>
        <CardHeader>
          <PanelHeading id="settings-ticket-fiscal-ids">
            {t("settings_ticket_fiscal_ids")}
          </PanelHeading>
          <CardDescription>{t("settings_ticket_fiscal_ids_hint")}</CardDescription>
        </CardHeader>
        <CardContent className="flex min-w-0 flex-col gap-3">
          <div className="flex items-center gap-3">
            <Switch
              id="ticket-fiscal-ids"
              checked={settings.ticket_fiscal_ids}
              disabled={choose.isPending}
              onCheckedChange={(checked) => choose.mutate(checked)}
            />
            <Label htmlFor="ticket-fiscal-ids">{t("settings_ticket_fiscal_ids_toggle")}</Label>
          </div>
          {serverError !== null ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(serverError)}
            </p>
          ) : null}
        </CardContent>
      </Card>
    </section>
  );
}
