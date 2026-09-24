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
import { FirstSetupScreen } from "@/components/FirstSetupScreen";
import { FloatingControls } from "@/components/FloatingControls";
import { LockScreen } from "@/components/LockScreen";
import { ShopIdentityOnboarding } from "@/components/ShopIdentityOnboarding";
import { SignInScreen } from "@/components/SignInScreen";
import { useTranslation } from "@/i18n";
import { useSession } from "@/lib/session";

export const Route = createRootRoute({ component: RootLayout });

function RootLayout() {
  const { dir } = useTranslation();
  const { status, locked, freshOwner } = useSession();
  return (
    // The provider already writes `dir` on the document element; this repeats
    // it on the tree so a component reading its own inherited direction (and
    // a test rendering a screen without the document) agrees with the page.
    <div dir={dir} className="min-h-screen">
      {/* The one floating control, on every status: `checking` included,
          because a slow first `/health` should not make the theme flash
          back to Comptoir for the moment before a screen exists to hold
          the topbar's own copy. */}
      <FloatingControls />
      {status === "checking" ? null : status === "needs-setup" ? (
        <FirstSetupScreen />
      ) : status === "signed-out" ? (
        <SignInScreen />
      ) : status === "signed-in" && freshOwner ? (
        // T24/T38: the shop's own identity, asked once right after the
        // owner claims the shop, skippable. Shown instead of the shell
        // (never behind it) so the till's own opening popup does not fire
        // underneath a screen the owner has not gotten past yet.
        <ShopIdentityOnboarding />
      ) : (
        // The shell and the route inside it stay mounted whether or not the
        // till is locked: `LockScreen` below is an overlay on top of this,
        // never a replacement for it, which is what keeps a cart on the till
        // alive while the screen is covered.
        //
        // `inert` while locked, not just visually covered: the overlay sits
        // above this in the stacking order, which stops a click, but a key
        // typed by a scanner or a keyboard goes to whatever element holds
        // DOM focus, not to whatever is on top, and nothing about being
        // covered moves focus away on its own. `inert` does: the browser
        // blurs a focused element the moment its subtree turns inert, drops
        // it from the tab order, and makes `.focus()` a no-op on it, so a
        // search box that had focus when the till went idle stops being
        // reachable at all, by a click, a Tab, a scan or an effect calling
        // `.focus()` on it again. `className="contents"` keeps the wrapper
        // this needs out of AppShell's own layout — `inert` cascades to a
        // whole subtree regardless of `display`, so it costs nothing here.
        <div inert={locked} className="contents">
          <AppShell>
            <Outlet />
          </AppShell>
        </div>
      )}
      {status === "signed-in" && locked ? <LockScreen /> : null}
    </div>
  );
}
