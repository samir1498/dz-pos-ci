// The shell and the kit in a real browser, which is where the three things
// jsdom cannot answer live: whether the sidebar is actually on the reading
// side of an Arabic screen, whether the overlays open at all (Radix wants
// pointer capture and `scrollIntoView`, and jsdom has neither), and what the
// four themes look like.
//
// It also writes the four kit screenshots. They run in the fr project only,
// the way products.png does: three projects would write the same four files
// three times over and the last one to finish would decide what is committed.
//
// This file runs before the settings and theme specs (one worker, files in
// name order, one database), so it hands the shop back to Comptoir, the
// default, before it leaves.

import { expect, test } from "./auth";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { currentLang, t } from "./messages";

const here = fileURLToPath(new URL(".", import.meta.url));

/**
 * The four themes, in the order the switcher offers them. Hand-listed for the
 * same reason `theme.spec.ts` hand-lists them: a spec reads the running app,
 * not the package, and `src/theme.test.ts` is what chains this list to the
 * design package through index.html's own copy.
 */
const THEMES = ["comptoir", "registre", "observe", "observe-dark"] as const;

const themeOf = (page: import("@playwright/test").Page) =>
  page.locator("html").getAttribute("data-theme");

/** Every screen in the sidebar, and what each is called in this project. */
const NAV: readonly { readonly testId: string; readonly key: string; readonly path: string }[] = [
  { testId: "nav-till", key: "nav_till", path: "/till" },
  { testId: "nav-customers", key: "nav_customers", path: "/customers" },
  { testId: "nav-documents", key: "nav_documents", path: "/documents" },
  { testId: "nav-suppliers", key: "nav_suppliers", path: "/suppliers" },
  { testId: "nav-purchases", key: "nav_purchases", path: "/purchases" },
  { testId: "nav-expenses", key: "nav_expenses", path: "/expenses" },
  { testId: "nav-products", key: "nav_products", path: "/products" },
  { testId: "nav-settings", key: "nav_settings", path: "/settings" },
];

test("the sidebar reaches every screen and the topbar names it", async ({ page }) => {
  await page.goto("/till");
  await expect(page.getByTestId("shell-topbar")).toBeVisible();

  for (const item of NAV) {
    await page.getByTestId(item.testId).click();
    await expect(page).toHaveURL(new RegExp(`${item.path}$`));
    // The heading and the sidebar item say the same thing, in whichever
    // language this project runs: one list in AppShell feeds both.
    await expect(page.getByTestId("shell-title")).toHaveText(t(item.key));
    await expect(page.getByTestId(item.testId)).toHaveAttribute("data-active", "true");
  }
});

/**
 * The panel's edge, its border and the half it slides in from all follow one
 * `side`, which the shell computes from the page direction. Read as a
 * geometry rather than as a class name, because a logical class that the
 * stylesheet never resolved would still be in the attribute.
 */
test("the sidebar sits on the reading side", async ({ page }) => {
  await page.goto("/till");
  const sidebar = page.locator('[data-slot="sidebar-container"]');
  await expect(sidebar).toBeVisible();
  const box = await sidebar.boundingBox();
  const width = page.viewportSize()?.width ?? 0;
  expect(box).not.toBeNull();
  const middle = (box?.x ?? 0) + (box?.width ?? 0) / 2;
  if (currentLang() === "ar") {
    expect(middle, "the sidebar should be on the right in Arabic").toBeGreaterThan(width / 2);
  } else {
    expect(middle, "the sidebar should be on the left in French and English").toBeLessThan(width / 2);
  }
});

test("a narrow window folds the sidebar into a sheet", async ({ page }) => {
  await page.goto("/till");
  await expect(page.locator('[data-slot="sidebar-container"]')).toBeVisible();

  await page.setViewportSize({ width: 700, height: 800 });
  // The permanent panel is gone; the shop opens it from the topbar instead.
  await expect(page.locator('[data-slot="sidebar-container"]')).toBeHidden();

  await page.getByTestId("sidebar-trigger").click();
  const sheet = page.getByRole("dialog");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByTestId("nav-products")).toBeVisible();

  // The sheet opens on the same side the permanent panel was on, which in
  // Arabic is the right. It is the sidebar's `side` that decides, and the
  // shell reads that off the page direction.
  const box = await sheet.boundingBox();
  expect(box).not.toBeNull();
  const middle = (box?.x ?? 0) + (box?.width ?? 0) / 2;
  if (currentLang() === "ar") {
    expect(middle, "the sheet should open on the right in Arabic").toBeGreaterThan(350);
  } else {
    expect(middle, "the sheet should open on the left").toBeLessThan(350);
  }

  await page.keyboard.press("Escape");
  await expect(sheet).toBeHidden();
});

