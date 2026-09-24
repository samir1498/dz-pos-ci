// The first day of a shop, as one walk: Samir's 2026-09-24 hand-test
// replayed from an empty database, first setup included (plan
// automated-qa-rounds-for-the-shop, round 1). Every figure below is the
// literal he wrote down, never a sum this file works out on its own, so a
// screen that rounds differently or a core that changes a rule without
// telling the paper both fail the same way: the number on the line stops
// matching.
//
// `playwright.first-day.config.ts` runs no globalSetup, so nobody is signed
// in for free: this file uses `@playwright/test`'s own `test`, never
// `./auth`'s (which signs in the fixed owner, a credential this database has
// never heard of). The owner claims the shop through the UI in the first
// `test.step`, and the standalone `request` fixture — its own cookie jar,
// unrelated to the page's (`e2e/auth.ts` says why there are two) — signs in
// right after with the same name and password, over `POST /auth/login`, so
// every API assertion below rides a real session rather than only the
// launch-token header every route already demands.
//
// Four findings shape the walk without failing it: T9 (the till's
// opening-cash prompt is the very next thing an owner meets, so the shop's
// identity is filled in from a direct `page.goto` rather than fighting that
// dialog first), T24 (setup itself asks nothing about the shop), T25 (the
// print button only previews; nothing here asks it to print), T39 (leaving
// `/till` mid-cart drops the basket, so nothing here does). Two more, T31
// and T34, are assertions of the *fixed* behaviour that fail against today's
// core; they sit in their own `test.fail` cases below the walk, so the walk
// itself stays green and Playwright will complain, loudly, the day somebody
// fixes either finding and forgets to delete the annotation.

import { expect, test } from "@playwright/test";
import type { APIRequestContext, Locator, Page } from "@playwright/test";
import { formatCentimes } from "@dzpos/shared";
import { apiHeaders, apiUrl, printedNumber, seriesOf } from "../api";
import { t } from "../messages";

test.describe.configure({ mode: "serial" });

// ---- the cast, named once so every step and both guards read the same
// figures.

const OWNER_NAME = "Nassim Haddad";
const OWNER_PASSWORD = "ouverture2026";

const EAU_NAME = "Eau minérale 1,5L";
const EAU_BARCODE = "6131016000013";
const CAFE_NAME = "Café moulu 250g";
const CAFE_BARCODE = "6130008123457";
const SUCRE_NAME = "Sucre en vrac";
const SUCRE_BARCODE = "2000010000012";
const SUCRE_RENAMED = "Sucre en vrac (renommé)";

const BENALI_NAME = "Épicerie Benali";
const AMINE_NAME = "Distributeur Amine";

/** `crates/retail/src/audit_actions.rs::ACTION_CREDIT_OVERRIDE`. A wire
 * constant, not a translated string, so it is spelled out here the way the
 * core spells it rather than read off a dictionary that has no entry for
 * it. */
const CREDIT_OVERRIDE_ACTION = "document.issue_override";

// ---- what a run leaves behind for its own two guards below. Set once, by
// the walk, and read by nothing else: `test.describe.configure({ mode:
// "serial" })` is what makes the order safe to depend on.
let benaliId: number | null = null;
let tk2Id: number | null = null;
let tk3Id: number | null = null;
let supplierId: number | null = null;

function must<T>(value: T | null, what: string): T {
  if (value === null) throw new Error(`first-day.spec.ts: ${what} was never set by the walk`);
  return value;
}

// ---- the shapes this file reads back off the API. Loose on purpose: a
// field this file never asks about is left out rather than typed and
// ignored.

interface Totals {
  total_ht_centimes: number;
  tva_centimes: number;
  total_ttc_centimes: number;
  stamp_centimes: number;
  net_to_pay_centimes: number;
}

interface Balance {
  old_balance_centimes: number;
  remaining_debt_centimes: number;
  total_debt_centimes: number;
}

interface Sale {
  id: number;
  kind: string;
  series: string;
  ref_document_id: number | null;
  printed_number: string;
  totals: Totals;
  tendered_centimes: number | null;
  change_centimes: number | null;
  warning: string | null;
  balance: Balance | null;
}

