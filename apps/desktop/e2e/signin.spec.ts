// The two doors themselves, driven through the real keys and the real
// fields: `e2e/auth.ts` signs every other spec in through the API so they
// can get on with what they are actually testing, and this file is what
// proves that door itself works. Plain `@playwright/test`, not `./auth`:
// every test here starts signed out on purpose.

import { expect, test } from "@playwright/test";

import { apiUrl } from "./api";
import { OWNER_NAME, OWNER_PASSWORD, OWNER_PIN } from "./auth";
import { t } from "./messages";

const OWNER_ID = "1";

async function pressPad(page: import("@playwright/test").Page, keys: readonly string[]): Promise<void> {
  const pad = page.getByTestId("keypad");
  for (const key of keys) {
    await pad.getByRole("button", { name: key, exact: true }).click();
  }
}

async function pressEnter(page: import("@playwright/test").Page): Promise<void> {
  await page.getByTestId("keypad").getByRole("button", { name: t("keypad_enter") }).click();
}

test.describe("the PIN pad", () => {
  test("refuses a wrong PIN and signs the owner in on the right one", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByTestId("signin-screen")).toBeVisible();
    await expect(page.getByTestId("signin-pin-display")).toBeVisible();

    // The id stage: the pad has no picker to choose the owner off
    // (open item, see the report), so the id is typed the way the PIN is.
    await pressPad(page, [...OWNER_ID]);
    await pressEnter(page);
    await expect(page.getByText(t("signin_pin_pin_label"), { exact: true })).toBeVisible();

    // A PIN that is not the owner's.
    await pressPad(page, ["0", "0", "0", "0"]);
    await pressEnter(page);
    await expect(page.getByTestId("signin-error")).toHaveText(t("signin_error_auth_refused"));

    // The right one, same id stage (a wrong PIN does not send the pad back
    // to the id stage).
    await pressPad(page, [...OWNER_PIN]);
    await pressEnter(page);

    await expect(page.getByTestId("shell-topbar")).toBeVisible();
    await expect(page.getByTestId("user-menu-trigger")).toContainText(OWNER_NAME);
  });

  test("backspace on an empty PIN returns to the id stage", async ({ page }) => {
    await page.goto("/");
    await pressPad(page, [...OWNER_ID]);
    await pressEnter(page);
    await expect(page.getByText(t("signin_pin_pin_label"), { exact: true })).toBeVisible();

    await page.getByTestId("keypad").getByRole("button", { name: t("keypad_backspace") }).click();
    await expect(page.getByText(t("signin_pin_id_label"), { exact: true })).toBeVisible();
  });
});

test.describe("the password screen", () => {
  test("refuses a wrong password and signs the owner in on the right one", async ({ page }) => {
    await page.goto("/");
    await page.getByTestId("signin-mode-switch").click();
    await expect(page.getByTestId("signin-name")).toBeVisible();

    await page.getByTestId("signin-name").fill(OWNER_NAME);
    await page.getByTestId("signin-password").fill("not-the-password");
    await page.getByTestId("signin-submit").click();
    await expect(page.getByTestId("signin-error")).toHaveText(t("signin_error_auth_refused"));

    await page.getByTestId("signin-password").fill(OWNER_PASSWORD);
    await page.getByTestId("signin-submit").click();

    await expect(page.getByTestId("shell-topbar")).toBeVisible();
    await expect(page.getByTestId("user-menu-trigger")).toContainText(OWNER_NAME);
  });
});

test.describe("the locked-out wait", () => {
  // Not the real owner: three wrong PINs against the seeded row would lock
  // it out for real and poison every spec that signs in after this one in
  // the same database. `POST /auth/login` is intercepted instead, so the
  // screen is proven against the one thing the brief asks for, a wait
  // counted down from the figure the server sent, without the server ever
  // being asked to enforce it here.
  test("counts the wait down from the server's own figure, never its own", async ({ page }) => {
    await page.route(`${apiUrl()}/auth/login`, async (route) => {
      await route.fulfill({
        status: 429,
        contentType: "application/json",
        body: JSON.stringify({
          error: { code: "locked_out", message: "too many tries", retry_after_seconds: 3 },
        }),
      });
    });

    await page.goto("/");
    await pressPad(page, [...OWNER_ID]);
    await pressEnter(page);
    await pressPad(page, [...OWNER_PIN]);
    await pressEnter(page);

    await expect(page.getByTestId("signin-retry")).toContainText("3");
    await expect(page.getByTestId("keypad").getByRole("button", { name: "1" })).toBeDisabled();
    // The clock ticks down on its own and re-enables the pad at zero,
    // without a second answer from the server.
    await expect(page.getByTestId("signin-retry")).toContainText("1", { timeout: 5_000 });
    await expect(page.getByTestId("keypad").getByRole("button", { name: "1" })).toBeEnabled({
      timeout: 5_000,
    });
  });
});
