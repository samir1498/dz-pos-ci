// What this phone is and how to take it off the shop.
//
// Everything here is read-only except the two ways out. "Forget this phone"
// throws away the device token locally; it does not revoke anything on the
// server, and the copy says so — a stolen phone is unpaired from the till
// computer's device list, not from the stolen phone.
//
// Shared across both router roots (C7): `app/(signed-in)/settings.tsx` and
// `app-clinic/(signed-in)/settings.tsx` both re-export this file unchanged.
// "the till computer" in the copy below names the desktop machine this
// phone pairs to, whatever trade it runs; a cabinet build still says it,
// which reads oddly and is open for Samir to rename once a clinic phone
// screen exists to compare it against.

import { useRouter } from "expo-router";
import { useEffect, useState } from "react";
import { View } from "react-native";

import { Button, Callout, Screen, Text } from "../../components/ui";
import { useTheme } from "../../design/theme";
import { API_BASE } from "../../lib/api";
import { list } from "../../lib/queue";
import { useTranslation } from "../../providers/LanguageProvider";
import { useSession } from "../../providers/SessionProvider";

export default function Settings() {
  const theme = useTheme();
  const router = useRouter();
  const { t } = useTranslation();
  const { session, device, signOut, deviceLost } = useSession();
  const [queued, setQueued] = useState(0);

  useEffect(() => {
    void list().then((q) => setQueued(q.length));
  }, []);

  return (
    <Screen scroll>
      <View style={{ gap: theme.space[4], paddingTop: theme.space[4] }}>
        <View style={{ flexDirection: "row", alignItems: "center", gap: theme.space[3] }}>
          <Button title={t("action_back")} variant="secondary" onPress={() => router.back()} />
          <Text variant="title">{t("settings_title")}</Text>
        </View>

        <Row
          label={t("settings_signed_in_as")}
          value={session === null ? t("settings_nobody") : `${session.name} · ${session.role}`}
        />
        <Row label={t("settings_till_computer")} value={API_BASE} />
        {/* The first eight characters are enough to tell two phones apart in
            the till computer's device list, and short enough to read out
            over the counter. The rest is a credential. */}
        <Row
          label={t("settings_paired_as")}
          value={device === null ? t("settings_not_paired") : `${device.slice(0, 8)}…`}
        />
        <Row label={t("settings_queue_waiting")} value={String(queued)} />

        {queued > 0 && <Callout tone="info">{t("settings_unpair_would_lose")}</Callout>}

        <View style={{ gap: theme.space[2], paddingTop: theme.space[2] }}>
          <Button title={t("auth_sign_out")} variant="secondary" onPress={() => void signOut()} />
          <Button
            title={t("settings_forget_phone")}
            variant="danger"
            onPress={() => {
              void deviceLost({ key: "pairing_lost" });
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
