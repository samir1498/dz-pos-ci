// Which wire the shop's thermal head is sent: text or a raster.
//
// A cheap 80 mm head prints one byte per column out of a single-byte table,
// which is fast and is what French and English have always used. The other
// way is to draw the same lines into a bitmap and send them as dots, which
// needs no table at all (crates/core, print::thermal).
//
// The one thing this choice does not reach is Arabic. No single-byte table
// a cheap head ships with has Arabic in it, and a text-mode head neither
// joins the letters nor runs them right to left, so an Arabic ticket sent as
// text comes out as a box per byte. Arabic is therefore always drawn, and
// the note under the picker says so rather than leaving an owner to find it
// on paper (context/plans/20260921-arabic-on-a-cheap-thermal-head.md).
//
// No "follow the till" sentinel, unlike PrintLanguagePanel: a head is always
// on one of the two wires, so there is no null to stand in for.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { SettingsDto, ThermalModeDto } from "@dzpos/shared";

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

const MODES = ["text", "raster"] as const;

const MODE_LABEL: Record<ThermalModeDto, Key> = {
  text: "thermal_mode_text",
  raster: "thermal_mode_raster",
};

const MODE_HINT: Record<ThermalModeDto, Key> = {
  text: "thermal_mode_text_hint",
  raster: "thermal_mode_raster_hint",
};

export function ThermalModePanel({ settings }: { settings: SettingsDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const choose = useMutation({
    mutationFn: (mode: ThermalModeDto) => api.setThermalMode(mode),
    onSuccess: async (next) => {
      setServerError(null);
      queryClient.setQueryData<SettingsDto>(settingsQueryKey, next);
      await queryClient.invalidateQueries({ queryKey: settingsQueryKey });
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  return (
    <section aria-labelledby="settings-thermal-mode">
      <Card>
        <CardHeader>
          <PanelHeading id="settings-thermal-mode">{t("settings_thermal_mode")}</PanelHeading>
          <CardDescription>{t("settings_thermal_mode_hint")}</CardDescription>
        </CardHeader>
        <CardContent className="flex min-w-0 flex-col gap-3">
          <Select
            value={settings.thermal_mode}
            disabled={choose.isPending}
            onValueChange={(value) => {
              // Never guessed. A value this build did not offer is ignored
              // rather than sent on, the same shape the two pickers beside
              // it use: which wire a head is on decides whether a customer
              // gets a receipt or a page of boxes.
              const picked = MODES.find((mode) => mode === value);
              if (picked === undefined) return;
              choose.mutate(picked);
            }}
          >
            <SelectTrigger aria-label={t("settings_thermal_mode")} className="sm:w-80">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {MODES.map((mode) => (
                <SelectItem key={mode} value={mode}>
                  {t(MODE_LABEL[mode])}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-sm text-muted-foreground">{t(MODE_HINT[settings.thermal_mode])}</p>
          <p className="text-sm text-muted-foreground">{t("thermal_mode_arabic_always_drawn")}</p>
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
