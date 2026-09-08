---
type: 'now'
updated: '2026-09-08'
---
## Active
Roadmap `dz-pos-to-first-shop` (`just ctx roadmap show dz-pos-to-first-shop`
from the repo root, full text `docs/roadmap.md`): M0 in flight, the ladder
below is its steps. M1 to M6 are paused stub plans that receive tasks when
their milestone starts.

Run `just status` from the repo root for the ladder and the plans.
Run `just ctx show <slug>` for plan details.

## Done recently

## Pending

## Not started

## Ladder (step-by-step)

Samir reviews between steps; one step in flight at a time.

1. [x] Stamp, TVA, rounding, facture mentions, numbering, IFU, words read from primary sources; all in `research/legal-fiscal/2026-09-08-fiscal-sources-and-findings.md` and the Source column of `docs/features.md` (R1, R2, R4, R5 done; R3, R6 partial)
2. [ ] `Money` newtype and `pct` in `crates/core`, first fixtures, first proptest (money plan T1)
3. [ ] TVA grouping per rate and global discount spread; fixtures shared with `design/shared/money.js` via vitest (T2, T5)
4. [ ] Stamp duty per step 1 (T3, T8)
5. [ ] Amount in words fr/ar/en with golden files (T4, R6)
6. [ ] First migration (`shop_id`, products with a price model for both regimes, settings with the dated régime fiscal, one seeded owner user), `crates/api` with GET/POST /products, `packages/shared` with the ts-rs types, and the products screen reading it, shown on the laptop

## From Anouar, 2026-09-08 (Discord), to lock after the steps

- Product name proposal: "Dinar".
- GitHub organisation created; Samir invited. Repo transfer + bundle
  identifier change when the name is final.
- Anouar worked on a similar product before: the law and the calculations
  are not simple; an accountant may be needed as the project grows (R8).

## Decisions from Samir, 2026-09-08 (evening)

- E2E: ObserveOne is the e2e tool for this product; outside skills only
  if they prove better.
- Tauri dev loop: web UI on the WSL box (`pnpm desktop dev`); the laptop
  and the native window only when a feature is ready for a full-app pass.
- Versioning from the first release: semver + git short hash + build date,
  all three embedded in the binary and shown in About.
- Payment modes v1: cash, credit (customer ledger), card on a TPE with no
  integration; transfer and cheque later.
- Milestone order (later that evening): a cash sale with a printed ticket
  ships before customers and debt; sequence and exit criteria, no dates.
  `docs/roadmap.md`.
