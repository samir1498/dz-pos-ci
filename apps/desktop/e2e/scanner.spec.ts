// Every barcode scanner behaviour we could get at without a scanner.
//
// Anouar asked for the models to be tested and nobody knows which models
// they are. It turns out not to matter much: a wedge scanner is a keyboard,
// and what separates one model from the next is what it sends after the
// code, whether the right box had focus, and whether its keyboard layout
// matches the machine's. Each of those is a case below, played through the
// real browser against the real app.
//
// The clock is Playwright's, not the wall's. `page.clock.install()` freezes
// `performance.now()`, which is what `useScanner` measures a burst with, so
// a burst here is a burst on a loaded box as much as an idle one, and the
// pause that ends a burst is `clock.runFor`, exact to the millisecond
// rather than a sleep that hopes.
//
// The one thing this file cannot prove is the one Samir tests by hand: that
// Tauri's webview reports `code` the way Chromium does. See the plan's
// matrix, last two rows.

import { expect, test } from "./auth";
import type { APIRequestContext, Page } from "@playwright/test";
import { apiHeaders, apiUrl } from "./api";
import { t } from "./messages";

/** The article a scan is supposed to find, as the shop filed it. */
const ARTICLE = "Biscuit scan e2e";
const CODE = "6130009000110";

/** Filed as a thirteen-digit EAN; the box carries the twelve-digit UPC,
 * which is the same number without its leading zero. */
const UPC_ARTICLE = "Thé scan e2e";
const UPC_ON_FILE = "0613000900127";
const UPC_ON_THE_BOX = "613000900127";

/** No article carries this one. */
const UNKNOWN = "6130009999999";

const PRICE = 12_000;
const STOCK_MILLI = 20_000;

/** The digits' physical keys, and what a French keyboard makes of each of
 * them unshifted. A scanner configured for a US keyboard presses the first
 * and the page is handed the second. */
const AZERTY_UNSHIFTED = ["à", "&", "é", '"', "'", "(", "-", "è", "_", "ç"];

async function seed(request: APIRequestContext, name: string, barcode: string): Promise<void> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name,
      barcode,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: PRICE,
      wholesale_centimes: null,
      qty_on_hand_milli: STOCK_MILLI,
      low_stock_at_milli: 0,
      rate_bps: 1900,
      active: true,
    },
  });
  // 409 is this article already on file: the language projects share one
  // database and a retried test seeds again.
  expect([201, 409]).toContain(res.status());
}

function searchBox(page: Page) {
  return page.getByLabel(t("till_search"), { exact: true });
}

function cartRow(page: Page, name: string) {
  return page.getByRole("row").filter({ hasText: name });
}

function qtyBox(page: Page, name: string) {
  return page.getByLabel(`${t("field_qty")} ${name}`, { exact: true });
}

/**
 * A wedge scanner typing a code.
 *
 * Every character goes through CDP rather than `keyboard.type`, because the
 * whole point of one of these cases is a `key` that disagrees with its
 * `code`, and that pairing is exactly what the high-level API decides for
 * you. `layout: "us"` is a scanner set up for a US keyboard on the French
 * machine the shop owns: the physical digit keys, read through AZERTY.
 */
async function scan(
  page: Page,
  code: string,
  options: { layout?: "fr" | "us"; terminator?: "Enter" | "Tab" | null; prefix?: string } = {},
): Promise<void> {
  const { layout = "fr", terminator = "Enter", prefix } = options;
  const cdp = await page.context().newCDPSession(page);
  const press = async (key: string, keyCode: string) => {
    await cdp.send("Input.dispatchKeyEvent", { type: "keyDown", key, code: keyCode, text: key });
    await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", key, code: keyCode });
  };
  if (prefix !== undefined) await press(prefix, "Digit8");
  for (const digit of code) {
    const index = Number(digit);
    const physical = `Digit${digit}`;
    await press(layout === "us" ? AZERTY_UNSHIFTED[index] : digit, physical);
  }
  if (terminator !== null) {
    await cdp.send("Input.dispatchKeyEvent", {
      type: "rawKeyDown",
      key: terminator,
      code: terminator,
      windowsVirtualKeyCode: terminator === "Enter" ? 13 : 9,
    });
    await cdp.send("Input.dispatchKeyEvent", {
      type: "keyUp",
      key: terminator,
      code: terminator,
      windowsVirtualKeyCode: terminator === "Enter" ? 13 : 9,
    });
  }
  await cdp.detach();
}

test.beforeEach(async ({ request, page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);
  await expect(searchBox(page)).toBeFocused();
  // Frozen from here: every gap this file measures is one it chose.
  await page.clock.install();
});

test.describe("what the model sends after the code", () => {
  test.beforeAll(async ({ request }) => {
    await seed(request, ARTICLE, CODE);
  });

  test("Enter, and the article lands once", async ({ page }) => {
    await scan(page, CODE);
    await expect(cartRow(page, ARTICLE)).toBeVisible();
    // One scan is one add, whichever handler ends up reading the
    // terminator. The screen has two that could.
    await expect(qtyBox(page, ARTICLE)).toHaveValue("1");
    await expect(searchBox(page)).toHaveValue("");
  });

  test("Tab, which the other half of them send", async ({ page }) => {
    await scan(page, CODE, { terminator: "Tab" });
    await expect(cartRow(page, ARTICLE)).toBeVisible();
  });

  test("nothing at all: the pause is the terminator", async ({ page }) => {
    await scan(page, CODE, { terminator: null });
    await expect(cartRow(page, ARTICLE)).toBeHidden();
    await page.clock.runFor(150);
    await expect(cartRow(page, ARTICLE)).toBeVisible();
  });

  test("a prefix character of the scanner's own is not part of the code", async ({ page }) => {
    await scan(page, CODE, { prefix: "*" });
    await expect(cartRow(page, ARTICLE)).toBeVisible();
  });
});

