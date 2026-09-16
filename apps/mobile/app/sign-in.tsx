// Signing a person in on a phone that is already paired.
//
// Behind the device gate but outside the session one: a sign-in behind a
// session guard would be a lock with its key inside, but a LAN holder of
// only the launch token mints no sessions.
//
// The screen opens on the shop's names, read from `GET /auth/staff` with the
// device token alone. A cashier taps theirs and types only a PIN; the user id
// `LoginDto::Pin` wants is the users table's row number, which nobody at a
// counter knows, and the first version of this screen asked for it anyway.
// A name whose fiche has no PIN opens the password box instead, with the
// name already filled: that is the owner, who claimed the shop with a
// password and may never set a PIN. The refusal for a wrong secret names no
// field on purpose (`core::Error::AuthRefused`), so the screen has to steer
// by `has_pin` up front rather than let a door that cannot open read as a
// mistyped PIN.

import type { SessionDto, StaffDto } from "@dzpos/shared";
import { useQuery } from "@tanstack/react-query";
import { Redirect, useRouter } from "expo-router";
import { useState } from "react";
import { Pressable, View } from "react-native";

import { Button, Callout, Field, Screen, Text } from "../components/ui";
import { call, get } from "../lib/api";
import { useTheme } from "../design/theme";
import { useSession } from "../providers/SessionProvider";

export default function SignIn() {
  const theme = useTheme();
  const router = useRouter();
  const { device, ready, signedIn, lost, deviceLost } = useSession();
  const [person, setPerson] = useState<StaffDto | null>(null);
  const [pin, setPin] = useState("");
  const [password, setPassword] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const staff = useQuery({
    queryKey: ["staff", device],
    queryFn: () => get<StaffDto[]>("/auth/staff", { deviceToken: device }),
    enabled: ready && device !== null,
  });

  function pick(next: StaffDto | null) {
    setPerson(next);
    setPin("");
    setPassword("");
    setProblem(null);
  }

  // The two bodies are built here and nowhere else: a body carrying a name
  // beside a PIN is a caller that has not decided which door it is at, and
  // the server refuses that as malformed rather than as a wrong secret.
  const usesPin = person?.has_pin === true;
  const filled = usesPin ? pin !== "" : password !== "";

  async function signIn() {
    if (device === null || person === null) return;
    setBusy(true);
    setProblem(null);
    const { outcome, body } = await call<SessionDto>("/auth/login", {
      method: "POST",
      credentials: { deviceToken: device },
      body: JSON.stringify(
        usesPin ? { user_id: person.id, pin } : { name: person.name, password },
      ),
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
        <Text variant="title">{person === null ? "Who is signing in?" : person.name}</Text>
        {lost !== null && <Callout tone="danger">{lost}</Callout>}

        {person === null ? (
          staff.isPending ? (
            <Text tone="secondary">Reading the staff list…</Text>
          ) : staff.isError ? (
            <>
              <Callout tone="danger">
                {staff.error instanceof Error
                  ? staff.error.message
                  : "Could not read the staff list."}
              </Callout>
              <Button variant="secondary" title="Try again" onPress={() => void staff.refetch()} />
            </>
          ) : staff.data.length === 0 ? (
            <Callout tone="danger">Nobody can sign in on this shop yet.</Callout>
          ) : (
            <View style={{ gap: theme.space[2] }}>
              {staff.data.map((row) => (
                <Pressable
                  key={row.id}
                  accessibilityRole="button"
                  onPress={() => pick(row)}
                  style={({ pressed }) => ({
                    minHeight: theme.layout["touch-min"],
                    paddingHorizontal: theme.space[4],
                    paddingVertical: theme.space[3],
                    borderRadius: theme.radius.sm,
                    borderWidth: 1,
                    borderColor: theme.colors.border.default,
                    backgroundColor: pressed
                      ? theme.colors.surface.raised
                      : theme.colors.surface.card,
                    justifyContent: "center",
                  })}
                >
                  <Text variant="label">{row.name}</Text>
                  <Text variant="caption" tone="tertiary">
                    {row.role}
                    {row.has_pin ? "" : " · password"}
                  </Text>
                </Pressable>
              ))}
            </View>
          )
        ) : (
          <>
            {usesPin ? (
              <Field
                label="PIN"
                placeholder="••••"
                keyboardType="number-pad"
                secureTextEntry
                autoFocus
                value={pin}
                onChangeText={setPin}
              />
            ) : (
              <Field
                label="Password"
                placeholder="••••••••"
                autoCapitalize="none"
                autoCorrect={false}
                secureTextEntry
                autoFocus
                value={password}
                onChangeText={setPassword}
              />
            )}
            {problem !== null && <Callout tone="danger">{problem}</Callout>}
            <Button title="Sign in" onPress={signIn} busy={busy} disabled={!filled} />
            <Button variant="secondary" title="Not you" onPress={() => pick(null)} />
          </>
        )}
      </View>
    </Screen>
  );
}
