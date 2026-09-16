import { useEffect, useState } from "react";
import { Text, View } from "react-native";

import { clearMode, loadMode, type Mode } from "./src/lib/mode";
import {
  clearDevice,
  clearSession,
  loadDevice,
  loadSession,
  type Session,
} from "./src/lib/session";
import { Onboarding } from "./src/screens/Onboarding";
import { SignIn } from "./src/screens/SignIn";
import { Till } from "./src/screens/Till";

const API_BASE = process.env.EXPO_PUBLIC_API_URL ?? "http://127.0.0.1:4317";
// The server operator's secret, provisioned at build time: it says this
// phone may reach the server at all (the launch token), nothing about
// which phone or which person — those come from pairing and sign-in.
const LAUNCH = process.env.EXPO_PUBLIC_API_TOKEN ?? "";

export default function App() {
  const [session, setSession] = useState<Session | null | undefined>(undefined);
  // Why the cashier is looking at the sign-in screen again. Without it, a
  // session that idled out mid-shift looks like the app forgot them.
  const [lost, setLost] = useState<string | null>(null);
  const [device, setDevice] = useState<string | null>(null);

  useEffect(() => {
    loadSession().then(setSession).catch(() => setSession(null));
    loadDevice().then(setDevice).catch(() => setDevice(null));
  }, []);

  if (session === undefined) {
    return (
      <View style={{ flex: 1, padding: 16 }}>
        <Text>Loading…</Text>
      </View>
    );
  }

  async function signOut() {
    if (session !== null && session !== undefined) {
      // End it on the server first, while the headers still exist: a token
      // cleared locally but live on the server stays usable until the idle
      // rule kills it. Best-effort — offline, there is nothing to tell, the
      // local copy still goes and the server row dies on idle.
      try {
        await fetch(`${API_BASE}/auth/logout`, {
          method: "POST",
          headers: {
            Authorization: `Bearer ${LAUNCH}`,
            "x-dzpos-device": session.deviceToken,
            "x-dzpos-session": session.sessionToken,
          },
        });
      } catch {
        // offline: nothing to tell
      }
    }
    await clearSession();
    setLost(null);
    setSession(null);
  }

  /** The person's session died — idled out, or ended elsewhere. The phone
   * is still paired, so only the person has to come back. */
  async function sessionLost(message: string) {
    await clearSession();
    setLost(message);
    setSession(null);
  }

  /** The phone itself is no longer trusted (revoked from settings). Clearing
   * the session clears the device token with it, so the next screen is the
   * pairing one and a manager has to hand over a fresh QR. */
  async function deviceLost(message: string) {
    await clearSession();
    await clearDevice();
    setDevice(null);
    setLost(message);
    setSession(null);
  }

  return (
    <View style={{ flex: 1 }}>
      {session === null ? (
        <>
          {lost !== null && <Text style={{ padding: 16 }}>{lost}</Text>}
          <SignIn
            apiBase={API_BASE}
            launch={LAUNCH}
            knownDevice={device}
            onSignedIn={(s) => {
              setDevice(s.deviceToken);
              setLost(null);
              setSession(s);
            }}
          />
        </>
      ) : (
        <Till
          apiBase={API_BASE}
          launch={LAUNCH}
          session={session}
          onSignOut={signOut}
          onSessionLost={sessionLost}
          onDeviceLost={deviceLost}
        />
      )}
      <View style={{ padding: 8, alignItems: "center" }}>
        <Text style={{ fontSize: 12, color: "#666" }}>pair • ticket • products • customers • more</Text>
      </View>
    </View>
  );
}
