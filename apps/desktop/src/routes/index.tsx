// The till is the home for a retail build; a build without retail has no
// till to open on, so the home is the first screen the trades it does
// carry offer (C6). The redirect runs before the route loads, which keeps
// "Dinar" from flashing on every launch.
//
// `to` is a plain `string`, the way `AppShell`'s `NavItem.to` and
// `settings.tsx`'s `SectionItem.to` already are: the home for a build
// without `retail` is a path this file cannot know exists in the
// registered route tree at all (`patients.tsx`/`queue.tsx` are excluded
// from that tree the moment `clinic` is not in `VITE_DINAR_MODULES`), so a
// literal here would refuse to compile the moment retail is left out.

import { createFileRoute, redirect } from "@tanstack/react-router";

import { builtModules, type Module } from "@/lib/modules";

/** The one screen each trade opens on. Retail wins when both are built,
 *  the way the shop's own home always has: a cabinet with a shop side is
 *  still a shop first. */
const HOME: Readonly<Record<Module, string>> = {
  retail: "/till",
  clinic: "/queue",
};

export function homeRoute(modules: readonly Module[]): string {
  if (modules.includes("retail")) return HOME.retail;
  const first = modules[0];
  return first === undefined ? HOME.retail : HOME[first];
}

export const Route = createFileRoute("/")({
  beforeLoad: () => {
    throw redirect({ to: homeRoute(builtModules()) });
  },
});
