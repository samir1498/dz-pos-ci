import { useEffect, useState } from "react";
import { Text, View } from "react-native";

import { clearMode, loadMode, type Mode } from "./src/lib/mode";
import { clearSession, loadSession, type Session } from "./src/lib/session";
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

  useEffect(() => {
    loadSession().then(setSession).catch(() => setSession(null));
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
    setSession(null);
  }

  return (
    <View style={{ flex: 1 }}>
      {session === null ? (
        <SignIn apiBase={API_BASE} launch={LAUNCH} onSignedIn={setSession} />
      ) : (
        <Till apiBase={API_BASE} launch={LAUNCH} session={session} onSignOut={signOut} />
      )}
      <View style={{ padding: 8, alignItems: "center" }}>
        <Text style={{ fontSize: 12, color: "#666" }}>pair • ticket • products • customers • more</Text>
      </View>
    </View>
  );
}