interface Party {
  id: number;
  name: string;
  balance_centimes: number;
}

interface Product {
  id: number;
  barcode: string | null;
  qty_on_hand_milli: number;
}

interface AuditEntry {
  entity_id: number | null;
  after: string | null;
}

// ---- small helpers, one job each, mirrored on the pattern the rest of the
// suite already uses (till.spec.ts, till-credit.spec.ts, suppliers.spec.ts).

async function signInAsOwner(request: APIRequestContext): Promise<void> {
  const res = await request.post(`${apiUrl()}/auth/login`, {
    headers: apiHeaders(),
    data: { name: OWNER_NAME, password: OWNER_PASSWORD },
  });
  if (!res.ok()) {
    throw new Error(
      `first-day.spec.ts: the request fixture's own sign-in was refused (${res.status()} ${await res.text()})`,
    );
  }
}

async function partyByName(
  request: APIRequestContext,
  endpoint: "customers" | "suppliers",
  name: string,
): Promise<Party> {
  const res = await request.get(`${apiUrl()}/${endpoint}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const all: Party[] = await res.json();
  const found = all.find((p) => p.name === name);
  if (found === undefined) throw new Error(`first-day.spec.ts: the API does not know ${name}`);
  return found;
}

async function partyById(
  request: APIRequestContext,
  endpoint: "customers" | "suppliers",
  id: number,
): Promise<Party> {
  const res = await request.get(`${apiUrl()}/${endpoint}/${id}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  return res.json();
}

async function stockOf(request: APIRequestContext, barcode: string): Promise<number> {
  const res = await request.get(`${apiUrl()}/products`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const rows: Product[] = await res.json();
  const found = rows.find((p) => p.barcode === barcode);
  if (found === undefined) throw new Error(`first-day.spec.ts: no product carries barcode ${barcode}`);
  return found.qty_on_hand_milli;
}

async function readSale(request: APIRequestContext, id: number): Promise<Sale> {
  const res = await request.get(`${apiUrl()}/sales/${id}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  return res.json();
}

async function overrideAuditRows(request: APIRequestContext): Promise<AuditEntry[]> {
  const res = await request.get(
    `${apiUrl()}/audit-log?action=${encodeURIComponent(CREDIT_OVERRIDE_ACTION)}`,
    { headers: apiHeaders() },
  );
  expect(res.ok()).toBe(true);
  const body: { rows: AuditEntry[] } = await res.json();
  return body.rows;
}

function postedSale(page: Page) {
  return page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
}

/** A scanner's Enter: one unit lands in the cart and the box empties. */
async function scan(page: Page, barcode: string): Promise<void> {
  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.fill(barcode);
  await search.press("Enter");
  await expect(search).toHaveValue("");
}

/** A tile clicked by hand: the search narrows the grid, the click adds one. */
async function addByTile(page: Page, name: string): Promise<void> {
  await page.getByLabel(t("till_search"), { exact: true }).fill(name);
  await page.getByTestId("tiles").getByRole("button", { name }).click();
}

async function bumpQty(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: `${t("till_qty_increase")} ${name}` }).click();
}

/** The search box narrows the list the server answers with, and the answer
 * itself is what commits the choice (till-credit.spec.ts, till-facture.spec.ts). */
async function pickCustomer(page: Page, name: string): Promise<void> {
  await page.getByLabel(t("till_customer_search"), { exact: true }).fill(name);
  await page.getByRole("group", { name: t("till_customer") }).getByRole("button", { name }).click();
}

async function chooseRate(page: Page, label: string): Promise<void> {
  await page.getByRole("combobox", { name: t("field_rate"), exact: true }).click();
  await page.getByRole("option", { name: label, exact: true }).click();
}

interface NewProduct {
  name: string;
  barcode: string;
  cost: string;
  price: string;
  rateLabel: string;
  stock: string;
}

async function addProduct(page: Page, input: NewProduct): Promise<void> {
  await page.getByRole("button", { name: t("products_add") }).click();
  await page.getByLabel(t("field_name")).fill(input.name);
  await page.getByLabel(t("field_barcode"), { exact: true }).fill(input.barcode);
  await page.getByLabel(t("field_price")).fill(input.price);
  await page.getByLabel(t("field_cost"), { exact: true }).fill(input.cost);
  await page.getByLabel(t("field_stock"), { exact: true }).fill(input.stock);
  await chooseRate(page, input.rateLabel);
  await page.getByRole("button", { name: t("action_save") }).click();
  await expect(page.getByRole("row").filter({ hasText: input.name })).toBeVisible();
}

/** A kit select in a purchase order line: open the trigger, click the
 * option (purchases.spec.ts). */
async function choose(page: Page, trigger: Locator, option: string): Promise<void> {
  await trigger.click();
  await page.getByRole("option", { name: option, exact: true }).click();
}

/** A copy row, by the table's own rowgroups: head then body
 * (backups.spec.ts). */
function copiesIn(table: Locator): Locator {
  return table.getByRole("rowgroup").nth(1).getByRole("row");
}

test.describe("the first day", () => {
  test("walks a shop from an empty database to a closed till, a backup taken and restored", async ({
    page,
    request,
  }) => {
    await test.step("1. the owner claims the shop, then its identity", async () => {
      await page.goto("/");
      await expect(page.getByTestId("setup-screen")).toBeVisible();
      await page.getByTestId("setup-name").fill(OWNER_NAME);
      await page.getByTestId("setup-password").fill(OWNER_PASSWORD);
      await page.getByTestId("setup-confirm").fill(OWNER_PASSWORD);
      await page.getByTestId("setup-submit").click();
      await expect(page.getByTestId("shell-topbar")).toBeVisible();
      await expect(page.getByTestId("user-menu-trigger")).toContainText(OWNER_NAME);

      // The standalone `request` fixture is its own session: it has never
      // shown this API a credential, and the owner's is only good from here.
      await signInAsOwner(request);

      // T9: "/" would otherwise land on "/till", which raises the
      // opening-cash prompt the moment nobody holds a shift — before the
      // shop even has a name. Going straight to the settings screen is how
      // this walk gets past it rather than answering it early.
      await page.goto("/settings/shop");
      await expect(page.getByRole("main").getByRole("heading", { name: t("settings_title") })).toBeVisible();
      await page.getByLabel(t("field_name"), { exact: true }).fill("Supérette El Baraka");
      await page.getByLabel(t("field_rc"), { exact: true }).fill("16/00-7654321B22");
      await page.getByLabel(t("field_nif"), { exact: true }).fill("000216987654321");
      await page.getByLabel(t("field_nis"), { exact: true }).fill("000216012345678");
      await page.getByLabel(t("field_ai"), { exact: true }).fill("16012345678");
      await page.getByLabel(t("field_address"), { exact: true }).fill("12 rue Larbi Ben M'hidi, Alger");
      await page.getByLabel(t("field_phone"), { exact: true }).fill("023 45 67 89");
      const storeForm = page.getByRole("form", { name: t("settings_store") });
      await storeForm.getByRole("button", { name: t("action_save") }).click();
      await expect(page.getByRole("status")).toHaveText(t("settings_saved"));

      // The régime the migration seeds is already "réel"; nothing to change,
      // only to find true.
      const settings = await request.get(`${apiUrl()}/settings`, { headers: apiHeaders() });
      expect(settings.ok()).toBe(true);
      const settingsBody: { regime: { regime: string } } = await settings.json();
      expect(settingsBody.regime.regime).toBe("reel");
    });

    await test.step("2. the catalogue: three products", async () => {
      await page.goto("/products");
      await addProduct(page, {
        name: EAU_NAME,
        barcode: EAU_BARCODE,
        cost: "30",
        price: "45",
        rateLabel: t("rate_1900"),
        stock: "48",
      });
      await addProduct(page, {
        name: CAFE_NAME,
        barcode: CAFE_BARCODE,
        cost: "320",
        price: "420",
        rateLabel: t("rate_900"),
        stock: "20",
      });
      await addProduct(page, {
        name: SUCRE_NAME,
        barcode: SUCRE_BARCODE,
        cost: "95",
        price: "110",
        rateLabel: t("rate_0"),
        stock: "50",
      });

      expect(await stockOf(request, EAU_BARCODE)).toBe(48_000);
      expect(await stockOf(request, CAFE_BARCODE)).toBe(20_000);
      expect(await stockOf(request, SUCRE_BARCODE)).toBe(50_000);
    });

    await test.step("3. the till opens with 5 000,00", async () => {
      await page.goto("/");
      await expect(page).toHaveURL(/\/till$/);
      await expect(page.getByTestId("till-open-dialog")).toBeVisible();
      await page.getByTestId("till-open-amount").fill("5000");
      const opened = page.waitForResponse(
        (res) => res.url().endsWith("/till/shifts") && res.request().method() === "POST",
      );
      await page.getByTestId("till-open-submit").click();
      const openedRes = await opened;
      expect(openedRes.status()).toBe(201);
      const shift: { opening_cash_centimes: number } = await openedRes.json();
      expect(shift.opening_cash_centimes).toBe(500_000);
      await expect(page.getByTestId("till-shift-badge")).toBeVisible();
    });

    let tk1: Sale;
    await test.step("4. TK1: cash sale of 2 Eau + 1 Café", async () => {
      await scan(page, EAU_BARCODE);
      await bumpQty(page, EAU_NAME);
      await addByTile(page, CAFE_NAME);
      await page.getByLabel(t("field_tendered"), { exact: true }).fill("1000");
      const issued = postedSale(page);
      await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
      const res = await issued;
      expect(res.status()).toBe(201);
      tk1 = await res.json();
      expect(tk1.totals.total_ht_centimes).toBe(51_000);
      expect(tk1.totals.tva_centimes).toBe(5_490);
      expect(tk1.totals.total_ttc_centimes).toBe(56_490);
      expect(tk1.totals.stamp_centimes).toBe(600);
      expect(tk1.totals.net_to_pay_centimes).toBe(57_090);
      expect(tk1.tendered_centimes).toBe(100_000);
      expect(tk1.change_centimes).toBe(42_910);
      expect(tk1.printed_number).toMatch(printedNumber("TK", 1));

      // Finding T25: the Imprimer button only previews a page, never a real
      // print job, so what is asserted is the confirmation card, not paper.
      const done = page.getByRole("status");
      await expect(done.getByTestId("till-document-number")).toHaveText(printedNumber("TK", 1));
      await expect(done).toContainText(formatCentimes(57_090));
      await expect(done).toContainText(formatCentimes(42_910));
    });

    await test.step("5. customer: Épicerie Benali", async () => {
      await page.goto("/customers");
      await page.getByRole("button", { name: t("customers_add") }).click();
      await expect(page.getByTestId("customer-fiche")).toBeVisible();
      await page.getByLabel(t("field_name"), { exact: true }).fill(BENALI_NAME);
      await page.getByRole("radio", { name: t("party_company"), exact: true }).click();
      await page.getByLabel(t("field_phone"), { exact: true }).fill("0550 12 34 56");
      await page.getByLabel(t("field_rc"), { exact: true }).fill("16/00-1234567B21");
      await page.getByLabel(t("field_nif"), { exact: true }).fill("000216123456789");
      await page.getByLabel(t("field_nis"), { exact: true }).fill("000216123450001");
      await page.getByLabel(t("field_credit_limit"), { exact: true }).fill("2000");
      await page.getByLabel(t("field_warn_threshold"), { exact: true }).fill("1500");
      await page.getByLabel(t("field_opening_debt"), { exact: true }).fill("1000");
      await page.getByRole("button", { name: t("action_save") }).click();
      await expect(page.getByRole("row").filter({ hasText: BENALI_NAME })).toBeVisible();

      const created = await partyByName(request, "customers", BENALI_NAME);
      benaliId = created.id;
      expect(created.balance_centimes).toBe(100_000);
    });

    await test.step("6. TK2: credit sale of 1 Café to Benali", async () => {
      await page.goto("/");
      await expect(page).toHaveURL(/\/till$/);
      await pickCustomer(page, BENALI_NAME);
      await addByTile(page, CAFE_NAME);
      await page.getByRole("radio", { name: t("pay_credit"), exact: true }).click();
      const issued = postedSale(page);
      await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
      const res = await issued;
      expect(res.status()).toBe(201);
      const tk2: Sale = await res.json();
      tk2Id = tk2.id;
      expect(tk2.totals.net_to_pay_centimes).toBe(45_780);
      expect(tk2.balance?.total_debt_centimes).toBe(145_780);
      // Below the 1 500,00 warning: nothing warns yet.
      expect(tk2.warning).toBeNull();

      const done = page.getByRole("status");
      await expect(done.getByTestId("till-near-limit")).toHaveCount(0);
      await expect(done.getByTestId("till-new-balance")).toHaveText(formatCentimes(145_780));

      const afterTk2 = await partyById(request, "customers", must(benaliId, "benaliId"));
      expect(afterTk2.balance_centimes).toBe(145_780);
    });

    await test.step("7. TK3: 2 Café on credit, refused past the limit, then overridden", async () => {
      // The sale that just succeeded clears the cart and the picked
      // customer (till.tsx's own pay.onSuccess); the walk re-picks the same
      // way till-credit.spec.ts does before every new attempt.
      await pickCustomer(page, BENALI_NAME);
      await addByTile(page, CAFE_NAME);
      await bumpQty(page, CAFE_NAME);
      await page.getByRole("radio", { name: t("pay_credit"), exact: true }).click();
      const refused = postedSale(page);
      await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
      expect((await refused).status()).toBe(422);

      const overridden = postedSale(page);
      await page.getByRole("button", { name: t("till_override"), exact: true }).click();
      await page.getByTestId("till-override-dialog-confirm").click();
      const overriddenRes = await overridden;
      expect(overriddenRes.status()).toBe(201);
      const tk3: Sale = await overriddenRes.json();
      tk3Id = tk3.id;
      expect(tk3.totals.net_to_pay_centimes).toBe(91_560);
      expect(tk3.balance?.total_debt_centimes).toBe(237_340);
      // Which warning code the override carries is T31's `test.fail` below;
      // the walk leaves it unasserted so the fix does not turn the walk red.

      const done = page.getByRole("status");
      await expect(done.getByTestId("till-near-limit")).toBeVisible();
      await expect(done.getByTestId("till-new-balance")).toHaveText(formatCentimes(237_340));

      const afterTk3 = await partyById(request, "customers", must(benaliId, "benaliId"));
      expect(afterTk3.balance_centimes).toBe(237_340);

      // An audit row exists for the override and names the document it
      // produced; T31 is only about what its `warning` field says.
      const rows = await overrideAuditRows(request);
      const overrideRow = rows.find((row) => row.entity_id === tk3Id);
      expect(overrideRow, JSON.stringify(rows)).toBeDefined();
    });

    await test.step("8. debt payment: 500,00 cash from Benali", async () => {
      await page.goto("/customers");
      const row = page.getByRole("row").filter({ hasText: BENALI_NAME });
      await row.getByRole("link", { name: BENALI_NAME }).click();
      await page.getByRole("button", { name: t("customers_pay"), exact: true }).click();
      await expect(page.getByTestId("customer-pay-dialog")).toBeVisible();
      await page.getByLabel(t("field_payment_amount"), { exact: true }).fill("500");
      await page.getByRole("radio", { name: t("payment_cash"), exact: true }).click();
      await page.getByRole("button", { name: t("action_take_payment"), exact: true }).click();
      await expect(page.getByText(t("customers_paid"))).toBeVisible();
      await expect(page.getByTestId("customer-balance")).toContainText(formatCentimes(187_340));

      const afterPayment = await partyById(request, "customers", must(benaliId, "benaliId"));
      expect(afterPayment.balance_centimes).toBe(187_340);

      // Which papers the 500,00 settled is T34's `test.fail` below; only
      // the balance, which the fix does not move, is asserted here.
    });

    let facture: Sale;
    await test.step("9. facture FA1: cash sale of 2 Eau + 1 Sucre to Benali", async () => {
      await page.goto("/");
      await expect(page).toHaveURL(/\/till$/);
      await pickCustomer(page, BENALI_NAME);
      await scan(page, EAU_BARCODE);
      await bumpQty(page, EAU_NAME);
      await addByTile(page, SUCRE_NAME);
      await page.getByRole("radio", { name: t("till_facture"), exact: true }).click();
      await page.getByLabel(t("field_tendered"), { exact: true }).fill("1000");
      const issued = postedSale(page);
      await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
      const res = await issued;
      expect(res.status()).toBe(201);
      facture = await res.json();
      expect(facture.kind).toBe("facture");
      // 217,10 DA is under the 300,00 DA floor: no droit de timbre.
      expect(facture.totals.stamp_centimes).toBe(0);
      expect(facture.totals.net_to_pay_centimes).toBe(21_710);
      expect(facture.tendered_centimes).toBe(100_000);
      expect(facture.change_centimes).toBe(78_290);
      expect(facture.printed_number).toMatch(printedNumber("FA", 1));

      const done = page.getByRole("status");
      await expect(done.getByTestId("till-document-number")).toHaveText(printedNumber("FA", 1));
      await expect(done).toContainText(formatCentimes(21_710));
      await expect(done).toContainText(formatCentimes(78_290));

      // Paid in cash: the debt does not move.
      const afterFacture = await partyById(request, "customers", must(benaliId, "benaliId"));
      expect(afterFacture.balance_centimes).toBe(187_340);
    });

    await test.step("10. avoir AV1: 1 Eau returned from FA1, credited to the account", async () => {
      await page.goto("/documents");
      await page.getByRole("button", { name: facture.printed_number }).click();
      const detail = page.getByRole("region", { name: t("documents_detail") });
      await detail.getByRole("button", { name: t("documents_avoir_new") }).click();

      const dialog = page.getByRole("dialog");
      await dialog.getByLabel(`${t("documents_avoir_qty")} ${EAU_NAME}`, { exact: true }).fill("1");
      const issued = page.waitForResponse(
        (res) => res.url().includes("/avoir") && res.request().method() === "POST",
      );
      // Left on its default, "ledger": the facture named a buyer, so that is
      // the account this credit lands on rather than cash out of the drawer.
      await dialog.getByRole("button", { name: t("documents_avoir_write"), exact: true }).click();
      const res = await issued;
      expect(res.status()).toBe(201);
      const avoir: Sale = await res.json();
      expect(avoir.kind).toBe("avoir");
      expect(avoir.series).toMatch(seriesOf("doc_avoir"));
      expect(avoir.ref_document_id).toBe(facture.id);
      expect(avoir.totals.net_to_pay_centimes).toBe(5_355);

      await expect(detail.getByText(avoir.printed_number)).toBeVisible();
      await expect(detail).toContainText(formatCentimes(5_355));

      const afterAvoir = await partyById(request, "customers", must(benaliId, "benaliId"));
      expect(afterAvoir.balance_centimes).toBe(181_985);
      // 45 000: 48 000 opening, -2 000 on TK1, -2 000 on the facture, +1 000
      // back on the avoir.
      expect(await stockOf(request, EAU_BARCODE)).toBe(45_000);
    });

    await test.step("11. supplier Distributeur Amine, a purchase received and part-paid", async () => {
      await page.goto("/suppliers");
      await page.getByTestId("page-header").getByRole("button", { name: t("suppliers_add") }).click();
      await page.getByLabel(t("field_name"), { exact: true }).fill(AMINE_NAME);
      await page.getByLabel(t("field_phone"), { exact: true }).fill("0661 22 33 44");
      await page.getByRole("button", { name: t("action_save") }).click();
      await expect(page.getByTestId("supplier-sheet")).toBeHidden();
      await expect(page.getByRole("row").filter({ hasText: AMINE_NAME })).toBeVisible();

      await page.goto("/purchases/new");
      await expect(page.getByRole("heading", { name: t("purchases_new") })).toBeVisible();
      await choose(page, page.getByRole("combobox", { name: t("col_supplier"), exact: true }), AMINE_NAME);

      const pickProduct = page.getByRole("combobox", { name: t("col_product"), exact: true });
      const qty = page.getByRole("textbox", { name: t("col_qty"), exact: true });
      const cost = page.getByRole("textbox", { name: t("col_unit_cost"), exact: true });
      await choose(page, pickProduct.first(), EAU_NAME);
      await qty.first().fill("24");
      await cost.first().fill("28");

      await page.getByRole("button", { name: t("purchases_add_line") }).click();
      await choose(page, pickProduct.nth(1), CAFE_NAME);
      await qty.nth(1).fill("100");
      await cost.nth(1).fill("300");

      // `field_receive_now` stays checked: the goods came with the bon, so
      // the delivery happens at issue rather than through a second step.
      await page.getByLabel(t("field_paid_now"), { exact: true }).fill("1836");
      await page.getByRole("button", { name: t("purchases_save") }).click();

      await page.waitForURL(/\/purchases\/\d+$/);
      await expect(page.getByTestId("purchase-status")).toHaveText(t("purchase_status_received"));

      const supplier = await partyByName(request, "suppliers", AMINE_NAME);
      supplierId = supplier.id;
      // 24 x 28,00 + 100 x 300,00 = 30 672,00, less the 1 836,00 paid at once.
      expect(await partyById(request, "suppliers", supplierId)).toMatchObject({
        balance_centimes: 2_883_600,
      });
      expect(await stockOf(request, EAU_BARCODE)).toBe(69_000);
      expect(await stockOf(request, CAFE_BARCODE)).toBe(116_000);
    });

    await test.step("12. a second supplier payment: 5 000,00, never through the till", async () => {
      await page.goto("/suppliers");
      const before = page.getByRole("row").filter({ hasText: AMINE_NAME });
      await expect(before.getByRole("cell", { name: formatCentimes(2_883_600), exact: true })).toBeVisible();
      await before.getByRole("link", { name: AMINE_NAME }).click();
      await page.getByTestId("supplier-pay-open").click();
      await page.getByLabel(t("field_payment_amount"), { exact: true }).fill("5000");
      await page.getByRole("button", { name: t("action_pay_supplier"), exact: true }).click();
      await expect(page.getByText(t("suppliers_paid"))).toBeVisible();

      expect(await partyById(request, "suppliers", must(supplierId, "supplierId"))).toMatchObject({
        balance_centimes: 2_383_600,
      });
    });

    await test.step("13. close the till: counted 6 288,00, no discrepancy", async () => {
      // Supplier payments never touch the drawer: 5 000,00 opening + TK1's
      // 570,90 + Benali's 500,00 + FA1's 217,10.
      const open = await request.get(`${apiUrl()}/till/shifts/open`, { headers: apiHeaders() });
      expect(open.ok()).toBe(true);
      const report: { shift: { id: number }; expected_centimes: number } | null = await open.json();
      if (report === null) throw new Error("first-day.spec.ts: nobody holds an open shift to close");
      expect(report.expected_centimes).toBe(628_800);

      await page.goto("/");
      await expect(page).toHaveURL(/\/till$/);
      await page.getByTestId("till-close-trigger").click();
      await page.getByTestId("till-counted").fill("6288");
      const closed = page.waitForResponse(
        (res) => /\/till\/shifts\/\d+\/close$/.test(res.url()) && res.request().method() === "POST",
      );
      await page.getByTestId("till-close-submit").click();
      const closedRes = await closed;
      expect(closedRes.status()).toBe(200);
      const shift: { difference_centimes: number | null } = await closedRes.json();
      expect(shift.difference_centimes).toBe(0);
      await expect(page.getByTestId("till-closed-banner")).toBeVisible();
    });

    await test.step("14. stock at day's end", async () => {
      expect(await stockOf(request, EAU_BARCODE)).toBe(69_000);
      expect(await stockOf(request, CAFE_BARCODE)).toBe(116_000);
      expect(await stockOf(request, SUCRE_BARCODE)).toBe(49_000);
    });

    await test.step("15. a backup, a change undone, the day's figures untouched", async () => {
      await page.goto("/settings/backups");
      await expect(page.getByRole("heading", { name: t("settings_backups") })).toBeVisible();
      await page.getByRole("button", { name: t("action_backup_now") }).click();
      await expect(page.getByRole("status")).toHaveText(t("backups_created"));

      await page.goto("/products");
      const sucreRow = page.getByRole("row").filter({ hasText: SUCRE_NAME });
      await sucreRow.getByRole("button", { name: `${t("products_edit")} ${SUCRE_NAME}` }).click();
      await page.getByLabel(t("field_name")).fill(SUCRE_RENAMED);
      await page.getByRole("button", { name: t("action_save") }).click();
      await expect(page.getByRole("row").filter({ hasText: SUCRE_RENAMED })).toBeVisible();

      await page.goto("/settings/backups");
      const copies = copiesIn(page.getByTestId("backups-table"));
      await expect(copies).toHaveCount(1);
      await copies.first().getByRole("button", { name: t("action_restore") }).click();
      const asking = page.getByRole("dialog");
      await expect(asking).toContainText(t("backups_confirm_restore"));
      await asking.getByRole("button", { name: t("backups_restore_confirm") }).click();
      await expect(page.getByRole("status")).toHaveText(t("backups_restored"));

      await page.goto("/products");
      await expect(page.getByRole("row").filter({ hasText: SUCRE_NAME })).toBeVisible();
      await expect(page.getByRole("row").filter({ hasText: SUCRE_RENAMED })).toHaveCount(0);

      expect(await partyById(request, "customers", must(benaliId, "benaliId"))).toMatchObject({
        balance_centimes: 181_985,
      });
      expect(await partyById(request, "suppliers", must(supplierId, "supplierId"))).toMatchObject({
        balance_centimes: 2_383_600,
      });
    });
  });

  // ---- T31. The core writes an audit row for every override, and the
  // `warning` it carries should say which rule was crossed: "over_limit"
  // when the balance the override let through is past the credit limit, as
  // TK3's is. Today `Warning` has one code, `near_limit`, for both cases, so
  // this reads the wrong half against `document.issue_override`'s own row.
  test.fail("T31: the override row for TK3 should say over_limit, not near_limit", async ({
    request,
  }) => {
    await signInAsOwner(request);
    const rows = await overrideAuditRows(request);
    const row = rows.find((r) => r.entity_id === must(tk3Id, "tk3Id"));
    if (row === undefined) throw new Error("first-day.spec.ts: the walk left no override row for TK3");
    if (row.after === null) throw new Error("first-day.spec.ts: the override row carries no after state");
    const after: { warning: string | null } = JSON.parse(row.after);
    expect(after.warning).toBe("over_limit");
  });

  // ---- T34. A payment on Benali's account settled the two credit tickets
  // oldest first and left the opening debt untouched, because the opening
  // balance is a ledger row and not a document a payment can be allocated
  // against. The fix the finding asks for settles the opening debt first —
  // it existed before either ticket — leaving TK2 and TK3 exactly as issued.
  test.fail("T34: a debt payment should settle the opening debt before either ticket", async ({
    request,
  }) => {
    await signInAsOwner(request);
    const tk2 = await readSale(request, must(tk2Id, "tk2Id"));
    const tk3 = await readSale(request, must(tk3Id, "tk3Id"));
    expect(tk2.balance?.remaining_debt_centimes).toBe(45_780);
    expect(tk3.balance?.remaining_debt_centimes).toBe(91_560);
    // TK2 and TK3 carrying their full, untouched amounts is what "the
    // opening debt is reduced to 500,00" means here: the account's total
    // (1 873,40, unaffected by which of the three the 500,00 landed on) is
    // 1 000,00 opening + 457,80 + 915,60 less the payment, so if neither
    // ticket moved the opening debt is what did, down from 1 000,00 to
    // 500,00. There is no separate field for the opening portion alone to
    // assert directly against.
    const benali = await partyById(request, "customers", must(benaliId, "benaliId"));
    expect(benali.balance_centimes).toBe(187_340);
  });
});
