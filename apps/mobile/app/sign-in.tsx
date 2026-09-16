// Signing a person in on a phone that is already paired.
//
// Behind the device gate but outside the session one: a sign-in behind a
// session guard would be a lock with its key inside, but a LAN holder of
// only the launch token mints no sessions. A cashier signs in with a PIN, an
// owner with a password — same route, because the server decides which by
// what it is given.

import type { SessionDto } from "@dzpos/shared";
import { Redirect, useRouter } from "expo-router";
import { useState } from "react";
import { View } from "react-native";

import { Button, Callout, Field, Screen, Text } from "../components/ui";
import { call } from "../lib/api";
import { useTheme } from "../design/theme";
import { useSession } from "../providers/SessionProvider";

export default function SignIn() {
  const theme = useTheme();
  const router = useRouter();
  const { device, ready, signedIn, lost, deviceLost } = useSession();
  const [userId, setUserId] = useState("");
  const [pin, setPin] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function signIn() {
    if (device === null) return;
    setBusy(true);
    setProblem(null);
    const { outcome, body } = await call<SessionDto>("/auth/login", {
      method: "POST",
      credentials: { deviceToken: device },
      body: JSON.stringify({ user_id: Number(userId), pin }),
    });
    setBusy(false);

    if (outcome.kind === "rang" && body !== null) {
      await signedIn({
        deviceToken: device,
        sessionToken: body.token,
        name: body.me.name,
        role: body.me.role,
      });
      router.replace("/till");
      return;
    }
    if (outcome.kind === "pair-again") {
      // The phone itself was revoked while it sat on this screen. Sending it
      // back to pairing is the only thing that helps; a PIN never will.
      await deviceLost(outcome.message);
      router.replace("/pair");
      return;
    }
    setProblem(
      outcome.kind === "queue"
        ? "No answer from the till computer. Check the Wi-Fi and try again."
        : "message" in outcome
          ? outcome.message
          : "That did not sign you in.",
    );
  }

  // There is no PIN to type on a phone that has never paired: a sign-in
  // needs the device token in the header before the server will read it.
  if (!ready) return <Screen />;
  if (device === null) return <Redirect href="/pair" />;

  return (
    <Screen scroll>
      <View style={{ gap: theme.space[4], paddingTop: theme.space[6] }}>
        <Text variant="title">Sign in on this phone</Text>
        {lost !== null && <Callout tone="danger">{lost}</Callout>}
        <Field
          label="Your number"
          placeholder="e.g. 2"
          keyboardType="number-pad"
          value={userId}
          onChangeText={setUserId}
        />
        <Field
          label="PIN"
          placeholder="••••"
          keyboardType="number-pad"
          secureTextEntry
          value={pin}
          onChangeText={setPin}
        />
        {problem !== null && <Callout tone="danger">{problem}</Callout>}
        <Button
          title="Sign in"
          onPress={signIn}
          busy={busy}
          disabled={userId.trim() === "" || pin === ""}
        />
      </View>
    </Screen>
  );
}
