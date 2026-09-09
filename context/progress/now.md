---
type: 'now'
updated: '2026-09-09'
---
## Active
Roadmap `dz-pos-to-first-shop` (`just ctx roadmap show dz-pos-to-first-shop`
from the repo root, full text `docs/roadmap.md`): M0 closed on 2026-09-09
with PR #14 merged (c3f6384, two review rounds, gates and CI green). M1 is
active and runs as a loop (Samir, 2026-09-09 13:36): tasks T0 to T8 in
`plans/20260908-m1-sale-and-ticket-on-one-desktop.md`, run in sequence on
`m1/2026-09-09`, gates and a review before every merge, checkpoint PRs to
main at T1, T4 and T8 that Samir merges. T6 needs Samir at the laptop with
the printer. The ledger on the WSL box (`~/.dz-night/ledger.md`) holds the
ticks. M2 to M6 stay stub
plans until their turn.

Status site for Anouar: https://dinar-reports.pages.dev/ (progress reports
and reference pages; generator in `~/.dz-night/report/` on the WSL box).

Run `just status` from the repo root for the ladder and the plans.
Run `just ctx show <slug>` for plan details.

## Done recently
- 2026-09-09: M1 T3 (documents, gapless numbering, stock ledger, audit log, POST /sales; 8985d9d), T7 (three languages, RTL; 33ad612), T8 (daily backup, restore in place; dcf6f99), T4 (till screen as home; 415ade3) and T5 (80 mm ticket rendered by the core, nine goldens; 9cc9410) merged into `m1/2026-09-09` by the loop, each after dz-review lenses and a fix round. Docs sweep merged (d03ab61); code sweep on `m1/sweep-code` in flight. Checkpoint PR waits for `/dz-pr m1/2026-09-09`. T6 blocked on the printer.
- 2026-09-09: M1 T2 merged into `m1/2026-09-09` (settings screen: store block, dated régime on the shop's UTC+1 calendar; the review moved the seller snapshot and back-dating into T3). Stale-info sweep of the repo: context fixed on main, docs on `m1/stale-docs`; the generic `stale-check` skill is user-level, `.claude/stale-homes.md` holds where each fact lives.
- 2026-09-09: M1 T0 (launch token on every API route, transport-and-auth section in architecture.md) and T1 (products CRUD with edit, PUT /products/{id}) merged into `m1/2026-09-09`; checkpoint PR to main waits for `/dz-pr m1/2026-09-09`.
- 2026-09-09: PR #14 merged, M0 closed. Review of the branch found and fixed: a near-total discount printing a negative TVA base (share now capped at the group HT), the API panicking on i64::MIN, raw SQLite text reachable on the wire, the mockup writing the amount in words differently from the core. Coverage in CI scoped to the two crates with tests.

## Pending

## Not started

## M0 ladder (closed; kept as the record of what M0 was)

M0 ran step by step, Samir reviewing between steps. M1 runs as the loop
described under Active; its tasks and statuses are in the M1 plan, not
here.

1. [x] Stamp, TVA, rounding, facture mentions, numbering, IFU, words read from primary sources; all in `research/legal-fiscal/2026-09-08-fiscal-sources-and-findings.md` and the Source column of `docs/features.md` (R1, R2, R4, R5 done; R3, R6 partial)
2. [x] `Money` newtype and `pct` in `crates/core`, first fixtures, first proptest (money plan T1)
3. [x] TVA grouping per rate and global discount spread; fixtures shared with `design/shared/money.js` via vitest (T2, T5)
4. [x] Stamp duty per step 1 (T3, T8)
5. [x] Amount in words fr/ar/en with golden files (T4, R6)
6. [x] First migration (`shop_id`, products with a price model for both regimes, settings with the dated régime fiscal, one seeded owner user), `crates/api` with GET/POST /products, `packages/shared` with the ts-rs types, and the products screen reading it, shown on the laptop; the `webapp-testing` and `frontend-design` trial (R9) runs on that screen

## From Anouar, 2026-09-08 (Discord), to lock after the steps

- Product name proposal: "Dinar".
- GitHub organisation created; Samir invited. Repo transfer + bundle
  identifier change when the name is final.
- Anouar worked on a similar product before: the law and the calculations
  are not simple; an accountant may be needed as the project grows (R8).

## Decisions from Samir, 2026-09-08 (evening)

- E2E: ObserveOne is the e2e tool for this product; outside skills only
  if they prove better.
- Tauri dev loop: web UI on the WSL box (today `just api` then `just dev`); the laptop
  and the native window only when a feature is ready for a full-app pass.
- Versioning from the first release: semver + git short hash + build date,
  all three embedded in the binary and shown in About.
- Payment modes v1: cash, credit (customer ledger), card on a TPE with no
  integration; transfer and cheque later.
- Milestone order (later that evening): a cash sale with a printed ticket
  ships before customers and debt; sequence and exit criteria, no dates.
  `docs/roadmap.md`.
