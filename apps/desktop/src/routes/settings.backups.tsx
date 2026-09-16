// How the shop's file is kept, and the one file to send when something has
// to be fixed from outside. Both are about the file rather than the shop,
// which is why they share a room.

import { createFileRoute } from "@tanstack/react-router";

import { BackupsPanel } from "@/components/BackupsPanel";
import { SupportBundlePanel } from "@/components/SupportBundlePanel";

export const Route = createFileRoute("/settings/backups")({ component: BackupsRoom });

export function BackupsRoom() {
  return (
    <>
      <BackupsPanel />
      <SupportBundlePanel />
    </>
  );
}
