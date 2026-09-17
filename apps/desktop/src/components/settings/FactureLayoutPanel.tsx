// Which layout the shop's factures print in.
//
// Not the paper. The sheet is named on each print, because the till is the
// only place that knows which tray the cashier reached for; the layout is
// chosen here once and every facture follows it (crates/core, print::facture).
//
// The list of layouts comes from the server rather than from a constant in
// this file. A layout is added in Rust, with a template and its goldens, and
// this screen shows it without being touched.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { FactureLayoutDto, SettingsDto } from "@dzpos/shared";

import { api, settingsQueryKey } from "@/api";
import { PanelHeading } from "@/components/settings/PanelHeading";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

const LAYOUT_LABEL: Record<FactureLayoutDto, Key> = {
  standard: "facture_layout_standard",
  compact: "facture_layout_compact",
};

const LAYOUT_HINT: Record<FactureLayoutDto, Key> = {
  standard: "facture_layout_standard_hint",
  compact: "facture_layout_compact_hint",
};

/** A layout the server offered that this build has no wording for is left
 *  out rather than shown by its code name. The list is the server's, so a
 *  newer server can offer one this screen was written before. */
function known(layout: FactureLayoutDto): boolean {
  return layout in LAYOUT_LABEL;
}

export function FactureLayoutPanel({ settings }: { settings: SettingsDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const choose = useMutation({
    mutationFn: (layout: FactureLayoutDto) => api.setFactureLayout(layout),
    onSuccess: async (next) => {
      setServerError(null);
      queryClient.setQueryData<SettingsDto>(settingsQueryKey, next);
      await queryClient.invalidateQueries({ queryKey: settingsQueryKey });
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  const offered = settings.facture_layouts.filter(known);

  return (
    <section aria-labelledby="settings-facture-layout">
      <Card>
        <CardHeader>
          <PanelHeading id="settings-facture-layout">{t("settings_facture_layout")}</PanelHeading>
          <CardDescription>{t("settings_facture_layout_hint")}</CardDescription>
        </CardHeader>
        <CardContent className="flex min-w-0 flex-col gap-3">
          <Select
            value={settings.facture_layout}
            disabled={choose.isPending}
            onValueChange={(value) => {
              const picked = offered.find((layout) => layout === value);
              // Never guessed. An option this build does not offer is
              // ignored rather than sent on: what a facture looks like is
              // not something to resolve by falling back.
              if (picked === undefined) return;
              choose.mutate(picked);
            }}
          >
            <SelectTrigger aria-label={t("settings_facture_layout")} className="sm:w-80">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {offered.map((layout) => (
                <SelectItem key={layout} value={layout}>
                  {t(LAYOUT_LABEL[layout])}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-sm text-muted-foreground">{t(LAYOUT_HINT[settings.facture_layout])}</p>
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
