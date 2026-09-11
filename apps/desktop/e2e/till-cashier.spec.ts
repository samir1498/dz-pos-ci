// A cashier in the browser suite (M4 T9). Every other spec here signs in
// as the shop's one owner (`./auth`), so deleting every
// `permissions::require` call in the API would still leave all twenty-one
// green: nothing ever asks to be refused. This file is the one that puts a
// cashier behind the wheel and drives the refusals the milestone's own
// permission table promises, through the real API and a real SQLite row,
// never a mock.
//
// Shape chosen: one new spec file, not a fixture the other twenty take a
// role parameter on. A parameterised `test` in `./auth` would have to
// change what every one of those specs imports and would risk their
// passing owner flows to buy a cashier flow none of them asked for; a
// second file costs nothing they have and reads `./auth` only for the
// owner constants it already exports. If a role-parameterised fixture
// earns its keep later (a manager needs the same treatment), this file is
// what it would generalise from.
//
// Two identities share the run, so this file does not import `./auth`'s
// `test`: that override signs `context` and `request` in as the owner
// before a test ever sees them, and this file needs the owner on one
// session and a cashier on another, at the same time. Plain
// `@playwright/test` instead: the test-scoped `request` fixture signs in
// as the owner and seeds; the test's own `context` (and so `page`) signs in
// as the cashier before its first navigation, the same order `./auth`
// itself uses and for the same reason (a same-site cookie has to be on the
// jar before the first `page.goto`). One cashier fiche is made once, in
// `beforeAll`, over a standalone `APIRequestContext` `./auth`'s `test`
// never hands to a hook: a fiche's name is unique per shop
// (`services::users::refuse_taken_name`), and every test in this file
// signs in as the same one.
//
// Placement: named to sort after `products.spec.ts` on purpose.
// `products.spec.ts`'s first assertion needs an empty catalogue, and this
// file's own sales leave two products behind for the rest of the run to
// find, the way every spec after it already does (`till.spec.ts` runs
// against a catalogue `products.spec.ts`, `purchases.spec.ts` and others
// filled first). It hands the discount threshold it raises back to zero
// before it leaves, the way `settings.spec.ts` hands the régime back and
// `theme.spec.ts` hands the theme back, even though nothing downstream
// reads that setting today.

import { test, expect, request as pwRequest } from "@playwright/test";
import type { APIRequestContext, Page } from "@playwright/test";
import { apiHeaders, apiUrl } from "./api";
import { OWNER_NAME, OWNER_PASSWORD } from "./auth";
import { t } from "./messages";

const CASHIER_NAME = "Caissière e2e";
/** Four digits, no run, no repeat (`services::users::validate_pin`). */
const CASHIER_PIN = "2580";

let cashierId: number;

function day(offsetDays: number): string {
  const d = new Date(Date.now() + 3600_000);
  d.setUTCDate(d.getUTCDate() + offsetDays);
  return d.toISOString().slice(0, 10);
}

async function signInOwner(api: APIRequestContext): Promise<void> {
  const res = await api.post(`${apiUrl()}/auth/login`, {
    headers: apiHeaders(),
    data: { name: OWNER_NAME, password: OWNER_PASSWORD },
  });
  if (!res.ok()) {
    throw new Error(`till-cashier.spec.ts: owner sign-in refused (${res.status()} ${await res.text()})`);
  }
}

async function signInCashier(api: APIRequestContext): Promise<void> {
  const res = await api.post(`${apiUrl()}/auth/login`, {
    headers: apiHeaders(),
    data: { user_id: cashierId, pin: CASHIER_PIN },
  });
  if (!res.ok()) {
    throw new Error(`till-cashier.spec.ts: cashier sign-in refused (${res.status()} ${await res.text()})`);
  }
}

/** Playwright starts a fresh worker process for the tests after any test in
 * this file fails (its own documented isolation guarantee), which reruns
 * `beforeAll`/`afterAll` in that new worker. Both touch state that is not
 * naturally idempotent — a uniquely-named fiche, an appended-only dated
 * setting — so both tolerate landing on a value a previous worker already
 * reached rather than treating that as a setup failure. */
