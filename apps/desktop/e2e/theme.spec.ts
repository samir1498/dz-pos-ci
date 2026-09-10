// The theme switch through a real browser against a real API. What a unit
// test cannot show and this can: that the choice reaches the shop file and
// comes back after a reload, and that the page actually repaints. The second
// half matters more than it looks. `data-theme` on `<html>` is only a
// promise; the shadcn names are declared once in `:root` pointing at our
// roles, so if the attribute ever landed on a wrapper instead, the attribute
// assertion would still pass and every colour would stay on Comptoir. So the
// computed background of the page is read as well.
//
// This file runs before the till specs (one worker, files in name order, one
// database), so it hands the shop back to "follow the machine" before it
// leaves, the way the settings spec hands the régime back.

import { expect, test } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { currentLang, t } from "./messages";

const here = fileURLToPath(new URL(".", import.meta.url));

/** The four names the design package emits a `[data-theme]` block for. */
const THEMES = ["comptoir", "registre", "observe", "observe-dark"] as const;

/** The label the switcher shows for each, from the running project's JSON. */
const LABEL: Record<(typeof THEMES)[number], string> = {
  comptoir: "theme_comptoir",
  registre: "theme_registre",
  observe: "theme_observe",
  "observe-dark": "theme_observe_dark",
};

const themeOf = (page: import("@playwright/test").Page) =>
  page.locator("html").getAttribute("data-theme");

/**
 * The document element, not the body: the generated base rule paints `html`,
 * so `body` is transparent and reading it would answer `rgba(0, 0, 0, 0)` on
 * every theme, which is a green assertion that proves nothing.
 */
const pageBackground = (page: import("@playwright/test").Page) =>
  page.evaluate(() => window.getComputedStyle(document.documentElement).backgroundColor);

test("each theme is kept in the shop file and survives a reload", async ({ page }) => {
  const puts: unknown[] = [];
  await page.route("**/settings/theme", async (route) => {
    if (route.request().method() === "PUT") puts.push(route.request().postDataJSON());
    await route.continue();
  });

  await page.goto("/products");
  const switcher = page.getByTestId("theme-switcher").first();
  await expect(switcher).toBeVisible();

  // Nothing chosen yet, and headless chromium reports a light machine, so
  // the app opens on the light theme.
  await expect.poll(() => themeOf(page)).toBe("comptoir");
  const light = await pageBackground(page);

  const seen = new Set<string>([light]);
  for (const name of THEMES) {
    await switcher.selectOption(name);
    await expect.poll(() => themeOf(page)).toBe(name);

    // The reload is the point: the value has to come back from the API, not
    // from React state that a refresh throws away.
    await page.reload();
    await expect.poll(() => themeOf(page)).toBe(name);
    await expect(page.getByTestId("theme-switcher").first()).toHaveValue(name);

    const painted = await pageBackground(page);
    expect(painted, `${name} did not repaint the page`).not.toBe("rgba(0, 0, 0, 0)");
    seen.add(painted);
  }

  expect(puts).toEqual(THEMES.map((theme) => ({ theme })));
  // Four themes, four grounds. Two blocks resolving to the same colour would
  // mean a role fell through to Comptoir's value.
  expect(seen.size, "two themes painted the same background").toBe(THEMES.length);
});

test("the settings screen offers the same switch, and saves the Arabic screenshots", async ({
  page,
}) => {
  await page.goto("/settings");
  await expect(page.getByRole("heading", { name: t("settings_title") })).toBeVisible();
  const panel = page.getByRole("region", { name: t("settings_theme") });
  await expect(panel).toBeVisible();
  const switcher = panel.getByTestId("theme-switcher");

  // The label is what a person reads, so the option is picked by its
  // translated text rather than by its value.
  await switcher.selectOption({ label: t(LABEL.comptoir) });
  await expect.poll(() => themeOf(page)).toBe("comptoir");
  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "theme-comptoir-ar.png"),
      fullPage: true,
    });
  }

  await switcher.selectOption({ label: t(LABEL.observe) });
  await expect.poll(() => themeOf(page)).toBe("observe");
  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "theme-observe-ar.png"),
      fullPage: true,
    });
  }

  // Back to following the machine, so the specs after this one photograph
  // the shop in the theme they have always been photographed in.
  await switcher.selectOption({ label: t("theme_system") });
  await expect.poll(() => themeOf(page)).toBe("comptoir");
  await page.reload();
  await expect(page.getByTestId("theme-switcher").first()).toHaveValue("system");
});
