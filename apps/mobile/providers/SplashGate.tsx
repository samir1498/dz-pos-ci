// Holding the splash until the phone knows who it is.
//
// Reading the device and session tokens off `AsyncStorage` takes a few
// frames. Without this, the splash lifts on the first paint, `app/index`
// renders with `ready` still false, and whoever is holding the phone gets a
// blank screen for those frames — or worse, on a slow phone, a glimpse of
// the pairing screen before the redirect to the till.
//
// So the native splash stays up until `SessionProvider` has finished, and
// the first thing anyone sees is the screen they were going to end up on.
//
// The language is waited on for the same reason and it is the louder of the
// two. Lifting the splash first would show the pairing screen in French,
// then swap every sentence to Arabic a frame later, on the one screen a
// cashier meets before they have been told anything about the phone.

import { useEffect, type ReactNode } from "react";
import * as SplashScreen from "expo-splash-screen";

import { useTranslation } from "./LanguageProvider";
import { useSession } from "./SessionProvider";

// Asked for at module scope, before React mounts anything: after the first
// frame there is no splash left to prevent from hiding.
void SplashScreen.preventAutoHideAsync();

export function SplashGate({ children }: { children: ReactNode }) {
  const { ready: sessionReady } = useSession();
  const { ready: languageReady } = useTranslation();
  const ready = sessionReady && languageReady;

  useEffect(() => {
    // Best-effort. A splash that fails to hide would leave the till
    // unusable behind a logo, so the failure is swallowed rather than
    // thrown: the screen underneath is already rendered and correct.
    if (ready) void SplashScreen.hideAsync().catch(() => {});
  }, [ready]);

  return <>{children}</>;
}
