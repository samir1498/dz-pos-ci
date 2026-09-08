---
type: 'now'
updated: '2026-09-08'
---
## Active
Run `bun run ctx status` for full overview.
Run `bun run ctx show <slug>` for plan details.

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
6. [ ] `crates/api` with GET/POST /products and the products screen reading it, shown on the laptop

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