async function setDiscountThreshold(owner: APIRequestContext, bps: number): Promise<void> {
  const res = await owner.post(`${apiUrl()}/settings/discount-threshold`, {
    headers: apiHeaders(),
    data: { threshold_bps: bps, valid_from: day(0) },
  });
  if (res.ok()) return;
  const body: { error?: { message?: string } } = await res.json().catch(() => ({}));
  const already = body.error?.message?.includes("already under that discount threshold") ?? false;
  if (already) return;
  throw new Error(
    `till-cashier.spec.ts: could not set the discount threshold to ${bps} bps (${res.status()} ${JSON.stringify(body)})`,
  );
}

async function seedProduct(
  owner: APIRequestContext,
  input: { name: string; barcode: string; price: number; cost?: number; wholesale?: number },
): Promise<number> {
  const res = await owner.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: input.name,
      barcode: input.barcode,
      category_id: 1,
      unit: "piece",
      cost_centimes: input.cost ?? 0,
      selling_centimes: input.price,
      wholesale_centimes: input.wholesale ?? null,
      qty_on_hand_milli: 10_000,
      low_stock_at_milli: 0,
      rate_bps: 0,
      active: true,
    },
  });
  expect(res.status(), await res.text()).toBe(201);
  const created: { id: number } = await res.json();
  return created.id;
}

async function seedCreditCustomer(
  owner: APIRequestContext,
  input: { name: string; limit: number },
): Promise<number> {
  const res = await owner.post(`${apiUrl()}/customers`, {
    headers: apiHeaders(),
    data: {
      name: input.name,
      party_kind: "consumer",
      phone: "0555 11 22 33",
      address: null,
      rc: null,
      nif: null,
      nis: null,
      ai: null,
      credit_limit_centimes: input.limit,
      warn_threshold_centimes: null,
      notes: null,
      active: true,
      opening_debt_centimes: null,
    },
  });
  expect(res.status(), await res.text()).toBe(201);
  const created: { id: number } = await res.json();
  return created.id;
}

interface ErrorBody {
  error: {
    code: string;
    message: string;
    permission?: string;
    field?: string;
    balance_after_centimes?: number;
    credit_limit_centimes?: number;
  };
}

/** Picks the customer at the till, the way `till-credit.spec.ts`'s own
 * helper does. */
async function pickCustomer(page: Page, name: string): Promise<void> {
  await page.getByLabel(t("till_customer_search"), { exact: true }).fill(name);
  const list = page.getByRole("group", { name: t("till_customer") });
  await list.getByRole("button", { name }).click();
}

/** The cashier's fiche, made once. Idempotent on purpose: Playwright starts
 * a fresh worker process for the remaining tests after any test in this
 * file fails, and a fresh worker reruns `beforeAll`. A second attempt at the
 * same name is refused with `conflict` (`services::users::refuse_taken_name`)
 * rather than a fresh row, so the fiche the first worker made is looked up
 * and reused instead of treating that refusal as a setup failure. */
async function makeOrFindCashier(owner: APIRequestContext): Promise<number> {
  const created = await owner.post(`${apiUrl()}/users`, {
    headers: apiHeaders(),
    data: { name: CASHIER_NAME, role: "cashier" },
  });
  if (created.status() === 201) {
    const user: { id: number } = await created.json();
    return user.id;
  }
  if (created.status() !== 409) {
    throw new Error(`till-cashier.spec.ts: could not create the cashier (${created.status()} ${await created.text()})`);
  }
  const list = await owner.get(`${apiUrl()}/users`, { headers: apiHeaders() });
  if (!list.ok()) {
    throw new Error(`till-cashier.spec.ts: could not list users to find the cashier (${list.status()} ${await list.text()})`);
  }
  const users: { id: number; name: string }[] = await list.json();
  const found = users.find((u) => u.name === CASHIER_NAME);
  if (found === undefined) {
    throw new Error(`till-cashier.spec.ts: "${CASHIER_NAME}" was refused as taken but is not in the user list`);
  }
  return found.id;
}

