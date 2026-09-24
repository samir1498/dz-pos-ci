// The language and theme choices, in one fixed corner, reachable from
// every screen the app shows including first setup and sign-in (T2/T3,
// `context/plans/20260924-shop-manual-test-findings.md`), modelled on
// ObserveOne's `ThemeSwitcher.tsx`, a fixed pill in the corner of every
// route rather than a control living inside one screen's own layout.
//
// Mounted once, in `routes/__root.tsx`, above the screen the session status
// picks (first setup, sign-in, or the shell): that is the one place common
// to all three, which is the whole point, since neither the shell's topbar
// nor the sign-in card exists before a status is known. It used to be two
// controls, one drawn in `AppShell`'s topbar and a second, non-floating
// `LanguageSwitcher` on the sign-in card; both are gone from there now, so
// a signed-in shop sees the language and the theme in exactly one place,
// not the topbar and the corner at once.
//
// Fixed at the corner on the reading side's start (`start-4`, physical left
// in French and English, physical right in Arabic) and the Toaster
// (`AppShell.tsx`) sits at the opposite physical corner in both directions,
// so the two never overlap. `z-40`, one below `LockScreen`'s `z-50`: the
// lock overlay already paints the whole window opaque, which is what keeps
// this widget out of reach while the till is locked, the same as it always
// was for the topbar's own switches sitting under that overlay.

import { ThemeSwitcher } from "@/components/ThemeSwitcher";
import { LanguageSwitcher } from "@/i18n/LanguageSwitcher";

export function FloatingControls() {
  return (
    <div
      data-testid="floating-controls"
      className="fixed bottom-4 start-4 z-40 flex items-center gap-2"
    >
      <LanguageSwitcher />
      <ThemeSwitcher />
    </div>
  );
}
