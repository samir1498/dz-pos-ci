// The till shift bar and the manager's list, driven by real cashiers in the
// browser (plan till-shifts-a-float-and-a-count T7). `till-cashier.spec.ts`
// beside this one is where the shape below comes from: `./auth`'s `test`
// signs `context`/`request` in as the owner before either fixture is ready,
// and this file needs three other identities, so it stays on plain
// `@playwright/test` and signs each one in itself, in the order a same-site
// cookie has to be on the jar before the first `page.goto`.
//
// Three fiches, made once in `beforeAll` over a standalone
// `APIRequestContext`: a cashier whose own drawer this file opens and
// closes across several tests in a row (`fullyParallel: false`, one worker,
// so the file's own order is the run's order — the same guarantee
// `till-cashier.spec.ts`'s credit test relies on for its own two-step
// flow), a second cashier who exists only to be refused another person's
// drawer, and a manager who reads the list back. `beforeAll` also closes
// any shift either cashier is left holding from a run that failed midway:
// a fiche and its PIN survive a rerun (`services::users::refuse_taken_name`
// is what makes `makeOrFindFiche` idempotent), but an open drawer is state
// too, and the first test in this file assumes there is none.

import { test, expect, request as pwRequest } from "@playwright/test";
import type { APIRequestContext } from "@playwright/test";
import { apiHeaders, apiUrl } from "./api";
import { OWNER_NAME, OWNER_PASSWORD } from "./auth";
import { t } from "./messages";

const CASHIER_NAME = "Caissière shifts e2e";
const CASHIER_PIN = "7146";
const SECOND_CASHIER_NAME = "Caissière shifts e2e deux";
const SECOND_CASHIER_PIN = "5137";
const MANAGER_NAME = "Manager shifts e2e";
const MANAGER_PIN = "8215";

let cashierId: number;
let secondCashierId: number;
let managerId: number;

interface ErrorBody {
  error: {
    code: string;
    message: string;
    permission?: string;
    field?: string;
  };
}

async function signInOwner(api: APIRequestContext): Promise<void> {
  const res = await api.post(`${apiUrl()}/auth/login`, {
    headers: apiHeaders(),
    data: { name: OWNER_NAME, password: OWNER_PASSWORD },
  });
  if (!res.ok()) {
    throw new Error(`till-shifts.spec.ts: owner sign-in refused (${res.status()} ${await res.text()})`);
  }
}

async function signInAs(api: APIRequestContext, userId: number, pin: string): Promise<void> {
  const res = await api.post(`${apiUrl()}/auth/login`, {
    headers: apiHeaders(),
    data: { user_id: userId, pin },
  });
  if (!res.ok()) {
    throw new Error(`till-shifts.spec.ts: sign-in refused for user ${userId} (${res.status()} ${await res.text()})`);
  }
}

/** A fiche of `role`, made once. Idempotent the way `till-cashier.spec.ts`'s
 * own `makeOrFindCashier` is: a fresh Playwright worker after a mid-file
 * failure reruns `beforeAll`, and a second attempt at the same name is a
 * `conflict` this looks the existing fiche up under rather than treating as
 * a setup failure. */
async function makeOrFindFiche(owner: APIRequestContext, name: string, role: string): Promise<number> {
  const created = await owner.post(`${apiUrl()}/users`, {
    headers: apiHeaders(),
    data: { name, role },
  });
  if (created.status() === 201) {
    const user: { id: number } = await created.json();
    return user.id;
  }
  if (created.status() !== 409) {
    throw new Error(`till-shifts.spec.ts: could not create "${name}" (${created.status()} ${await created.text()})`);
  }
  const list = await owner.get(`${apiUrl()}/users`, { headers: apiHeaders() });
  if (!list.ok()) {
    throw new Error(`till-shifts.spec.ts: could not list users to find "${name}" (${list.status()} ${await list.text()})`);
  }
  const users: { id: number; name: string }[] = await list.json();
  const found = users.find((u) => u.name === name);
  if (found === undefined) {
    throw new Error(`till-shifts.spec.ts: "${name}" was refused as taken but is not in the user list`);
  }
  return found.id;
}

async function setPin(owner: APIRequestContext, userId: number, pin: string): Promise<void> {
  const res = await owner.post(`${apiUrl()}/users/${userId}/pin`, {
    headers: apiHeaders(),
    data: { pin },
  });
  expect(res.ok(), await res.text()).toBe(true);
}

/** Closes whatever drawer `userId` is holding, at the exact figure the
 * server expects, so an equal count needs no note. Called from `beforeAll`
 * so a rerun after a mid-file failure starts from the same "nobody has a
 * drawer open" state the first test in this file assumes. */
