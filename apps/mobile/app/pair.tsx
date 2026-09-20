// Pairing: the phone trades a QR's pairing token for a device token.
//
// Outside the auth gate, because this is how a device token comes to exist —
// a claim behind a session guard would be a lock with its key inside. The
// server agrees: `/pairing/claim` is the one route that carries neither the
// device gate nor the session one.
//
// The camera is the way in and the keyboard is the way back. A pairing token
// is sixty-four hexadecimal characters and lives sixty seconds, which is not
// something a person types under a customer's eyes — but a cracked lens, a
// denied permission or a shop lit like a cave all happen, and a screen with
// no second path strands the cashier. So the typed box survives, one tap
// away, posting through exactly the same claim.

import type { DeviceTokenDto } from "@dzpos/shared";
import { CameraView, useCameraPermissions } from "expo-camera";
import { useRouter } from "expo-router";
import { useCallback, useRef, useState } from "react";
import { View } from "react-native";

import { Button, Callout, Field, Screen, Text } from "../components/ui";
import { call } from "../lib/api";
import type { Say } from "../lib/outcome";
import { tokenFromScan } from "../lib/pairing";
import { useTheme } from "../design/theme";
import { useTranslation } from "../providers/LanguageProvider";
import { useSession } from "../providers/SessionProvider";

export default function Pair() {
  const theme = useTheme();
  const router = useRouter();
  const { t } = useTranslation();
  const { paired, lost } = useSession();
  const [permission, requestPermission] = useCameraPermissions();
  const [typing, setTyping] = useState(false);
  const [token, setToken] = useState("");
  const [name, setName] = useState("");
  const [problem, setProblem] = useState<Say | null>(null);
  const [busy, setBusy] = useState(false);
  // A QR in frame fires this callback many times a second. Without the latch
  // the phone posts the same single-use token a dozen times and every claim
  // after the first comes back refused, so the cashier watches a successful
  // pairing report itself as a failure.
  const claiming = useRef(false);

  const claim = useCallback(
    async (pairingToken: string) => {
      if (claiming.current) return;
      claiming.current = true;
      setBusy(true);
      setProblem(null);
      const { outcome, body } = await call<DeviceTokenDto>("/pairing/claim", {
        method: "POST",
        body: JSON.stringify({
          pairing_token: pairingToken,
          device_name: name.trim() === "" ? t("pairing_default_device_name") : name.trim(),
        }),
      });
      setBusy(false);

      if (outcome.kind !== "rang" || body?.device_token == null) {
        // A pairing token lives sixty seconds and is single-use, so the
        // honest message is usually "ask for another one" rather than
        // anything the cashier did wrong.
        setProblem(
          outcome.kind === "queue"
            ? { key: "error_no_answer" }
            : "say" in outcome
              ? outcome.say
              : { key: "pairing_code_refused" },
        );
        // Released only on failure: a success navigates away, and a latch
        // that reopened on the way out would let a lingering frame fire a
        // second claim against a token already spent.
        claiming.current = false;
        return;
      }
      await paired(body.device_token);
      router.replace("/sign-in");
    },
    [name, paired, router, t],
  );

  const scanning = !typing && permission?.granted === true;

  return (
    <Screen scroll>
      <View style={{ gap: theme.space[4], paddingTop: theme.space[6] }}>
        <Text variant="title">{t("pairing_title")}</Text>
        <Text variant="body" tone="secondary">
          {t("pairing_hint")}
        </Text>
        {lost !== null && <Callout tone="danger">{t(lost.key, lost.vars)}</Callout>}

        {scanning ? (
          <View
            style={{
              // Square, because a QR is. A viewfinder the shape of the phone
              // makes people hold the code at the wrong distance.
              aspectRatio: 1,
              borderRadius: theme.radius.md,
              overflow: "hidden",
              backgroundColor: theme.colors.surface.raised,
            }}
          >
            <CameraView
              style={{ flex: 1 }}
              facing="back"
              barcodeScannerSettings={{ barcodeTypes: ["qr"] }}
              onBarcodeScanned={({ data }) => {
                const found = tokenFromScan(data);
                if (found === null) {
                  setProblem({ key: "pairing_not_a_code" });
                  return;
                }
                void claim(found);
              }}
            />
          </View>
        ) : null}

        {!scanning && !typing ? (
          <View style={{ gap: theme.space[3] }}>
            <Text variant="body">{t("pairing_camera_hint")}</Text>
            <Button
              title={t("pairing_use_camera")}
              onPress={() => void requestPermission()}
              disabled={permission?.canAskAgain === false}
            />
            {permission?.canAskAgain === false && (
              <Callout tone="info">{t("pairing_camera_blocked")}</Callout>
            )}
          </View>
        ) : null}

        {typing ? (
          <Field
            label={t("pairing_code_label")}
            placeholder={t("pairing_code_placeholder", { count: 64 })}
            autoCapitalize="none"
            autoCorrect={false}
            value={token}
            onChangeText={setToken}
          />
        ) : null}

        <Field
          label={t("pairing_device_name_label")}
          placeholder={t("pairing_default_device_name")}
          value={name}
          onChangeText={setName}
        />

        {problem !== null && <Callout tone="danger">{t(problem.key, problem.vars)}</Callout>}

        {typing ? (
          <Button
            title={t("pairing_submit")}
            onPress={() => void claim(token.trim())}
            busy={busy}
            disabled={token.trim() === ""}
          />
        ) : (
          busy && (
            <Text variant="body" tone="secondary">
              {t("pairing_in_progress")}
            </Text>
          )
        )}

        <Button
          title={typing ? t("pairing_switch_to_camera") : t("pairing_switch_to_code")}
          variant="secondary"
          onPress={() => {
            setProblem(null);
            claiming.current = false;
            setTyping((was) => !was);
          }}
        />
      </View>
    </Screen>
  );
}