test.beforeAll(async () => {
  const owner = await pwRequest.newContext();
  try {
    await signInOwner(owner);
    cashierId = await makeOrFindCashier(owner);
    // A PIN reset is the same call as a first PIN
    // (`services::users::set_pin`'s own doc), so this is safe to repeat.
    const pinned = await owner.post(`${apiUrl()}/users/${cashierId}/pin`, {
      headers: apiHeaders(),
      data: { pin: CASHIER_PIN },
    });
    expect(pinned.ok(), await pinned.text()).toBe(true);
    // Zero refuses every discount to a cashier (features.md §5: "a shop
    // that has never set one reads zero"), which is a real threshold but
    // not one the discount test below can be sure this run's other specs
    // left in place. Raised on purpose, and handed back in `afterAll`.
    await setDiscountThreshold(owner, 500);
  } finally {
    await owner.dispose();
  }
});

test.afterAll(async () => {
  const owner = await pwRequest.newContext();
  try {
    await signInOwner(owner);
    await setDiscountThreshold(owner, 0);
  } finally {
    await owner.dispose();
  }
});

test("a cashier rings a sale up — the one thing every role may do", async ({ page, context, request }) => {
  await signInOwner(request);
  const productId = await seedProduct(request, {
    name: "Savon Caissier e2e",
    barcode: "6130009200011",
    price: 20_000, // 200,00
  });
  expect(productId).toBeGreaterThan(0);

  await signInCashier(context.request);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.fill("Savon Caissier e2e");
  await page.getByTestId("tiles").getByRole("button", { name: "Savon Caissier e2e" }).click();
  // Generously over the price: this is a cash sale, not a fiscal fixture,
  // and change is not what this test is about.
  await page.getByLabel(t("field_tendered"), { exact: true }).fill("2000");

  const issued = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  const response = await issued;
  expect(response.status(), await response.text()).toBe(201);
  const sale: { id: number } = await response.json();
  expect(sale.id).toBeGreaterThan(0);

  // The confirmation the screen shows, not only the network call: a
  // document number on the page proves the whole flow landed, not just
  // that a request went out.
  await expect(page.getByTestId("till-document-number")).toBeVisible();
});

test("a discount above the shop's threshold is refused, and names that permission", async ({
  page,
  context,
  request,
}) => {
  await signInOwner(request);
  const productId = await seedProduct(request, {
    name: "Huile Caissier e2e",
    barcode: "6130009200028",
    price: 100_000, // 1 000,00; the threshold is 5 %, so 50,00 is the most a
    // cashier may take off without asking, and this test asks for twice that.
  });
  expect(productId).toBeGreaterThan(0);

  await signInCashier(context.request);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.fill("Huile Caissier e2e");
  await page.getByTestId("tiles").getByRole("button", { name: "Huile Caissier e2e" }).click();
  await page.getByLabel(t("field_global_discount"), { exact: true }).fill("100");
  await page.getByLabel(t("field_tendered"), { exact: true }).fill("2000");

  const refused = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  const response = await refused;
  expect(response.status()).toBe(403);
  const body: ErrorBody = await response.json();
  expect(body.error.code).toBe("forbidden");
  expect(body.error.permission).toBe("discount_above_threshold");

  // The screen still says something rather than swallowing the refusal —
  // though not, today, the right something. `till.tsx` keeps its own
  // `ERROR_KEY` map rather than the shared one `audit.tsx` and
  // `settings_.users.tsx` read from `@/lib/fields`, and that map has no
  // `forbidden` entry, so a cashier refused at the till reads the generic
  // "Une erreur est survenue." instead of the "you don't have permission"
  // sentence the shared map already knows. `apps/desktop/src/routes/` is
  // out of scope for this branch (the task's own instruction), so this
  // locks down what the screen actually does today rather than what it
  // should; the fix belongs in `till.tsx`'s `ERROR_KEY`, and it is reported
  // as a product gap rather than patched here.
  await expect(page.getByRole("alert").filter({ hasText: t("error_unknown") })).toBeVisible();
});

