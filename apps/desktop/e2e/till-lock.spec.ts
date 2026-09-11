// The brief's own proof: a cart being rung up survives the lock, and the
// lock is real. Locking is `__root.tsx` laying `LockScreen` over the shell
// without unmounting it (`routes/till.tsx` holds the cart in its own
// `useState`, and a route left mounted keeps it) and marking the shell
// `inert`, so a cart survives being covered *and* the covered screen stops
// answering to a click, a Tab, a scan or F9 while it is.
//
// Signed in through the API cookie (`./auth`), not through a sign-in call
// this window itself made, so the session carries no remembered
// `AuthMethod`; `signin.spec.ts` is where the PIN side of a sign-in is
// driven for real, and the third test below drives the lock screen's own
// PIN door the same way.

import { apiHeaders, apiUrl } from "./api";
import { expect, OWNER_PASSWORD, OWNER_PIN, test } from "./auth";
import { t } from "./messages";

const PRODUCT = "Cadenas e2e";
const BARCODE = "6130009000080";
// A second, distinct product: the three tests in this file share one
// database (one Playwright worker, one connection per project), so the
// unreachable-till test cannot reuse the first test's barcode.
const PRODUCT_2 = "Cadenas e2e 2";
const BARCODE_2 = "6130009000081";

async function stockMilli(
  request: import("@playwright/test").APIRequestContext,
  barcode: string,
): Promise<number> {
  const res = await request.get(`${apiUrl()}/products`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const rows: { barcode: string | null; qty_on_hand_milli: number }[] = await res.json();
  const found = rows.find((p) => p.barcode === barcode);
  if (found === undefined) throw new Error(`no product with barcode ${barcode}`);
  return found.qty_on_hand_milli;
}

test("locking the till from the topbar keeps the cart, and unlocking gives it back", async ({
  page,
  request,
}) => {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: PRODUCT,
      barcode: BARCODE,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: 10_000,
      wholesale_centimes: null,
      qty_on_hand_milli: 5_000,
      low_stock_at_milli: 0,
      rate_bps: 1900,
      active: true,
    },
  });
  expect(res.status()).toBe(201);

  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.fill(BARCODE);
  await search.press("Enter");
  const qty = page.getByLabel(`${t("field_qty")} ${PRODUCT}`, { exact: true });
  await expect(qty).toHaveValue("1");

  await page.getByTestId("user-menu-trigger").click();
  await page.getByTestId("user-menu-lock").click();

  await expect(page.getByTestId("lock-screen")).toBeVisible();
  // The shell is covered, not gone: the cart's quantity box is still in the
  // document underneath the overlay, carrying the value it had before the
  // lock came down.
  await expect(qty).toHaveValue("1");

  await page.getByTestId("lock-password").fill(OWNER_PASSWORD);
  await page.getByTestId("lock-unlock").click();

  await expect(page.getByTestId("lock-screen")).toBeHidden();
  await expect(qty).toHaveValue("1");
});

test("the covered till is unreachable: focus cannot land there, and F9 does not pay", async ({
  page,
  request,
}) => {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: PRODUCT_2,
      barcode: BARCODE_2,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: 10_000,
      wholesale_centimes: null,
      qty_on_hand_milli: 5_000,
      low_stock_at_milli: 0,
      rate_bps: 1900,
      active: true,
    },
  });
  expect(res.status()).toBe(201);

  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.fill(BARCODE_2);
  await search.press("Enter");
  const qty = page.getByLabel(`${t("field_qty")} ${PRODUCT_2}`, { exact: true });
  await expect(qty).toHaveValue("1");

  // Cash handed over, so `canPay` genuinely holds: the lock is only proven
  // by a sale F9 would otherwise really have taken. Unscoped: only the
  // till's own payment pad is in the document yet, the lock screen not
  // mounted until the till is actually locked below.
  const pad = page.getByTestId("keypad");
  for (const key of ["5", "0", "0"]) {
    await pad.getByRole("button", { name: key, exact: true }).click();
  }
  await expect(page.getByTestId("till-change")).toBeVisible();

  const stockBefore = await stockMilli(request, BARCODE_2);

  await page.getByTestId("user-menu-trigger").click();
  await page.getByTestId("user-menu-lock").click();
  await expect(page.getByTestId("lock-screen")).toBeVisible();

  // First path: the search box cannot be given focus while the shell that
  // holds it is `inert`, whatever tries — a click, a Tab, or (here) the
  // very call `till.tsx` itself makes after a scan or a completed sale.
  // `.focus()` and the check of `document.activeElement` run in the same
  // `evaluate`, on purpose: two round trips left a window for something
  // else (Radix's own focus return from the just-closed dropdown menu) to
  // land in between and pass either way regardless of `inert`.
  const focusStuck = await search.evaluate((el: HTMLInputElement) => {
    el.focus();
    return document.activeElement === el;
  });
  expect(focusStuck).toBe(false);

  // Same path, the other proof of it: with nothing to focus, a scanner's
  // keystrokes (a barcode, then Enter) land nowhere the till reads them.
  await page.keyboard.type(BARCODE_2);
  await page.keyboard.press("Enter");
  await expect(search).toHaveValue("");
  await expect(qty).toHaveValue("1");

  // Second path: F9 is a `window` listener, above any subtree `inert`
  // reaches, so it needs its own guard. There is nothing to wait on for an
  // absence, so this gives the handler a window in which it would already
  // have fired the request, were the guard missing.
  let saleRequested = false;
  page.on("request", (req) => {
    if (req.url().endsWith("/sales") && req.method() === "POST") saleRequested = true;
  });
  await page.keyboard.press("F9");
  await page.waitForTimeout(1_000);
  expect(saleRequested).toBe(false);
  expect(await stockMilli(request, BARCODE_2)).toBe(stockBefore);
  await expect(page.getByTestId("lock-screen")).toBeVisible();

  // The lock screen itself still works, underneath all of that.
  await page.getByTestId("lock-password").fill(OWNER_PASSWORD);
  await page.getByTestId("lock-unlock").click();
  await expect(page.getByTestId("lock-screen")).toBeHidden();
  await expect(qty).toHaveValue("1");
});

test("a session with no remembered method offers both unlock doors, and the PIN one really unlocks", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  await page.getByTestId("user-menu-trigger").click();
  await page.getByTestId("user-menu-lock").click();
  await expect(page.getByTestId("lock-screen")).toBeVisible();

  // Signed in through the cookie (`./auth`), so `method` is null and both
  // doors are offered: the password form by default, the PIN pad a switch
  // away. Forcing the password form here would strand a cashier who only
  // has a PIN behind "sign in as someone else", which signs out and drops
  // the cart this screen exists to protect.
  await expect(page.getByTestId("lock-password")).toBeVisible();
  await expect(page.getByTestId("lock-mode-switch")).toBeVisible();
  await page.getByTestId("lock-mode-switch").click();
  await expect(page.getByTestId("lock-pin-display")).toBeVisible();

  // Scoped to the lock screen: the till's own payment pad is still in the
  // document underneath (covered, not gone), carrying the same testid.
  const pad = page.getByTestId("lock-screen").getByTestId("keypad");
  for (const key of [...OWNER_PIN]) {
    await pad.getByRole("button", { name: key, exact: true }).click();
  }
  await pad.getByRole("button", { name: t("keypad_enter") }).click();

  await expect(page.getByTestId("lock-screen")).toBeHidden();
});
