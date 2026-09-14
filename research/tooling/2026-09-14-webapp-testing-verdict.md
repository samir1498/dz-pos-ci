# webapp-testing verdict — 2026-09-14

Trial: compared `anthropics/webapp-testing` (Playwright discipline: page objects, a11y locators, visual diffs, `test --ui`) against the product's own `apps/desktop/e2e` suite driven by `just e2e` (fresh API + DB, fr/en/ar, `till-cashier.spec.ts`, `till-facture.spec.ts`, `audit.spec.ts`, screenshots under `e2e/screenshots/*.png` for the landing page).

What it does: scaffolds `playwright.config.ts`, `tests/`, `fixtures`, and a `test --ui` loop, with guidance on locators and a11y.

Verdict: **not adopted; keep the ObserveOne e2e pattern already in repo.**

- `webapp-testing` and `dz-mockup`'s drive scripts do the same Playwright work, but the product's suite already has the shop-specific harness: `just api` (launch token), `just dev` (5173), `just e2e` (seeds, three locales sequentially on one port pair), and the screenshot contract the landing page reads (`apps/landing/src/lib/shots.ts`). Scaffolding a second harness would duplicate it.
- The skill's value is the discipline (one assertion per test, `getByRole`, `expect(page).toHaveScreenshot()`), which the existing suite already follows (e.g., `e2e/README.md` files list, `till-cashier.spec.ts` proves every permission refusal apart from status alone). No new dependency.
- `pdf` from the same marketplace remains useful for research sessions (DGI PDFs), but `webapp-testing` stays uninstalled.

If a new device flow (M6 phone on LAN) needs a separate Playwright project, re-evaluate the skill's `test --ui` and trace viewer on that flow.