async function closeIfOpen(userId: number, pin: string): Promise<void> {
  const fresh = await pwRequest.newContext();
  try {
    await signInAs(fresh, userId, pin);
    const open = await fresh.get(`${apiUrl()}/till/shifts/open`, { headers: apiHeaders() });
    expect(open.ok(), await open.text()).toBe(true);
    const report: { shift: { id: number }; expected_centimes: number } | null = await open.json();
    if (report === null) return;
    const closed = await fresh.post(`${apiUrl()}/till/shifts/${report.shift.id}/close`, {
      headers: apiHeaders(),
      data: { counted_centimes: report.expected_centimes, note: null },
    });
    expect(closed.ok(), await closed.text()).toBe(true);
  } finally {
    await fresh.dispose();
  }
}

test.beforeAll(async () => {
  const owner = await pwRequest.newContext();
  try {
    await signInOwner(owner);
    cashierId = await makeOrFindFiche(owner, CASHIER_NAME, "cashier");
    await setPin(owner, cashierId, CASHIER_PIN);
    secondCashierId = await makeOrFindFiche(owner, SECOND_CASHIER_NAME, "cashier");
    await setPin(owner, secondCashierId, SECOND_CASHIER_PIN);
    managerId = await makeOrFindFiche(owner, MANAGER_NAME, "manager");
    await setPin(owner, managerId, MANAGER_PIN);
    await closeIfOpen(cashierId, CASHIER_PIN);
    await closeIfOpen(secondCashierId, SECOND_CASHIER_PIN);
  } finally {
    await owner.dispose();
  }
});

test("the first sign-in raises the open popup and a real POST opens the shift", async ({
  page,
  context,
}) => {
  await signInAs(context.request, cashierId, CASHIER_PIN);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  // Ruling 2b: the first sign-in with no drawer open raises the popup on its
  // own, with no button clicked to ask for it.
  await expect(page.getByTestId("till-open-dialog")).toBeVisible();
  await page.getByTestId("till-open-amount").fill("5000");

  const opened = page.waitForResponse(
    (res) => res.url().endsWith("/till/shifts") && res.request().method() === "POST",
  );
  await page.getByTestId("till-open-submit").click();
  const response = await opened;
  expect(response.status(), await response.text()).toBe(201);
  const shift: { opening_cash_centimes: number } = await response.json();
  expect(shift.opening_cash_centimes).toBe(500_000);

  await expect(page.getByTestId("till-shift-badge")).toBeVisible();
});

test("the cashier closes with an exact count and no note is asked", async ({ page, context }) => {
  await signInAs(context.request, cashierId, CASHIER_PIN);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  // A drawer is already open from the previous test, so the popup does not
  // raise itself again: only the badge and the trigger to close it.
  await expect(page.getByTestId("till-shift-badge")).toBeVisible();
  await page.getByTestId("till-close-trigger").click();
  await expect(page.getByTestId("till-close-dialog")).toBeVisible();

  // No sale was rung, so the expected figure is exactly the float the
  // previous test opened with.
  await page.getByTestId("till-counted").fill("5000");
  expect(await page.getByTestId("till-close-note").count()).toBe(0);

  const closed = page.waitForResponse(
    (res) => /\/till\/shifts\/\d+\/close$/.test(res.url()) && res.request().method() === "POST",
  );
  await page.getByTestId("till-close-submit").click();
  const response = await closed;
  expect(response.status(), await response.text()).toBe(200);
  const shift: { difference_centimes: number | null } = await response.json();
  expect(shift.difference_centimes).toBe(0);

  await expect(page.getByTestId("till-closed-banner")).toBeVisible();
});

test("the cashier closes short and the note is required, both on the screen and at the server", async ({
  page,
  context,
}) => {
  await signInAs(context.request, cashierId, CASHIER_PIN);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  await expect(page.getByTestId("till-open-dialog")).toBeVisible();
  await page.getByTestId("till-open-amount").fill("3000");
  const opened = page.waitForResponse(
    (res) => res.url().endsWith("/till/shifts") && res.request().method() === "POST",
  );
  await page.getByTestId("till-open-submit").click();
  const openedResponse = await opened;
  const shift: { id: number } = await openedResponse.json();

  await page.getByTestId("till-close-trigger").click();
  await page.getByTestId("till-counted").fill("2500");

  // The screen's own guard: the note field appears, names the reason it is
  // asked for, and the submit button will not take the short count without
  // one — checked before the request that would refuse it for the same
  // reason ever leaves the browser.
  await expect(page.getByTestId("till-close-note")).toBeVisible();
  await expect(
    page.getByText(t("error_shift_note_required")),
  ).toBeVisible();
  await expect(page.getByTestId("till-close-submit")).toBeDisabled();

  // The server's own refusal, asked for directly: `services::shifts::close`
  // rather than the dialog merely staying open. The M4 rule is that a spec
  // names the refusal code apart, so the field and the exact wording are
  // both asserted, not only the status.
  const bypassed = await context.request.post(`${apiUrl()}/till/shifts/${shift.id}/close`, {
    headers: apiHeaders(),
    data: { counted_centimes: 250_000 },
  });
  expect(bypassed.status()).toBe(422);
  const bypassedBody: ErrorBody = await bypassed.json();
  expect(bypassedBody.error.code).toBe("validation");
  expect(bypassedBody.error.field).toBe("note");
  // The wire shape for every CoreError::Validation is "{field} is invalid:
  // {message}" (crates/api/src/error.rs, ApiError::message), so the field
  // name travels twice: once structured, once inside the sentence.
  expect(bypassedBody.error.message).toBe(
    "note is invalid: a drawer that does not match what was expected needs a reason",
  );

  // And the ordinary way through: a note unblocks the same submit.
  await page.getByTestId("till-close-note").fill("Écart de caisse constaté");
  await expect(page.getByTestId("till-close-submit")).toBeEnabled();
  const closed = page.waitForResponse(
    (res) => /\/till\/shifts\/\d+\/close$/.test(res.url()) && res.request().method() === "POST",
  );
  await page.getByTestId("till-close-submit").click();
  const closedResponse = await closed;
  expect(closedResponse.status(), await closedResponse.text()).toBe(200);
  const closedShift: { difference_centimes: number | null } = await closedResponse.json();
  expect(closedShift.difference_centimes).toBe(-50_000);
  await expect(page.getByTestId("till-closed-banner")).toBeVisible();
});