test("the topbar's theme switch repaints the page", async ({ page }) => {
  await page.goto("/till");
  const switcher = page.getByTestId("theme-switcher").first();
  await expect(switcher).toBeVisible();
  await expect.poll(() => themeOf(page)).toBe("comptoir");

  await switcher.selectOption("registre");
  await expect.poll(() => themeOf(page)).toBe("registre");

  await switcher.selectOption("comptoir");
  await expect.poll(() => themeOf(page)).toBe("comptoir");
});

test("the kit page shows every component, and every overlay opens", async ({ page }) => {
  await page.goto("/kit");
  await expect(page.getByTestId("kit-page")).toBeVisible();

  // The five pills, each with the word this project's dictionary carries.
  for (const key of ["pill_issued", "pill_cancelled", "pill_paid", "pill_open", "pill_low"]) {
    await expect(page.getByText(t(key), { exact: true }).first()).toBeVisible();
  }

  await page.getByTestId("kit-dialog-trigger").click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toBeHidden();

  await page.getByTestId("kit-sheet-trigger").click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toBeHidden();

  await page.getByTestId("kit-menu-trigger").click();
  await expect(page.getByRole("menu")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toBeHidden();

  await page.getByTestId("kit-toast-trigger").click();
  await expect(page.getByText("Facture enregistrée.")).toBeVisible();
});

/**
 * A money column is the reason `DataTable` exists: end-aligned in the figure
 * face, so a column of totals lines up on the digit. Read off the rendered
 * styles rather than the class list, because a utility that Tailwind never
 * generated would still be in the class list.
 */
test("the kit's money column is set in the figure face", async ({ page }) => {
  await page.goto("/kit");
  const cell = page.getByTestId("kit-table").getByRole("cell").nth(3);
  const styles = await cell.evaluate((node) => {
    const computed = window.getComputedStyle(node);
    return { align: computed.textAlign, family: computed.fontFamily };
  });
  expect(styles.family).toContain("JetBrains Mono");
  // `end`, not `right`: the browser keeps the logical value, which is the
  // whole point of writing it logically. So where it actually lands is asked
  // of the geometry below rather than of the property.
  expect(styles.align).toBe("end");

  const cellBox = await cell.boundingBox();
  const amountBox = await cell.locator("span").first().boundingBox();
  expect(cellBox).not.toBeNull();
  expect(amountBox).not.toBeNull();
  const gapBefore = (amountBox?.x ?? 0) - (cellBox?.x ?? 0);
  const gapAfter =
    (cellBox?.x ?? 0) + (cellBox?.width ?? 0) - ((amountBox?.x ?? 0) + (amountBox?.width ?? 0));
  if (currentLang() === "ar") {
    expect(gapAfter, "the amount should hug the start edge in Arabic").toBeGreaterThan(gapBefore);
  } else {
    expect(gapBefore, "the amount should hug the end edge").toBeGreaterThan(gapAfter);
  }
});

test("screenshot: the kit in each theme", async ({ page }) => {
  // fr only. Three projects would write the same four files three times and
  // whichever finished last would decide what is committed.
  test.skip(currentLang() !== "fr", "one language writes the committed shots");

  await page.goto("/kit");
  await expect(page.getByTestId("kit-page")).toBeVisible();
  const switcher = page.getByTestId("theme-switcher").first();

  for (const theme of THEMES) {
    await switcher.selectOption(theme);
    await expect.poll(() => themeOf(page)).toBe(theme);
    await page.screenshot({
      path: path.join(here, "screenshots", `kit-${theme}.png`),
      fullPage: true,
    });
  }

  // Back to the default, so the specs after this one photograph the shop in
  // the theme they have always been photographed in.
  await switcher.selectOption("comptoir");
  await expect.poll(() => themeOf(page)).toBe("comptoir");
});
