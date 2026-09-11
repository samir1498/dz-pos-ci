// M5 T0 (docs/architecture.md § Release): the launch token used to sit in
// `globalThis.__DZPOS_API_TOKEN__`, set by the Tauri process before any app
// script ran. A browser preview never used that global -- it reads
// VITE_API_TOKEN instead (apps/desktop/src/api.ts, apiToken()) -- so this
// suite could never see the Tauri-only path go wrong by itself. What it can
// prove, in any of the three languages, is the other half of the fix: that
// a value sitting in that global is no longer consulted at all, by anyone.
//
// Against the code before this change, `apiToken()` checked the global
// first and only fell back to VITE_API_TOKEN when it was absent. Planting a
// wrong value there made every request carry the wrong bearer, including
// the `auth.ts` fixture's own sign-in call: the launch token is the outer
// gate (crates/api's `token.rs::require`, checked before the session gate
// underneath it), so a wrong one 401s sign-in itself, not just the products
// fetch. Measured against the old `apiToken()`: the products screen is
// never reached at all, so the heading assertion below is what times out
// and goes red, not the alert check after it.

import { expect, test } from "./auth";
import { t } from "./messages";

test("a token planted in a page global is not honoured", async ({ page }) => {
  await page.addInitScript(() => {
    // Reflect.set rather than a direct assignment with a cast: the repo
    // allows no `as` type assertions, and this only needs to put a
    // property on the object, not read one back with a type.
    Reflect.set(globalThis, "__DZPOS_API_TOKEN__", "not-the-real-token");
  });

  await page.goto("/products");

  await expect(page.getByRole("main").getByRole("heading", { name: t("products_title") })).toBeVisible();
  // The real launch token (DZPOS_E2E_TOKEN, carried by VITE_API_TOKEN) is
  // what authenticated this fetch; the planted global sat there unread.
  await expect(page.getByRole("alert")).toHaveCount(0);
});
