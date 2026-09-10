// The router's outermost route. It sets the page direction and hands the
// screen to the shell; the sidebar, the topbar and everything in them live in
// components/AppShell.tsx, because the shell is a component the kit page can
// render too and this file is routing.
//
// The screens inside are untouched by this. None of them knows it is in a
// shell, which is what let the shell land before the screens were rewritten
// on the kit.

import { createRootRoute, Outlet } from "@tanstack/react-router";

import { AppShell } from "@/components/AppShell";
import { useTranslation } from "@/i18n";

export const Route = createRootRoute({ component: RootLayout });

function RootLayout() {
  const { dir } = useTranslation();
  return (
    // The provider already writes `dir` on the document element; this repeats
    // it on the tree so a component reading its own inherited direction (and
    // a test rendering a screen without the document) agrees with the page.
    <div dir={dir} className="min-h-screen">
      <AppShell>
        <Outlet />
      </AppShell>
    </div>
  );
}
