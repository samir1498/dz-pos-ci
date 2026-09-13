// The first screen a brand-new shop sees. Nobody has a PIN yet, so a
// sign-in pad asking for a user id would be a door with no key. This one
// takes a PIN twice, posts it to `/auth/first-pin`, and the owner is signed
// in. The door shuts for good after that (features.md §5).

import { useState } from "react";

import { Keypad, keyedDigits, type KeypadKey } from "@/components/Keypad";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Wordmark } from "@/components/Wordmark";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";
import { useSession } from "@/lib/session";

const MIN_PIN_DIGITS = 4;
const MAX_PIN_DIGITS = 6;

export function FirstPinScreen() {
  const { t } = useTranslation();
  const { claimFirstPin } = useSession();
  const [stage, setStage] = useState<"pin" | "confirm">("pin");
  const [pin, setPin] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<Key | null>(null);
  const [pending, setPending] = useState(false);

  const digits = stage === "pin" ? pin : confirm;

  function onKey(key: KeypadKey) {
    if (pending) return;
    if (key === "enter") {
      if (stage === "pin") {
        if (pin.length < MIN_PIN_DIGITS) return;
        setStage("confirm");
        setError(null);
        return;
      }
      if (confirm.length < MIN_PIN_DIGITS) return;
      if (confirm !== pin) {
        setConfirm("");
        setError("firstpin_mismatch");
        return;
      }
      setPending(true);
      setError(null);
      void claimFirstPin(pin)
        .catch((cause: unknown) => {
          setConfirm("");
          setError(errorKey(cause, {}));
        })
        .finally(() => setPending(false));
      return;
    }
    setError(null);
    if (stage === "pin") {
      setPin((current) => keyedDigits(current, key, MAX_PIN_DIGITS));
      return;
    }
    setConfirm((current) => keyedDigits(current, key, MAX_PIN_DIGITS));
  }

  return (
    <div
      data-testid="firstpin-screen"
      className="flex min-h-screen items-center justify-center bg-background p-4"
    >
      <Card className="w-full max-w-sm">
        <CardHeader className="items-center text-center">
          <Wordmark className="mb-2" />
          <CardTitle>{t("firstpin_title")}</CardTitle>
          <CardDescription>{t("firstpin_subtitle")}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <div className="text-sm text-muted-foreground">
            {stage === "pin" ? t("firstpin_pin_label") : t("firstpin_confirm_label")}
          </div>
          <div
            data-testid="firstpin-display"
            dir="ltr"
            className="rounded-md border border-border bg-muted px-3 py-2 text-end font-numeric text-2xl tabular-nums"
          >
            {"•".repeat(digits.length) || " "}
          </div>
          <Keypad onKey={onKey} disabled={pending} captureWindow />
          {error !== null ? (
            <p data-testid="firstpin-error" role="alert" className="text-sm text-fg-danger">
              {t(error)}
            </p>
          ) : null}
        </CardContent>
      </Card>
    </div>
  );
}
