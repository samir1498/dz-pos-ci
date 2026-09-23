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
//
// `afterSignIn` is where a signed-in phone lands (C7): `app/sign-in.tsx`
// passes "/till", `app-clinic/sign-in.tsx` passes "/settings" — the one
// signed-in screen left once a build carries no till.

import { PIN_DIGITS, type SessionDto, type StaffDto } from "@dzpos/shared";
import { useQuery } from "@tanstack/react-query";
import { Redirect, useRouter } from "expo-router";
import { useEffect, useState } from "react";
import { Pressable, View } from "react-native";

import { Button, Callout, Field, PinBoxes, Screen, Text } from "../components/ui";
import { ApiRefusal, call, get } from "../lib/api";
import type { Say } from "../lib/outcome";
import { useTheme } from "../design/theme";
import { useTranslation } from "../providers/LanguageProvider";
import { useSession } from "../providers/SessionProvider";

export default function SignIn({ afterSignIn }: { afterSignIn: string }) {
  const theme = useTheme();
  const router = useRouter();
  const { t } = useTranslation();
  const { device, ready, signedIn, lost, deviceLost } = useSession();
  const [person, setPerson] = useState<StaffDto | null>(null);
  const [pin, setPin] = useState("");
  const [password, setPassword] = useState("");
  const [problem, setProblem] = useState<Say | null>(null);
  const [busy, setBusy] = useState(false);

  // Never served from the cache: the owner sets a new cashier's PIN on the
  // desktop while this screen sits open, and a five-minute-old list would
  // still open the password box for them. Every return to the list reads
  // it again.
  const staff = useQuery({
    queryKey: ["staff", device],
    queryFn: () => get<StaffDto[]>("/auth/staff", { deviceToken: device }),
    enabled: ready && device !== null,
    staleTime: 0,
    refetchOnMount: "always",
  });

  // A revoked phone finds out here, before anyone types anything. The
  // staff list is behind the device gate (crates/api/src/router.rs, the
  // `phone_auth` router), so a phone a manager has just unpaired gets 401
  // device_refused on the read that fills this screen and never on the
  // login call below, which is where the same answer is handled. Without
  // this the screen said it could not read the staff list, the retry earned
  // the same 401 for ever, and nobody was sent to find a manager.
  //
  // Clearing the device is enough to move: the redirect above fires on the
  // next render, since `device` is then null.
  useEffect(() => {
    const failure = staff.error;
    if (failure instanceof ApiRefusal && failure.outcome.kind === "pair-again") {
      void deviceLost(failure.outcome.say);
    }
  }, [staff.error, deviceLost]);

  function pick(next: StaffDto | null) {
    setPerson(next);
    setPin("");
    setPassword("");
    setProblem(null);
    if (next === null) void staff.refetch();
  }

  // The two bodies are built here and nowhere else: a body carrying a name
  // beside a PIN is a caller that has not decided which door it is at, and
  // the server refuses that as malformed rather than as a wrong secret.
  const usesPin = person?.has_pin === true;
  const filled = usesPin ? pin.length === PIN_DIGITS : password !== "";

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
      router.replace(afterSignIn);
      return;
    }
    if (outcome.kind === "pair-again") {
      // The phone itself was revoked while it sat on this screen. Sending it
      // back to pairing is the only thing that helps; a PIN never will.
      await deviceLost(outcome.say);
      router.replace("/pair");
      return;
    }
    // A refused PIN empties the boxes: the next try starts from the first
    // box rather than from a full row that has to be deleted first.
    setPin("");
    setProblem(
      outcome.kind === "queue"
        ? { key: "error_no_answer" }
        : "say" in outcome
          ? outcome.say
          : { key: "auth_refused" },
    );
  }

  // There is no PIN to type on a phone that has never paired: a sign-in
  // needs the device token in the header before the server will read it.
  if (!ready) return <Screen />;
  if (device === null) return <Redirect href="/pair" />;

  return (
    <Screen scroll>
      <View style={{ gap: theme.space[4], paddingTop: theme.space[6] }}>
        <Text variant="title">{person === null ? t("auth_title") : person.name}</Text>
        {lost !== null && <Callout tone="danger">{t(lost.key, lost.vars)}</Callout>}

        {person === null ? (
          staff.isPending ? (
            <Text tone="secondary">{t("auth_staff_loading")}</Text>
          ) : staff.isError ? (
            <>
              {/* The screen knows what failed, so it says what failed. `sayOf`
                  answers about the call, and on this one that is either "no
                  answer from the till computer" or a bare status number,
                  neither of which tells the person in front of the phone
                  that it is the staff list they are waiting on. */}
              <Callout tone="danger">{t("auth_staff_unreadable")}</Callout>
              <Button
                variant="secondary"
                title={t("action_try_again")}
                onPress={() => void staff.refetch()}
              />
            </>
          ) : staff.data.length === 0 ? (
            <Callout tone="danger">{t("auth_nobody_yet")}</Callout>
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
                    {row.has_pin ? "" : ` · ${t("auth_by_password")}`}
                  </Text>
                </Pressable>
              ))}
            </View>
          )
        ) : (
          <>
            {usesPin ? (
              <PinBoxes label={t("auth_pin_label")} value={pin} onChange={setPin} autoFocus />
            ) : (
              <Field
                label={t("auth_password_label")}
                placeholder={t("auth_password_placeholder")}
                autoCapitalize="none"
                autoCorrect={false}
                secureTextEntry
                autoFocus
                value={password}
                onChangeText={setPassword}
              />
            )}
            {problem !== null && <Callout tone="danger">{t(problem.key, problem.vars)}</Callout>}
            <Button title={t("auth_sign_in")} onPress={signIn} busy={busy} disabled={!filled} />
            <Button variant="secondary" title={t("auth_not_you")} onPress={() => pick(null)} />
          </>
        )}
      </View>
    </Screen>
  );
}
