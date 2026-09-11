// The brief's own proof: a cart being rung up survives the lock. Locking
// is `__root.tsx` laying `LockScreen` over the shell without unmounting it
// (`routes/till.tsx` holds the cart in its own `useState`, and a route left
// mounted keeps it), so this seeds one product, scans it in, locks the
// till from the topbar, unlocks, and reads the same quantity box back.
//
// Signed in through the API cookie (`./auth`), not through a sign-in call
// this window itself made, so the session carries no remembered
// `AuthMethod` and the lock screen falls back to the password form
// (`LockScreen.tsx::unlockMethod`) the way any browser tab that resumed
// from its cookie would; `signin.spec.ts` is where the PIN side of a
// sign-in is driven for real.

import { apiHeaders, apiUrl } from "./api";
import { expect, OWNER_PASSWORD, test } from "./auth";
import { t } from "./messages";

const PRODUCT = "Cadenas e2e";
const BARCODE = "6130009000080";

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