test("a line priced under the product's card price is refused, and names that permission", async ({
  context,
  request,
}) => {
  await signInOwner(request);
  const price = 50_000; // 500,00
  const productId = await seedProduct(request, {
    name: "Farine Caissier e2e",
    barcode: "6130009200035",
    price,
  });

  // No screen offers this today (`apps/desktop/src/routes/-till/cart.tsx`
  // has no price field on a line): the till always sends the line at the
  // product's own price. The gate is real all the same
  // (`crates/core/src/services/sales.rs`, `Permission::ChangePriceAtTheTill`),
  // so this is the second half of the finding, asked for directly: what
  // the server refuses when somebody asks anyway.
  await signInCashier(context.request);
  const res = await context.request.post(`${apiUrl()}/sales`, {
    headers: apiHeaders(),
    data: {
      lines: [{ product_id: productId, qty_milli: 1_000, unit_price_centimes: price - 1_000 }],
      global_discount_centimes: 0,
      payment_mode: "cash",
      kind: "ticket",
    },
  });
  expect(res.status()).toBe(403);
  const body: ErrorBody = await res.json();
  expect(body.error.code).toBe("forbidden");
  expect(body.error.permission).toBe("change_price_at_the_till");
});

test("a credit sale past the limit is refused, and refused again — differently — on override", async ({
  page,
  context,
  request,
}) => {
  await signInOwner(request);
  const CUSTOMER = "Client Crédit Caissier e2e";
  const LIMIT = 100_000; // 1 000,00
  const PRICE = 150_000; // 1 500,00: over the limit on the first line already,
  // so this test needs no warning step to reach the refusal.
  const productId = await seedProduct(request, {
    name: "Ciment Caissier e2e",
    barcode: "6130009200042",
    price: PRICE,
  });
  const customerId = await seedCreditCustomer(request, { name: CUSTOMER, limit: LIMIT });

  await signInCashier(context.request);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  await pickCustomer(page, CUSTOMER);
  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.fill("Ciment Caissier e2e");
  await page.getByTestId("tiles").getByRole("button", { name: "Ciment Caissier e2e" }).click();
  await page.getByRole("radio", { name: t("pay_credit"), exact: true }).click();

  const firstRefusal = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  const first = await firstRefusal;
  expect(first.status()).toBe(422);
  const firstBody: ErrorBody = await first.json();
  expect(firstBody.error.code).toBe("credit_limit");
  expect(firstBody.error.balance_after_centimes).toBe(PRICE);
  expect(firstBody.error.credit_limit_centimes).toBe(LIMIT);

  // The screen reading the same two figures back, not a generic sentence:
  // this is the refusal a cashier who rang the sale up plainly gets.
  await expect(page.getByTestId("till-balance-after")).toBeVisible();
  await expect(page.getByTestId("till-credit-limit")).toBeVisible();

  // The till's own override button: no permission hides it (nothing in
  // `apps/desktop/src` gates it on the client), so a cashier can press it.
  // What answers is the point of this half of the test.
  page.once("dialog", (dialog) => void dialog.accept());
  const secondRefusal = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  await page.getByRole("button", { name: t("till_override"), exact: true }).click();
  const second = await secondRefusal;
  expect(second.status()).toBe(403);
  const secondBody: ErrorBody = await second.json();
  expect(secondBody.error.code).toBe("forbidden");
  expect(secondBody.error.permission).toBe("override_credit_block");

  // A `forbidden` carries neither figure `creditRefusal`
  // (`apps/desktop/src/routes/-till/payment.tsx`) reads for, so the
  // till's own credit panel drops away and a plain alert takes its place —
  // the trap the milestone's review named: two refusals that look alike on
  // screen and are not the same guard. What the alert says is today's
  // "Une erreur est survenue." rather than a named permission, the same
  // `till.tsx` `ERROR_KEY` gap the discount test above documents; this
  // assertion is on the panel disappearing and something replacing it, and
  // the two response bodies above are what actually tells the refusals
  // apart.
  await expect(page.getByTestId("till-balance-after")).toBeHidden();
  await expect(page.getByRole("alert").filter({ hasText: t("error_unknown") })).toBeVisible();

  // Nothing was written by either refusal: the ledger the sale would have
  // moved is still exactly what it was before the till ever asked.
  const ledger = await (
    await request.get(`${apiUrl()}/customers/${customerId}/ledger`, { headers: apiHeaders() })
  ).json();
  expect(ledger.balance_centimes).toBe(0);
  expect(ledger.entries).toHaveLength(0);

  // Both refusals are in the log all the same — the fix this milestone's
  // own review found missing (`crates/core/src/services/sales.rs`,
  // `audit::ACTION_CREDIT_BLOCKED` written outside the rolled-back
  // transaction) — and the two rows read apart by `asked_to_override`:
  // false for the plain refusal, true for the one that tried to pass it.
  const logRes = await request.get(
    `${apiUrl()}/audit-log?user_id=${cashierId}&action=sale.credit_blocked`,
    { headers: apiHeaders() },
  );
  expect(logRes.ok()).toBe(true);
  const log: { rows: { after: string | null }[] } = await logRes.json();
  const askedToOverride = log.rows
    .map((row) => (row.after === null ? null : (JSON.parse(row.after) as { asked_to_override: boolean })))
    .map((after) => after?.asked_to_override);
  expect(askedToOverride).toContain(false);
  expect(askedToOverride).toContain(true);
});

