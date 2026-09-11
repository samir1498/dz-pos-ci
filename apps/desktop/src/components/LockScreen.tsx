// The overlay `__root.tsx` puts on top of the shell once the idle timer
// has run out. Covers the window, but the shell and the route beneath it
// stay mounted: the till's cart lives in `routes/till.tsx`'s own state, and
// a route left mounted is a route that keeps it. Unlocking is a sign-in
// call like any other (`useSession().signInWithPin` /
// `signInWithPassword`), just one that already knows who it is asking for.

import { useState } from "react";

import { FormField } from "@/components/FormField";
import { Keypad, keyedDigits, type KeypadKey } from "@/components/Keypad";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useTranslation } from "@/i18n";
import { useLoginAttempt } from "@/lib/useLoginAttempt";
import { useSession, type AuthMethod } from "@/lib/session";

const MAX_PIN_DIGITS = 6;

function PinUnlock({ userId }: { userId: number }) {
  const { signInWithPin } = useSession();
  const [digits, setDigits] = useState("");
  const attempt = useLoginAttempt(() => signInWithPin(userId, digits));

  function onKey(key: KeypadKey) {
    if (attempt.locked) return;
    if (key === "enter") {
      if (digits === "") return;
      // Cleared before the call: `attempt.run()` still submits this
      // render's PIN, and a right one unlocks (this component stays
      // mounted, unlike the sign-in screen's) but clearing after would
      // still be a stale closure racing the next render's `attempt`.
      setDigits("");
      void attempt.run();
      return;
    }
    setDigits((current) => keyedDigits(current, key, MAX_PIN_DIGITS));
  }

  return (
    <div className="flex flex-col gap-3">
      <div
        data-testid="lock-pin-display"
        dir="ltr"
        className="rounded-md border border-border bg-muted px-3 py-2 text-end font-numeric text-2xl tabular-nums"
      >
        {"•".repeat(digits.length) || " "}
      </div>
      <Keypad onKey={onKey} disabled={attempt.locked} />
      <Feedback attempt={attempt} />
    </div>
  );
}

function PasswordUnlock({ name }: { name: string }) {
  const { t } = useTranslation();
  const { signInWithPassword } = useSession();
  const [password, setPassword] = useState("");
  const attempt = useLoginAttempt(() => signInWithPassword(name, password));

  return (
    <form
      className="flex flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        if (attempt.locked || password === "") return;
        setPassword("");
        void attempt.run();
      }}
    >
      <FormField label={t("signin_password_password_label")}>
        {(field) => (
          <Input
            {...field}
            data-testid="lock-password"
            type="password"
            value={password}
            disabled={attempt.locked}
            autoComplete="current-password"
            onChange={(event) => setPassword(event.target.value)}
          />
        )}
      </FormField>
      <Button type="submit" data-testid="lock-unlock" disabled={attempt.locked || password === ""}>
        {t("lock_unlock")}
      </Button>
      <Feedback attempt={attempt} />
    </form>
  );
}

function Feedback({ attempt }: { attempt: ReturnType<typeof useLoginAttempt> }) {
  const { t } = useTranslation();
  if (attempt.retryAfter !== null && attempt.retryAfter > 0) {
    return (
      <p data-testid="lock-retry" role="alert" className="text-sm text-fg-danger">
        {t("signin_error_locked_out")} {t("signin_retry_seconds").replace("{n}", String(attempt.retryAfter))}
      </p>
    );
  }
  if (attempt.error !== null) {
    return (
      <p data-testid="lock-error" role="alert" className="text-sm text-fg-danger">
        {t(attempt.error)}
      </p>
    );
  }
  return null;
}

/** Which unlock control to offer: the door the person came in through.
 * `method` is remembered from the sign-in call that succeeded (`lib/
 * session.tsx`), never worked out from the role, so an owner who happened
 * to sign in with a PIN this morning unlocks with the same PIN tonight. A
 * session resumed from the browser's cookie (no sign-in call ran on this
 * window) has no method to remember; the password form is the one that
 * never needs anything this window doesn't already have, `me.name`. */
function unlockMethod(method: AuthMethod | null): AuthMethod {
  return method ?? "password";
}

export function LockScreen() {
  const { t } = useTranslation();
  const { me, method, signOut } = useSession();
  if (me === null) return null;

  return (
    <div
      data-testid="lock-screen"
      className="fixed inset-0 z-50 flex items-center justify-center bg-background p-4"
    >
      <div className="w-full max-w-sm rounded-xl border border-border bg-card p-6 shadow-md">
        <div className="mb-4 text-center">
          <h2 className="text-lg font-semibold">{t("lock_title")}</h2>
          <p className="text-sm text-muted-foreground">{me.name}</p>
        </div>
        {unlockMethod(method) === "pin" ? (
          <PinUnlock userId={me.user_id} />
        ) : (
          <PasswordUnlock name={me.name} />
        )}
        <Button
          type="button"
          variant="link"
          data-testid="lock-switch-user"
          className="mt-3 h-auto p-0 text-sm"
          onClick={() => void signOut()}
        >
          {t("lock_switch_user")}
        </Button>
      </div>
    </div>
  );
}
