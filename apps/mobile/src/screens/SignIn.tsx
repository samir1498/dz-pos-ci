import { useState } from "react";
import { Button, Text, TextInput, View } from "react-native";

import { saveDevice, saveSession, type Session } from "../lib/session";

type Me = { user_id: number; name: string; role: string };
type LoginAnswer = { me: Me; token: string };

/** Pair this phone, then sign a person in (M7 T3). No camera yet: the QR's
 * pairing token is typed in by hand, which is also how the Maestro proof
 * drives it. Step one trades the token for a device token (which phone);
 * step two trades a PIN or password for a session (which person). */
export function SignIn({
  apiBase,
  launch,
  knownDevice,
  onSignedIn,
}: {
  apiBase: string;
  launch: string;
  /** The device token from an earlier pairing, if this phone still has one.
   * A session that idled out leaves it in place, so the cashier lands on the
   * PIN box rather than on a QR they would have to ask a manager for. */
  knownDevice: string | null;
  onSignedIn: (session: Session) => void;
}) {
  const [deviceToken, setDeviceToken] = useState<string | null>(knownDevice);
  const [pairingToken, setPairingToken] = useState("");
  const [phoneName, setPhoneName] = useState("");
  const [userId, setUserId] = useState("");
  const [pin, setPin] = useState("");
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function post(path: string, body: unknown, device: string | null) {
    const headers: Record<string, string> = {
      "content-type": "application/json",
      Authorization: `Bearer ${launch}`,
    };
    if (device !== null) headers["x-dzpos-device"] = device;
    const res = await fetch(`${apiBase}${path}`, {
      method: "POST",
      headers,
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      const code = await res
        .json()
        .then((b: unknown) => (b as { error?: { code?: string } }).error?.code ?? String(res.status))
        .catch(() => String(res.status));
      throw new Error(code);
    }
    return (await res.json()) as { device_token?: string } & LoginAnswer;
  }

  async function pair() {
    setBusy(true);
    setProblem(null);
    try {
      const answer = await post(
        "/pairing/claim",
        { pairing_token: pairingToken.trim(), device_name: phoneName.trim() || "Phone" },
        null,
      );
      if (!answer.device_token) throw new Error("no-device-token");
      await saveDevice(answer.device_token);
      setDeviceToken(answer.device_token);
    } catch (e) {
      setProblem(e instanceof Error ? e.message : "pair-failed");
    } finally {
      setBusy(false);
    }
  }

  async function signIn(body: unknown) {
    if (deviceToken === null) return;
    setBusy(true);
    setProblem(null);
    try {
      const answer = await post("/auth/login", body, deviceToken);
      const session: Session = {
        deviceToken,
        sessionToken: answer.token,
        name: answer.me.name,
        role: answer.me.role,
      };
      await saveSession(session);
      onSignedIn(session);
    } catch (e) {
      setProblem(e instanceof Error ? e.message : "sign-in-failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <View style={{ flex: 1, padding: 16, gap: 12 }}>
      {deviceToken === null ? (
        <View style={{ gap: 8 }}>
          <Text style={{ fontWeight: "600" }}>Pair this phone</Text>
          <TextInput
            placeholder="Pairing token from the desktop QR"
            value={pairingToken}
            onChangeText={setPairingToken}
            autoCapitalize="none"
            style={{ borderWidth: 1, padding: 8 }}
          />
          <TextInput
            placeholder="Phone name (e.g. Till phone)"
            value={phoneName}
            onChangeText={setPhoneName}
            style={{ borderWidth: 1, padding: 8 }}
          />
          <Button title={busy ? "Pairing…" : "Pair"} onPress={pair} disabled={busy} />
        </View>
      ) : (
        <View style={{ gap: 8 }}>
          <Text style={{ fontWeight: "600" }}>Sign in on this phone</Text>
          <TextInput
            placeholder="User id (PIN sign-in)"
            value={userId}
            onChangeText={setUserId}
            keyboardType="numeric"
            style={{ borderWidth: 1, padding: 8 }}
          />
          <TextInput
            placeholder="PIN"
            value={pin}
            onChangeText={setPin}
            secureTextEntry
            keyboardType="numeric"
            style={{ borderWidth: 1, padding: 8 }}
          />
          <Button
            title={busy ? "Signing in…" : "Sign in with PIN"}
            onPress={() => signIn({ user_id: Number(userId), pin })}
            disabled={busy}
          />
          <TextInput
            placeholder="Name (password sign-in)"
            value={name}
            onChangeText={setName}
            autoCapitalize="none"
            style={{ borderWidth: 1, padding: 8 }}
          />
          <TextInput
            placeholder="Password"
            value={password}
            onChangeText={setPassword}
            secureTextEntry
            style={{ borderWidth: 1, padding: 8 }}
          />
          <Button
            title={busy ? "Signing in…" : "Sign in with password"}
            onPress={() => signIn({ name, password })}
            disabled={busy}
          />
        </View>
      )}
      {problem !== null && <Text>Refused: {problem}</Text>}
    </View>
  );
}