test("the staff list and the audit log refuse a cashier at the server, not only on the sidebar", async ({
  page,
  context,
  request,
}) => {
  await signInOwner(request);
  await signInCashier(context.request);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  // The screen's own defence, checked first, and named as not being the
  // defence: the nav entry an owner sees is absent, while an ordinary
  // entry beside it is still there.
  await expect(page.getByTestId("nav-till")).toBeVisible();
  await expect(page.getByTestId("nav-audit")).toHaveCount(0);

  // Typing the address anyway. No `beforeLoad` guard sits in front of
  // either screen (`apps/desktop/src/routes/audit.tsx`'s own comment); the
  // server is what actually says no. Matched against the API's own base
  // URL and not just a path suffix: Vite serves the SPA shell for
  // `/settings/users` itself as a 200 `GET .../settings/users`, which also
  // "ends with /users" and would otherwise be mistaken for the API call.
  const auditLog = page.waitForResponse(
    (res) => res.url().startsWith(`${apiUrl()}/audit-log`) && res.request().method() === "GET",
  );
  await page.goto("/audit");
  const auditRes = await auditLog;
  expect(auditRes.status()).toBe(403);
  const auditBody: ErrorBody = await auditRes.json();
  expect(auditBody.error.code).toBe("forbidden");
  expect(auditBody.error.permission).toBe("see_audit_log");
  await expect(page.getByRole("alert").filter({ hasText: t("error_forbidden") })).toBeVisible();

  const users = page.waitForResponse(
    (res) => res.url() === `${apiUrl()}/users` && res.request().method() === "GET",
  );
  await page.goto("/settings/users");
  const usersRes = await users;
  expect(usersRes.status()).toBe(403);
  const usersBody: ErrorBody = await usersRes.json();
  expect(usersBody.error.code).toBe("forbidden");
  expect(usersBody.error.permission).toBe("manage_users");
  await expect(page.getByRole("alert").filter({ hasText: t("error_forbidden") })).toBeVisible();

  // Both refusals left the generic row the same review's second fix added
  // (`crates/api/src/session.rs::require` calling
  // `permissions::record_refusal` on every gate-table refusal): naming the
  // permission and the route that said no, independent of the 403 body a
  // client happened to read back above.
  const refusalLog = await request.get(
    `${apiUrl()}/audit-log?user_id=${cashierId}&action=permission.refused`,
    { headers: apiHeaders() },
  );
  expect(refusalLog.ok(), await refusalLog.text()).toBe(true);
  const refusals: { rows: { after: string | null }[] } = await refusalLog.json();
  const refused = refusals.rows
    .map((row) => (row.after === null ? null : (JSON.parse(row.after) as { permission: string })))
    .map((after) => after?.permission);
  expect(refused).toContain("see_audit_log");
  expect(refused).toContain("manage_users");
});

