// I18nProvider's effect (src/i18n/index.tsx) only runs once React has
// mounted; before that, index.html's own inline script has to have
// already set documentElement's lang and dir, or an Arabic-preferring
// return visitor gets a flash of LTR French on every load. `waitUntil:
// "commit"` stops at the earliest point Playwright will hand back
// control (the navigation is committed, nothing has necessarily
// rendered), and the assertions read the DOM directly rather than
// through an auto-retrying matcher, so a script that only fixes the
// attributes later (once React mounts) does not get papered over by a
// wait.
//
// The fr project cannot fail this test on its own: <html lang="fr"
// dir="ltr"> in index.html is already the right answer for fr with no
// script running at all, so a broken or removed inline script would
// still read correctly there. en (lang mismatches the static "fr") and
// ar (dir mismatches the static "ltr") are the two that actually
// exercise the script; fr stays only as the one case that must not
// regress once it does.

import { expect, test } from "@playwright/test";
import { currentLang } from "./messages";

test("document lang and dir are correct before React mounts", async ({ page }) => {
  await page.goto("/products", { waitUntil: "commit" });
  const lang = currentLang();
  const dir = await page.evaluate(() => document.documentElement.getAttribute("dir"));
  const htmlLang = await page.evaluate(() => document.documentElement.getAttribute("lang"));
  expect(dir).toBe(lang === "ar" ? "rtl" : "ltr");
  expect(htmlLang).toBe(lang);
});
