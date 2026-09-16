// What this phone is and how to take it off the shop.
//
// Everything here is read-only except the two ways out. "Forget this phone"
// throws away the device token locally; it does not revoke anything on the
// server, and the copy says so — a stolen phone is unpaired from the till
// computer's device list, not from the stolen phone.

import { useRouter } from "expo-router";
import { useEffect, useState } from "react";
import { View } from "react-native";

import { Button, Callout, Screen, Text } from "../../components/ui";
import { useTheme } from "../../design/theme";
import { API_BASE } from "../../lib/api";
import { list } from "../../lib/queue";
import { useSession } from "../../providers/SessionProvider";

export default function Settings() {
  const theme = useTheme();
  const router = useRouter();
  const { session, device, signOut, deviceLost } = useSession();
  const [queued, setQueued] = useState(0);

  useEffect(() => {
    void list().then((q) => setQueued(q.length));
  }, []);

  return (
    <Screen scroll>
      <View style={{ gap: theme.space[4], paddingTop: theme.space[4] }}>
        <View style={{ flexDirection: "row", alignItems: "center", gap: theme.space[3] }}>
          <Button title="Back" variant="secondary" onPress={() => router.back()} />
          <Text variant="title">This phone</Text>
        </View>

        <Row label="Signed in as" value={session === null ? "nobody" : `${session.name} · ${session.role}`} />
        <Row label="Till computer" value={API_BASE} />
        {/* The first eight characters are enough to tell two phones apart in
            the till computer's device list, and short enough to read out
            over the counter. The rest is a credential. */}
        <Row label="Paired as" value={device === null ? "not paired" : `${device.slice(0, 8)}…`} />
        <Row label="Sales waiting to be sent" value={String(queued)} />

        {queued > 0 && (
          <Callout tone="info">
            Unpairing now would leave those sales unsent. Get back on the shop Wi-Fi and send them
            from the till screen first.
          </Callout>
        )}

        <View style={{ gap: theme.space[2], paddingTop: theme.space[2] }}>
          <Button title="Sign out" variant="secondary" onPress={() => void signOut()} />
          <Button
            title="Forget this phone"
            variant="danger"
            onPress={() => {
              void deviceLost("This phone was unpaired here. Ask a manager for a fresh QR.");
              router.replace("/pair");
            }}
          />
        </View>
      </View>
    </Screen>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  const theme = useTheme();
  return (
    <View style={{ gap: theme.space[1] }}>
      <Text variant="label" tone="secondary">
        {label}
      </Text>
      <Text variant="body">{value}</Text>
    </View>
  );
}
