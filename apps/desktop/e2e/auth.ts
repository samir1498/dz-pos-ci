// The one door every spec now has to get through. M4 T4 puts a sign-in
// screen in front of the whole app, so a bare `page.goto` that used to land
// straight on a route now lands on the PIN pad instead.
//
// What the sign-in screen itself does, the PIN pad's two stages, the wrong
// answer and its counted-down wait, the password form, is exercised for
// real in `signin.spec.ts` and `till-lock.spec.ts`, through the actual keys and
// the actual fields. Every other spec only wants to be past the door, so
// this signs in once through the API, before the browser context's first
// navigation, and every `goto` and `reload` after that carries the session
// the way any cookie-backed site's does: a browser attaches a `SameSite=Lax`
// cookie to a same-site request whatever the port, and the API and Vite
// both bind 127.0.0.1 (playwright.config.ts), so the two are cross-origin
// but same-site and the cookie rides along `context.request`'s jar onto
// `page`, which shares it.
//
// The credential itself is `globalSetup.ts`'s job: a fresh e2e database's
// seeded owner carries no usable one until that has run.
//
// Two fixtures, not one. `page` sits on `context`, and every screen's own
// `fetch` carries the cookie that landed on it; but a spec that seeds a
// product or a customer straight through the API (`seed(request, ...)` in
// till.spec.ts and the like) does it with the standalone `request` fixture,
// which Playwright gives its own `APIRequestContext` and its own cookie jar,
// unrelated to `context`'s. Both are now behind the session guard T2 put on
// every route but the auth ones (`crates/api/src/session.rs::require`), so
// both are signed in here; a spec that never destructures `request` never
// pays for the second login, fixtures only run when a test asks for them.

import type { APIRequestContext } from "@playwright/test";
import { expect, test as base } from "@playwright/test";

import { apiHeaders, apiUrl } from "./api";

/** The seeded owner's name (`crates/core/migrations/.../000000_init`), and
 * the PIN and password `globalSetup.ts` gives that row: the same two
 * figures `dzpos-seed` prints on every developer's machine
 * (`crates/core/src/services/seed.rs::OWNER_PIN` / `OWNER_PASSWORD`). Not a
 * secret: a throwaway database this suite deletes at the start of every
 * run. */
export const OWNER_NAME = "Propriétaire";
export const OWNER_PIN = "1379";
export const OWNER_PASSWORD = "developpement";

async function signIn(api: APIRequestContext): Promise<void> {
  const res = await api.post(`${apiUrl()}/auth/login`, {
    headers: apiHeaders(),
    data: { name: OWNER_NAME, password: OWNER_PASSWORD },
  });
  if (!res.ok()) {
    throw new Error(
      `e2e/auth.ts: the fixed owner credential was refused (${res.status()} ${await res.text()}). ` +
        "Did globalSetup.ts run, and does its hash still match OWNER_PASSWORD above?",
    );
  }
}

export const test = base.extend({
  context: async ({ context }, use) => {
    await signIn(context.request);
    await use(context);
  },
  request: async ({ request }, use) => {
    await signIn(request);
    await use(request);
  },
});

export { expect };
