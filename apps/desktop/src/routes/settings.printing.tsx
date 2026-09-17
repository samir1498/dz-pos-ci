// What the shop prints: for now, which layout a facture is drawn in.

import { createFileRoute } from "@tanstack/react-router";

import { FactureLayoutPanel } from "@/components/settings/FactureLayoutPanel";
import { SettingsLoad } from "@/components/settings/SettingsLoad";

export const Route = createFileRoute("/settings/printing")({ component: PrintingRoom });

export function PrintingRoom() {
  return <SettingsLoad>{(settings) => <FactureLayoutPanel settings={settings} />}</SettingsLoad>;
}