test.describe("a scanner set up for a US keyboard, on the French machine", () => {
  test.beforeAll(async ({ request }) => {
    await seed(request, `${ARTICLE} US`, "6130009000141");
  });

  test("the digits arrive as punctuation and the article is still found", async ({ page }) => {
    await scan(page, "6130009000141", { layout: "us" });
    await expect(cartRow(page, `${ARTICLE} US`)).toBeVisible();
  });
});

test.describe("where the cashier left the focus", () => {
  test.beforeAll(async ({ request }) => {
    await seed(request, `${ARTICLE} focus`, "6130009000158");
  });

  test("on a button, and the scan still goes to the box", async ({ page }) => {
    await page.getByRole("button", { name: t("till_all_categories"), exact: true }).focus();
    await scan(page, "6130009000158");
    await expect(cartRow(page, `${ARTICLE} focus`)).toBeVisible();
  });

  test("in the amount box, which is left alone", async ({ page }) => {
    const tendered = page.getByLabel(t("field_tendered"), { exact: true });
    await tendered.focus();
    await scan(page, "6130009000158", { terminator: null });
    await page.clock.runFor(150);
    // The cashier's own field keeps what was typed into it and nothing is
    // added: a scan started there is a slip, and guessing is worse than not.
    await expect(cartRow(page, `${ARTICLE} focus`)).toBeHidden();
    await expect(searchBox(page)).toHaveValue("");
  });
});

test.describe("a code no article carries", () => {
  test("says so, and keeps the digits where they are", async ({ page }) => {
    await scan(page, UNKNOWN);
    await expect(page.getByText(t("till_scan_unknown"))).toBeVisible();
    await expect(page.getByText(UNKNOWN)).toBeVisible();
    await expect(searchBox(page)).toHaveValue(UNKNOWN);
  });
});

test.describe("the same article under two numbers", () => {
  test.beforeAll(async ({ request }) => {
    await seed(request, UPC_ARTICLE, UPC_ON_FILE);
  });

  test("a twelve-digit UPC finds the thirteen-digit EAN on file", async ({ page }) => {
    await scan(page, UPC_ON_THE_BOX);
    await expect(cartRow(page, UPC_ARTICLE)).toBeVisible();
    // The row alone is not the whole claim: only `onScan` empties the box,
    // so an empty box is what says the scan went through the canonicalising
    // path rather than the till's own "one thing is visible, add it".
    await expect(searchBox(page)).toHaveValue("");
  });
});

test.describe("a scanner slower than the burst rule, which is still a scanner", () => {
  test.beforeAll(async ({ request }) => {
    await seed(request, `${ARTICLE} slow`, "6130009000165");
  });

  test("its code arrives as typing, and the box is empty after each one", async ({ page }) => {
    // Some models put 100 ms between characters, which is past
    // SCAN_MAX_GAP_MS on purpose: nothing about such a unit can be told from
    // a person, so the burst rule never fires and the code lands in the box
    // as ordinary typing. The till's own Enter has to finish the job, and it
    // has to leave the box empty or the next code appends to this one.
    //
    // This is also the only case in the file where the clock actually moves
    // between characters. Every other `scan()` here runs under a frozen
    // clock, so its gaps are zero and none of them would notice
    // `performance.now()` being replaced by a constant.
    const cdp = await page.context().newCDPSession(page);
    for (const digit of "6130009000165") {
      await cdp.send("Input.dispatchKeyEvent", {
        type: "keyDown",
        key: digit,
        code: `Digit${digit}`,
        text: digit,
      });
      await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", key: digit, code: `Digit${digit}` });
      await page.clock.runFor(100);
    }
    await expect(searchBox(page)).toHaveValue("6130009000165");
    await expect(cartRow(page, `${ARTICLE} slow`)).toBeHidden();

    await page.keyboard.press("Enter");
    await expect(cartRow(page, `${ARTICLE} slow`)).toBeVisible();
    await expect(searchBox(page)).toHaveValue("");
    await cdp.detach();
  });
});

test.describe("a person, who must never be read as a scanner", () => {
  test("Space still works the button it was pressed on", async ({ page }) => {
    // The hook moves focus to the search box on a character that could be
    // part of a code. Space used to count, and a button fires its click on
    // Space's keyup, which by then landed on the box: the button did
    // nothing and the box got a space.
    const all = page.getByRole("button", { name: t("till_all_categories"), exact: true });
    await all.focus();
    await page.keyboard.press("Space");
    await expect(searchBox(page)).toHaveValue("");
    await expect(all).toBeFocused();
  });

  test("a name typed with human pauses adds nothing", async ({ page }) => {
    const box = searchBox(page);
    await box.focus();
    for (const letter of "biscuit") {
      await page.keyboard.press(`Key${letter.toUpperCase()}`);
      await page.clock.runFor(200);
    }
    await page.clock.runFor(200);
    await expect(cartRow(page, ARTICLE)).toBeHidden();
    await expect(box).toHaveValue("biscuit");
  });
});
