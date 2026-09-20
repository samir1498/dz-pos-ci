// What the shop prints: which layout a facture is drawn in, and the
// language every fiscal paper prints in.

import { createFileRoute } from "@tanstack/react-router";

import { FactureLayoutPanel } from "@/components/settings/FactureLayoutPanel";
import { PrintLanguagePanel } from "@/components/settings/PrintLanguagePanel";
import { SettingsLoad } from "@/components/settings/SettingsLoad";

export const Route = createFileRoute("/settings/printing")({ component: PrintingRoom });

export function PrintingRoom() {
  return (
    <SettingsLoad>
      {(settings) => (
        <div className="flex flex-col gap-6">
          <FactureLayoutPanel settings={settings} />
          <PrintLanguagePanel settings={settings} />
        </div>
      )}
    </SettingsLoad>
  );
}
