// The two doors onto the app: name and password (the office), and the
// till's PIN pad. `__root.tsx` shows this instead of the shell while nobody
// is signed in, so the screen owns the whole window and no route beneath it
// is reachable past it. The office door is the default: the owner claimed
// the shop with a password, and a cashier's PIN is set later.
//
// The PIN pad has no list of faces to pick from. `POST /auth/login` takes a
// user id and a PIN (`LoginDto::Pin`), and no route today hands the desktop
// a list of ids to show as a picker: that is user management, which lands
// with T8. Until then a cashier types their id on the same pad they type
// the PIN with, one stage then the other; the two digit strings never
// become numbers the way an amount does (`keyedDigits`, not `keyedAmount`,
// because `0512` and `512` are different PINs).

import { KeyRound, User } from "lucide-react";
import { useState } from "react";

import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Keypad, keyedDigits, type KeypadKey } from "@/components/Keypad";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Wordmark } from "@/components/Wordmark";
import { useTranslation } from "@/i18n";
import { LanguageSwitcher } from "@/i18n/LanguageSwitcher";
import { useLoginAttempt } from "@/lib/useLoginAttempt";
import { useSession } from "@/lib/session";

const MAX_ID_DIGITS = 6;
const MAX_PIN_DIGITS = 6;

/** The figure a display box shows: the id in the clear (not a secret), the
 * PIN masked (it is one), so a shoulder behind the counter reads a row of
 * dots and not the four digits it is made of. */
function maskedOrPlain(digits: string, mask: boolean): string {
  return mask ? "•".repeat(digits.length) : digits;
}

function PinPad() {
  const { t } = useTranslation();
  const { signInWithPin } = useSession();
  const [stage, setStage] = useState<"id" | "pin">("id");
  const [idDigits, setIdDigits] = useState("");
  const [pinDigits, setPinDigits] = useState("");
  const attempt = useLoginAttempt(() => signInWithPin(Number(idDigits), pinDigits));

  function backToId() {
    setStage("id");
    setPinDigits("");
  }

  function onKey(key: KeypadKey) {
    if (attempt.locked) return;
    if (stage === "id") {
      if (key === "enter") {
        if (idDigits === "") return;
        setStage("pin");
        return;
      }
      setIdDigits((current) => keyedDigits(current, key, MAX_ID_DIGITS));
      return;
    }
    if (key === "enter") {
      if (pinDigits === "") return;
      // Cleared before the call, not after: `attempt.run()` still submits
      // this render's PIN (the callback closed over it), and a right guess
      // unmounts this whole screen before an "after" would ever run, which
      // is a `setState` on an unmounted component.
      setPinDigits("");
      void attempt.run();
      return;
    }
    if (key === "backspace" && pinDigits === "") {
      backToId();
      return;
    }
    setPinDigits((current) => keyedDigits(current, key, MAX_PIN_DIGITS));
  }

  const digits = stage === "id" ? idDigits : pinDigits;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between text-sm text-muted-foreground">
        <span>{stage === "id" ? t("signin_pin_id_label") : t("signin_pin_pin_label")}</span>
        {stage === "pin" ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-auto p-0 text-muted-foreground"
            onClick={backToId}
          >
            {t("signin_pin_change_id")}
          </Button>
        ) : null}
      </div>
      <div
        data-testid="signin-pin-display"
        dir="ltr"
        className="rounded-md border border-border bg-muted px-3 py-2 text-end font-numeric text-2xl tabular-nums"
      >
        {maskedOrPlain(digits, stage === "pin") || " "}
      </div>
      <Keypad onKey={onKey} disabled={attempt.locked} captureWindow />
      <AttemptFeedback attempt={attempt} />
    </div>
  );
}

function PasswordForm() {
  const { t } = useTranslation();
  const { signInWithPassword } = useSession();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const attempt = useLoginAttempt(() => signInWithPassword(name, password));

  return (
    <form
      className="flex flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        if (attempt.locked || name === "" || password === "") return;
        void attempt.run();
      }}
    >
      <FormField label={t("signin_password_name_label")}>
        {(field) => (
          <Input
            {...field}
            data-testid="signin-name"
            value={name}
            disabled={attempt.locked}
            autoComplete="username"
            onChange={(event) => setName(event.target.value)}
          />
        )}
      </FormField>
      <FormField label={t("signin_password_password_label")}>
        {(field) => (
          <Input
            {...field}
            data-testid="signin-password"
            type="password"
            value={password}
            disabled={attempt.locked}
            autoComplete="current-password"
            onChange={(event) => setPassword(event.target.value)}
          />
        )}
      </FormField>
      <Button
        type="submit"
        data-testid="signin-submit"
        disabled={attempt.locked || name === "" || password === ""}
      >
        {t("signin_password_submit")}
      </Button>
      <AttemptFeedback attempt={attempt} />
    </form>
  );
}

function AttemptFeedback({
  attempt,
}: {
  attempt: ReturnType<typeof useLoginAttempt>;
}) {
  const { t } = useTranslation();
  if (attempt.retryAfter !== null && attempt.retryAfter > 0) {
    return (
      <p data-testid="signin-retry" role="alert" className="text-sm text-fg-danger">
        {t("signin_error_locked_out")} {t("signin_retry_seconds").replace("{n}", String(attempt.retryAfter))}
      </p>
    );
  }
  if (attempt.error !== null) {
    return (
      <p data-testid="signin-error" role="alert" className="text-sm text-fg-danger">
        {t(attempt.error)}
      </p>
    );
  }
  return null;
}

export function SignInScreen() {
  const { t } = useTranslation();
  const [mode, setMode] = useState<"pin" | "password">("password");

  return (
    <div
      data-testid="signin-screen"
      className="flex min-h-screen flex-col items-center justify-center bg-background p-4"
    >
      <div className="w-full max-w-sm self-end sm:self-center">
        <LanguageSwitcher className="ms-auto" />
      </div>
      <Card className="mt-4 w-full max-w-sm">
        <CardHeader className="items-center text-center">
          <Wordmark className="mb-2" />
          <CardTitle>{mode === "pin" ? t("signin_pin_title") : t("signin_password_title")}</CardTitle>
          <CardDescription>
            {mode === "pin" ? t("signin_pin_subtitle") : t("signin_password_subtitle")}
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          {mode === "pin" ? <PinPad /> : <PasswordForm />}
          <Button
            type="button"
            variant="ghost"
            data-testid="signin-mode-switch"
            className="gap-2"
            onClick={() => setMode(mode === "pin" ? "password" : "pin")}
          >
            <Icon as={mode === "pin" ? KeyRound : User} size={18} />
            {mode === "pin" ? t("signin_pin_switch_to_password") : t("signin_password_switch_to_pin")}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