test("a cashier trying to close another person's shift is refused, naming the permission", async ({
  context,
  request,
}) => {
  await signInAs(context.request, cashierId, CASHIER_PIN);
  const opened = await context.request.post(`${apiUrl()}/till/shifts`, {
    headers: apiHeaders(),
    data: { opening_cash_centimes: 100_000 },
  });
  expect(opened.status(), await opened.text()).toBe(201);
  const shift: { id: number } = await opened.json();

  const second = await pwRequest.newContext();
  try {
    await signInAs(second, secondCashierId, SECOND_CASHIER_PIN);
    const refused = await second.post(`${apiUrl()}/till/shifts/${shift.id}/close`, {
      headers: apiHeaders(),
      data: { counted_centimes: 100_000 },
    });
    expect(refused.status()).toBe(403);
    const body: ErrorBody = await refused.json();
    expect(body.error.code).toBe("forbidden");
    expect(body.error.permission).toBe("close_another_persons_till");
  } finally {
    await second.dispose();
  }

  // Left open on purpose: the cashier's own drawer, refused to somebody
  // else, is still theirs to count. Closed here through the owner's session
  // so the next test's read of the list finds a real closed row rather than
  // an open one this file forgot to finish.
  await signInOwner(request);
  const closed = await request.post(`${apiUrl()}/till/shifts/${shift.id}/close`, {
    headers: apiHeaders(),
    data: { counted_centimes: 100_000 },
  });
  expect(closed.status(), await closed.text()).toBe(200);
});

test("the manager reads the closed shifts back and the cashier is refused the list", async ({
  page,
  context,
}) => {
  await signInAs(context.request, managerId, MANAGER_PIN);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);
  // The screen-first check, named as not being the defence: the entry a
  // manager sees is there, and the server behind it is what actually
  // decides (checked on the cashier below).
  await expect(page.getByTestId("nav-till/shifts")).toBeVisible();

  // The exact path, not a prefix: `TillShiftBar` on every screen fires
  // `GET /till/shifts/open` on mount, which a bare `startsWith` would also
  // catch — the same trap `till-cashier.spec.ts`'s own dashboard test names.
  const apiOrigin = new URL(apiUrl()).origin;
  const list = page.waitForResponse((res) => {
    const url = new URL(res.url());
    return url.origin === apiOrigin && url.pathname === "/till/shifts" && res.request().method() === "GET";
  });
  await page.goto("/till/shifts");
  const listRes = await list;
  expect(listRes.status(), await listRes.text()).toBe(200);
  const rows: { id: number; opened_by: number; difference_centimes: number | null }[] =
    await listRes.json();
  // The row the earlier test closed 50 000 short is in the day list with
  // that figure, and the screen draws it: a 200 with an empty or garbled
  // table would otherwise pass here.
  const short = rows.find((row) => row.opened_by === cashierId && row.difference_centimes === -50_000);
  expect(short, JSON.stringify(rows)).toBeDefined();
  await expect(page.getByTestId(`till-shifts-difference-${short?.id}`)).toBeVisible();

  await signInAs(context.request, cashierId, CASHIER_PIN);
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);
  await expect(page.getByTestId("nav-till")).toBeVisible();
  await expect(page.getByTestId("nav-till/shifts")).toHaveCount(0);

  const refused = page.waitForResponse((res) => {
    const url = new URL(res.url());
    return url.origin === apiOrigin && url.pathname === "/till/shifts" && res.request().method() === "GET";
  });
  await page.goto("/till/shifts");
  const refusedRes = await refused;
  expect(refusedRes.status()).toBe(403);
  const body: ErrorBody = await refusedRes.json();
  expect(body.error.code).toBe("forbidden");
  expect(body.error.permission).toBe("see_reports");
  await expect(page.getByRole("alert").filter({ hasText: t("error_forbidden") })).toBeVisible();
});
