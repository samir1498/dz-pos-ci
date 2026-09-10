// Whether the window is narrow enough that the sidebar becomes a sheet.
// shadcn ships this beside its sidebar; the changes here are the two the
// house rules ask for.
//
// The file is `useIsMobile.ts`, not `use-mobile.ts`: hooks are named for the
// hook (context/processes/frontend-conventions).
//
// And `window.matchMedia` is guarded. jsdom implements none, and while
// `src/test/setup.ts` stubs one for the app's own tests, a webview with the
// API turned off would throw on mount here and take the whole shell with it.
// Without the media query the answer is the width alone, which is the same
// answer the listener would settle on a moment later.

import { useEffect, useState } from "react";

const MOBILE_BREAKPOINT = 768;

function narrow(): boolean {
  return window.innerWidth < MOBILE_BREAKPOINT;
}

export function useIsMobile(): boolean {
  const [isMobile, setIsMobile] = useState<boolean>(narrow);

  useEffect(() => {
    const onChange = () => setIsMobile(narrow());
    if (typeof window.matchMedia !== "function") {
      // No media query to listen to, so the resize event is the only signal.
      window.addEventListener("resize", onChange);
      return () => window.removeEventListener("resize", onChange);
    }
    const query = window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT - 1}px)`);
    query.addEventListener("change", onChange);
    // The window may have been resized between the first render and this
    // effect.
    onChange();
    return () => query.removeEventListener("change", onChange);
  }, []);

  return isMobile;
}
