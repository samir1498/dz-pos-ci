// Pairing: the phone trades a QR's pairing token for a device token.
//
// Outside the auth gate, because this is how a device token comes to exist —
// a claim behind a session guard would be a lock with its key inside. The
// server agrees: `/pairing/claim` is the one route that carries neither the
// device gate nor the session one.
//
// No camera yet, so the 64 hex characters are typed by hand. That is
// miserable on a phone and is the first thing to fix once a dev build makes
// `expo-camera` available; it is also how the Maestro flow drives it.

import type { DeviceTokenDto } from "@dzpos/shared";
import { useRouter } from "expo-router";
import { useState } from "react";
import { View } from "react-native";

import { Button, Callout, Field, Screen, Text } from "../components/ui";
import { call } from "../lib/api";
import { useTheme } from "../design/theme";
import { useSession } from "../providers/SessionProvider";

export default function Pair() {
  const theme = useTheme();
  const router = useRouter();
  const { paired, lost } = useSession();
  const [token, setToken] = useState("");
  const [name, setName] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function pair() {
    setBusy(true);
    setProblem(null);
    const { outcome, body } = await call<DeviceTokenDto>("/pairing/claim", {
      method: "POST",
      body: JSON.stringify({
        pairing_token: token.trim(),
        device_name: name.trim() === "" ? "Phone" : name.trim(),
      }),
    });
    setBusy(false);

    if (outcome.kind !== "rang" || body?.device_token == null) {
      // A pairing token lives sixty seconds and is single-use, so the honest
      // message is usually "ask for another one" rather than anything the
      // cashier typed wrong.
      setProblem(
        outcome.kind === "queue"
          ? "No answer from the till computer. Check the Wi-Fi and try again."
          : "message" in outcome
            ? outcome.message
            : "That code did not work. Ask for a fresh QR — they last a minute.",
      );
      return;
    }
    await paired(body.device_token);
    router.replace("/sign-in");
  }

  return (
    <Screen scroll>
      <View style={{ gap: theme.space[4], paddingTop: theme.space[6] }}>
        <Text variant="title">Pair this phone</Text>
        <Text variant="body" tone="secondary">
          Ask a manager to show the QR on the till computer, then type the code under it.
        </Text>
        {lost !== null && <Callout tone="danger">{lost}</Callout>}
        <Field
          label="Code from the QR"
          placeholder="64 characters"
          autoCapitalize="none"
          autoCorrect={false}
          value={token}
          onChangeText={setToken}
        />
        <Field
          label="Name for this phone"
          placeholder="Phone"
          value={name}
          onChangeText={setName}
        />
        {problem !== null && <Callout tone="danger">{problem}</Callout>}
        <Button title="Pair" onPress={pair} busy={busy} disabled={token.trim() === ""} />
      </View>
    </Screen>
  );
}
