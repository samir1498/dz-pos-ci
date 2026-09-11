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
import { LockScreen } from "@/components/LockScreen";
import { SignInScreen } from "@/components/SignInScreen";
import { useTranslation } from "@/i18n";
import { useSession } from "@/lib/session";

export const Route = createRootRoute({ component: RootLayout });

function RootLayout() {
  const { dir } = useTranslation();
  const { status, locked } = useSession();
  return (
    // The provider already writes `dir` on the document element; this repeats
    // it on the tree so a component reading its own inherited direction (and
    // a test rendering a screen without the document) agrees with the page.
    <div dir={dir} className="min-h-screen">
      {status === "checking" ? null : status === "signed-out" ? (
        <SignInScreen />
      ) : (
        // The shell and the route inside it stay mounted whether or not the
        // till is locked: `LockScreen` below is an overlay on top of this,
        // never a replacement for it, which is what keeps a cart on the till
        // alive while the screen is covered.
        <AppShell>
          <Outlet />
        </AppShell>
      )}
      {status === "signed-in" && locked ? <LockScreen /> : null}
    </div>
  );
}
