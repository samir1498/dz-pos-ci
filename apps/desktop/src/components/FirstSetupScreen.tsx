// The first screen a brand-new shop sees. Name and password, you are the
// owner. The till PIN is set later from the users screen. Posts to
// `/auth/first-setup`; the door shuts for good after that (features.md §5).

import { useState, type FormEvent } from "react";

import { FormField } from "@/components/FormField";
import { PasswordInput } from "@/components/PasswordInput";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Wordmark } from "@/components/Wordmark";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";
import { useSession } from "@/lib/session";

export function FirstSetupScreen() {
  const { t } = useTranslation();
  const { claimFirstOwner } = useSession();
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<Key | null>(null);
  const [pending, setPending] = useState(false);

  const nameError: Key | null = name.trim() === "" ? "validation_required" : null;
  const passwordError: Key | null =
    password === "" ? null : password.length < 8 ? "validation_password_length" : null;
  const confirmError: Key | null =
    confirm === "" ? null : confirm !== password ? "setup_mismatch" : null;

  function onSubmit(event: FormEvent) {
    event.preventDefault();
    if (pending) return;
    if (name.trim() === "" || password.length < 8 || confirm !== password) {
      if (name.trim() === "") setError("validation_required");
      else if (password.length < 8) setError("validation_password_length");
      else setError("setup_mismatch");
      return;
    }
    setPending(true);
    setError(null);
    void claimFirstOwner(name.trim(), password)
      .catch((cause: unknown) => {
        setError(errorKey(cause, {}));
      })
      .finally(() => setPending(false));
  }

  return (
    <div
      data-testid="setup-screen"
      className="flex min-h-screen items-center justify-center bg-background p-4"
    >
      <Card className="w-full max-w-sm">
        <CardHeader className="items-center text-center">
          <Wordmark className="mb-2" />
          <CardTitle>{t("setup_title")}</CardTitle>
          <CardDescription>{t("setup_subtitle")}</CardDescription>
        </CardHeader>
        <CardContent>
          <form className="flex flex-col gap-3" onSubmit={onSubmit}>
            <FormField
              label={t("setup_name_label")}
              error={nameError ? t(nameError) : undefined}
            >
              {(field) => (
                <Input
                  {...field}
                  data-testid="setup-name"
                  value={name}
                  autoComplete="username"
                  autoFocus
                  disabled={pending}
                  aria-invalid={nameError !== null}
                  onChange={(event) => {
                    setName(event.target.value);
                    setError(null);
                  }}
                />
              )}
            </FormField>
            <FormField
              label={t("setup_password_label")}
              hint={t("validation_password_hint")}
              error={passwordError ? t(passwordError) : undefined}
            >
              {(field) => (
                <PasswordInput
                  {...field}
                  data-testid="setup-password"
                  value={password}
                  autoComplete="new-password"
                  disabled={pending}
                  aria-invalid={passwordError !== null}
                  onChange={(event) => {
                    setPassword(event.target.value);
                    setError(null);
                  }}
                />
              )}
            </FormField>
            <FormField
              label={t("setup_confirm_label")}
              error={confirmError ? t(confirmError) : undefined}
            >
              {(field) => (
                <PasswordInput
                  {...field}
                  data-testid="setup-confirm"
                  value={confirm}
                  autoComplete="new-password"
                  disabled={pending}
                  aria-invalid={confirmError !== null}
                  onChange={(event) => {
                    setConfirm(event.target.value);
                    setError(null);
                  }}
                />
              )}
            </FormField>
            <Button type="submit" data-testid="setup-submit" disabled={pending}>
              {t("setup_submit")}
            </Button>
            {error !== null ? (
              <p data-testid="setup-error" role="alert" className="text-sm text-fg-danger">
                {t(error)}
              </p>
            ) : null}
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
