// Scene 4: the day. What came in, what is owed, what is short, and the
// month drawn by week then by day. Nothing is seeded: the figures are
// whatever the scenes before this one rang up.

import { t } from "../messages";
import { beat, expect, keepClip, test } from "./scene";

test("the day", async ({ page }, testInfo) => {
  await page.goto("/dashboard");
  await expect(
    page.getByRole("main").getByRole("heading", { name: t("dashboard_title") }),
  ).toBeVisible();
  await expect(page.getByTestId("dashboard-chart")).toBeVisible();
  await beat(page, 4);

  await page.getByTestId("chart-grain-weeks").click();
  await beat(page, 3);
  await page.getByTestId("chart-grain-days").click();
  await beat(page, 2);

  // Down to the lists: what is running low, what sells.
  await page.getByTestId("dashboard-low-stock").scrollIntoViewIfNeeded();
  await beat(page, 3);

  await keepClip(page, testInfo);
});
