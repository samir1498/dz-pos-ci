// The till is where whoever sells opens; an owner or a manager (anyone who
// can see the reports) opens on the dashboard instead (T9): a fresh shop
// with no products yet used to greet every role, owner included, with
// "Ouverture de la caisse" asking for a drawer float against a catalogue
// nobody had rung a sale on. A cashier still opens straight on the till,
// which is the one screen their day is spent on.
//
// The redirect reads `me` off the session, which is why it is this route's
// own component and not a `beforeLoad`: `beforeLoad` fires before the
// session has resolved, outside the React tree `useSession` reads, so it
// cannot ask a permission at all. Nothing here renders anything a person
// sees, the way the old `beforeLoad` redirect rendered nothing either:
// `__root.tsx` mounts the shell (and this route inside it) only once
// `status === "signed-in"`, and `me` is already populated by then
// (`useHasPermission`'s own doc makes the same promise for every other
// screen), so "Dinar" never flashes on the way past.

import { Navigate, createFileRoute } from "@tanstack/react-router";

import { builtModules, type Module } from "@/lib/modules";
import { hasPermission, useSession } from "@/lib/session";

/** The one screen each trade opens on for whoever sells, when nothing sends
 *  them to the dashboard instead. */
const HOME: Readonly<Record<Module, string>> = {
  retail: "/till",
  clinic: "/queue",
};

/** `seesReports` is `hasPermission(me, "see_reports")` — the same
 *  permission `AppShell.tsx`'s `NAV` gates the dashboard's own link on —
 *  read here rather than `me.role`, which `role.test.ts` holds every file
 *  under `src/` to never comparing. A build without retail has no
 *  dashboard to send anyone to, so the question does not arise there. */
export function homeRoute(modules: readonly Module[], seesReports: boolean): string {
  if (modules.includes("retail")) return seesReports ? "/dashboard" : HOME.retail;
  const first = modules[0];
  return first === undefined ? HOME.retail : HOME[first];
}

export const Route = createFileRoute("/")({ component: Home });

function Home() {
  const { me } = useSession();
  return <Navigate to={homeRoute(builtModules(), hasPermission(me, "see_reports"))} replace />;
}
