// Filling the two segmented date controls from a spec.
//
// `DateField` and `MonthField` replaced `<input type="date">` and
// `<input type="month">`, because a native picker reads in the machine's
// language and the machine's day order and an Algerian shop on a French
// Windows could not tell September from November. What each renders now is a
// `role="group"` holding two or three number boxes, and Playwright's `fill`
// refuses a group: the specs that still typed a whole ISO string into one
// had been failing on that ever since, on `/expenses` as well as on the
// régime form.
//
// Segment order is day, month, year and it is load-bearing: the day is
// clamped against whichever month and year are set beside it
// (`lib/date-segments.ts`), so a 31 typed before its month would come back
// as the last day of whatever month was there before.

import type { Locator, Page } from "@playwright/test";

/** `YYYY-MM-DD` into a `DateField`, by its `data-testid`. */
export async function fillDate(scope: Page | Locator, testId: string, iso: string): Promise<void> {
  const [year, month, day] = iso.split("-");
  await scope.getByTestId(`${testId}-day`).fill(day ?? "");
  await scope.getByTestId(`${testId}-month`).fill(month ?? "");
  await scope.getByTestId(`${testId}-year`).fill(year ?? "");
}

/** `YYYY-MM` into a `MonthField`, by its `data-testid`. */
export async function fillMonth(
  scope: Page | Locator,
  testId: string,
  isoMonth: string,
): Promise<void> {
  const [year, month] = isoMonth.split("-");
  await scope.getByTestId(`${testId}-month`).fill(month ?? "");
  await scope.getByTestId(`${testId}-year`).fill(year ?? "");
}
