// The two doors onto the app: name and password (the office), and the
// till's PIN pad. `__root.tsx` shows this instead of the shell while nobody
// is signed in, so the screen owns the whole window and no route beneath it
// is reachable past it. The office door is the default: the owner claimed
// the shop with a password, and a cashier's PIN is set later.
//
// The PIN pad opens on a list of names, not a box for an id. `POST
// /auth/login` takes a user id and a PIN (`LoginDto::Pin`), and the id is
// the users table's row number — a thing nobody standing at a counter
// knows. Until `GET /auth/staff` existed the pad had to ask for it anyway,
// which is how a real owner ended up typing "1" and being told the shop
// did not accept the credential. Now a cashier taps their name and types
// only the PIN; the id rides along from the row they tapped. The digit
// string never becomes a number the way an amount does (`keyedDigits`, not
// `keyedAmount`, because `0512` and `512` are different PINs).
//
// A name whose fiche has no PIN is still on the list — hiding it would say
// "you do not exist" to someone who plainly does — but tapping it says so
// and points at the password door, rather than taking four digits that can
// only be refused.

import { useQuery } from "@tanstack/react-query";
import { KeyRound, User } from "lucide-react";
import { useState } from "react";
import type { StaffDto } from "@dzpos/shared";

import { api, staffQueryKey } from "@/api";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Keypad, keyedDigits, type KeypadKey } from "@/components/Keypad";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Wordmark } from "@/components/Wordmark";
import { useTranslation } from "@/i18n";
import { LanguageSwitcher } from "@/i18n/LanguageSwitcher";
import { errorKey } from "@/lib/fields";
import { useLoginAttempt } from "@/lib/useLoginAttempt";
import { useSession } from "@/lib/session";

const MAX_PIN_DIGITS = 6;

/** The list of names to tap. Its own component so the pad below it keeps a
 * fixed hook count whichever of the three answers the query is in. */
function StaffPicker({ onPick }: { onPick: (person: StaffDto) => void }) {
  const { t } = useTranslation();
  const staff = useQuery({ queryKey: staffQueryKey, queryFn: () => api.listStaff() });

  if (staff.isPending) {
    return (
      <p className="text-sm text-muted-foreground" aria-busy="true">
        {t("signin_staff_loading")}
      </p>
    );
  }
  if (staff.isError) {
    return (
      <div className="flex flex-col items-start gap-2">
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(staff.error))}
        </p>
        <Button type="button" variant="outline" size="sm" onClick={() => void staff.refetch()}>
          {t("action_retry")}
        </Button>
      </div>
    );
  }
  if (staff.data.length === 0) {
    return <p className="text-sm text-muted-foreground">{t("signin_staff_empty")}</p>;
  }
  return (
    <ul data-testid="signin-staff" className="flex flex-col gap-2">
      {staff.data.map((person) => (
        <li key={person.id}>
          <Button
            type="button"
            variant="outline"
            className="h-auto w-full justify-start gap-3 py-3"
            data-testid={`signin-staff-${person.id}`}
            onClick={() => onPick(person)}
          >
            <Icon as={User} size={18} />
            <span className="truncate">{person.name}</span>
          </Button>
        </li>
      ))}
    </ul>
  );
}

function PinPad() {
  const { t } = useTranslation();
  const { signInWithPin } = useSession();
  const [person, setPerson] = useState<StaffDto | null>(null);
  const [pinDigits, setPinDigits] = useState("");
  const attempt = useLoginAttempt(() => signInWithPin(person?.id ?? 0, pinDigits));

  function backToList() {
    setPerson(null);
    setPinDigits("");
  }

  function onKey(key: KeypadKey) {
    if (attempt.locked || person === null) return;
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
      backToList();
      return;
    }
    setPinDigits((current) => keyedDigits(current, key, MAX_PIN_DIGITS));
  }

  if (person === null) {
    return (
      <div className="flex flex-col gap-3">
        <p className="text-sm text-muted-foreground">{t("signin_pin_id_label")}</p>
        <StaffPicker onPick={setPerson} />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between text-sm text-muted-foreground">
        <span data-testid="signin-picked">{person.name}</span>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-auto p-0 text-muted-foreground"
          onClick={backToList}
        >
          {t("signin_pin_change_id")}
        </Button>
      </div>
      {person.has_pin ? (
        <>
          <span className="text-sm text-muted-foreground">{t("signin_pin_pin_label")}</span>
          {/* Masked: a shoulder behind the counter reads a row of dots and
              not the four digits it is made of. */}
          <div
            data-testid="signin-pin-display"
            dir="ltr"
            className="rounded-md border border-border bg-muted px-3 py-2 text-end font-numeric text-2xl tabular-nums"
          >
            {"•".repeat(pinDigits.length) || " "}
          </div>
          <Keypad onKey={onKey} disabled={attempt.locked} captureWindow />
          <AttemptFeedback attempt={attempt} />
        </>
      ) : (
        <p data-testid="signin-no-pin" className="text-sm text-muted-foreground">
          {t("signin_pin_no_pin")}
        </p>
      )}
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