test("purchases and the dashboard refuse a cashier at the server too, sidebar hidden and the row left behind", async ({
  page,
  context,
  request,
}) => {
  await signInOwner(request);
  await signInCashier(context.request);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  // Same screen-first check as the staff/audit test above, on the two
  // routes M4 T5's review gated for the same reason (M4 T5 review,
  // 2026-09-11): the entries are gone, an ordinary one beside them is not.
  await expect(page.getByTestId("nav-till")).toBeVisible();
  await expect(page.getByTestId("nav-suppliers")).toBeVisible();
  await expect(page.getByTestId("nav-dashboard")).toHaveCount(0);
  await expect(page.getByTestId("nav-purchases")).toHaveCount(0);

  // Typing the addresses anyway. Matched on the API's own origin and an
  // exact path, not a suffix: Vite's dev server answers its own GET
  // `/dashboard` (the SPA shell, for the page navigation itself) with a
  // 200 on the *web* origin, same path, before the API call on the
  // *API* origin ever resolves, and a bare pathname match would catch that
  // one instead. The exact path (not a suffix) is what keeps `/dashboard`
  // from also catching the screen's separate `/dashboard/series` call; the
  // origin check is what the staff/audit test's `/users` exact-URL match
  // and the audit-log `startsWith` do for the same reason, spelled out
  // because two lookalike requests share a path here, not just a suffix.
  const apiOrigin = new URL(apiUrl()).origin;
  const dashboard = page.waitForResponse((res) => {
    const url = new URL(res.url());
    return url.origin === apiOrigin && url.pathname === "/dashboard" && res.request().method() === "GET";
  });
  await page.goto("/dashboard");
  const dashboardRes = await dashboard;
  expect(dashboardRes.status()).toBe(403);
  const dashboardBody: ErrorBody = await dashboardRes.json();
  expect(dashboardBody.error.code).toBe("forbidden");
  expect(dashboardBody.error.permission).toBe("see_reports");

  const purchases = page.waitForResponse((res) => {
    const url = new URL(res.url());
    return url.origin === apiOrigin && url.pathname === "/purchases" && res.request().method() === "GET";
  });
  await page.goto("/purchases");
  const purchasesRes = await purchases;
  expect(purchasesRes.status()).toBe(403);
  const purchasesBody: ErrorBody = await purchasesRes.json();
  expect(purchasesBody.error.code).toBe("forbidden");
  expect(purchasesBody.error.permission).toBe("see_cost_and_margin");

  // The same generic row again, this time naming the two permissions this
  // test asked to be refused by.
  const refusalLog = await request.get(
    `${apiUrl()}/audit-log?user_id=${cashierId}&action=permission.refused`,
    { headers: apiHeaders() },
  );
  expect(refusalLog.ok(), await refusalLog.text()).toBe(true);
  const refusals: { rows: { after: string | null }[] } = await refusalLog.json();
  const refused = refusals.rows
    .map((row) => (row.after === null ? null : (JSON.parse(row.after) as { permission: string })))
    .map((after) => after?.permission);
  expect(refused).toContain("see_reports");
  expect(refused).toContain("see_cost_and_margin");
});

test("a cashier reading the product list gets its cost fields redacted, not merely hidden from a screen", async ({
  context,
  request,
}) => {
  await signInOwner(request);
  const COST = 8_000; // 80,00
  const WHOLESALE = 15_000; // 150,00
  const productId = await seedProduct(request, {
    name: "Ciment Coût Caissier e2e",
    barcode: "6130009200059",
    price: 50_000,
    cost: COST,
    wholesale: WHOLESALE,
  });

  interface ProductRow {
    id: number;
    cost_centimes: number | null;
    wholesale_centimes: number | null;
  }

  // The owner reads the real figures back first: this is what locks down
  // that the fields below are redacted for a cashier, not simply absent
  // from the DTO for everyone (`routes/products.rs::redact_cost`, gated on
  // `SeeCostAndMargin` rather than a route-wide refusal, unlike purchases
  // and the dashboard above — the till itself reads this same route).
  const ownerRes = await request.get(`${apiUrl()}/products`, { headers: apiHeaders() });
  expect(ownerRes.ok(), await ownerRes.text()).toBe(true);
  const ownerProducts: ProductRow[] = await ownerRes.json();
  const ownerRow = ownerProducts.find((p) => p.id === productId);
  expect(ownerRow?.cost_centimes).toBe(COST);
  expect(ownerRow?.wholesale_centimes).toBe(WHOLESALE);

  await signInCashier(context.request);
  const cashierRes = await context.request.get(`${apiUrl()}/products`, { headers: apiHeaders() });
  // Not a refusal: a cashier rings this same catalogue up all day, so the
  // route answers 200 with the row still in it — redacted, not withheld.
  expect(cashierRes.status()).toBe(200);
  const cashierProducts: ProductRow[] = await cashierRes.json();
  const cashierRow = cashierProducts.find((p) => p.id === productId);
  expect(cashierRow).toBeDefined();
  expect(cashierRow?.cost_centimes).toBeNull();
  expect(cashierRow?.wholesale_centimes).toBeNull();
});
