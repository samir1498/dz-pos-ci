// T8 (context/plans/20260924-shop-manual-test-findings.md): a password
// field with our own show/hide eye, since Edge and Chrome/Brave/Firefox
// disagreed about drawing one at all. Proves the toggle actually flips the
// input's `type` and that the field is still the same element a caller's
// own test finds by `data-testid` (FirstSetupScreen.test.tsx and friends
// type into it directly).

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, test } from "vitest";

import { I18nProvider } from "@/i18n";
import { PasswordInput } from "../../src/components/PasswordInput";

function mount(disabled = false) {
  render(
    <I18nProvider lang="fr">
      <PasswordInput
        data-testid="pw"
        value="secret"
        disabled={disabled}
        onChange={() => {}}
      />
    </I18nProvider>,
  );
}

test("opens masked, with the field still reachable by its own testid", () => {
  mount();
  expect(screen.getByTestId("pw")).toHaveAttribute("type", "password");
});

test("the eye button reveals the text and switches back", async () => {
  const user = userEvent.setup();
  mount();
  const toggle = screen.getByTestId("pw-toggle");
  expect(toggle).toHaveAttribute("aria-pressed", "false");

  await user.click(toggle);
  expect(screen.getByTestId("pw")).toHaveAttribute("type", "text");
  expect(toggle).toHaveAttribute("aria-pressed", "true");

  await user.click(toggle);
  expect(screen.getByTestId("pw")).toHaveAttribute("type", "password");
});

test("a disabled field disables its own toggle too", () => {
  mount(true);
  expect(screen.getByTestId("pw-toggle")).toBeDisabled();
});
