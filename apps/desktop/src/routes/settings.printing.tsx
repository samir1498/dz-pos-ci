// What the shop prints: which layout a facture is drawn in, the language
// every fiscal paper prints in, and which wire its thermal head is sent.

import { createFileRoute } from "@tanstack/react-router";

import { FactureLayoutPanel } from "@/components/settings/FactureLayoutPanel";
import { PrintLanguagePanel } from "@/components/settings/PrintLanguagePanel";
import { SettingsLoad } from "@/components/settings/SettingsLoad";
import { ThermalModePanel } from "@/components/settings/ThermalModePanel";

export const Route = createFileRoute("/settings/printing")({ component: PrintingRoom });

export function PrintingRoom() {
  return (
    <SettingsLoad>
      {(settings) => (
        <div className="flex flex-col gap-6">
          <FactureLayoutPanel settings={settings} />
          <PrintLanguagePanel settings={settings} />
          <ThermalModePanel settings={settings} />
        </div>
      )}
    </SettingsLoad>
  );
}
