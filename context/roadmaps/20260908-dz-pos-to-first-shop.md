---
title: 'dz-pos to first shop'
slug: 'dz-pos-to-first-shop'
status: 'active'
category: 'roadmap'
created: 20260908
period: '2026-H2'
tldr: 'Eight milestones from the money core to a paired phone proven in the shop; full text in docs/roadmap.md'
priority: 90
entries:
  - ref: 'money-module-centimes-tva-stamp-amount-in-words'
    status: 'done'
    note: 'M0 closed 2026-09-09 with PR #14 merged (c3f6384)'
  - ref: 'legal-fiscal-and-tooling-research-for-dz-pos'
    status: 'in-progress'
    note: 'M0: R6, R8, R9 open; R3 and R7 done; R8 and R6 gate M2'
  - ref: 'repo-tooling-skills-and-rules-for-dz-pos'
    status: 'done'
    note: 'closed 2026-09-21: the laptop clone exists and Sonar runs its Rust check'
  - ref: 'm1-sale-and-ticket-on-one-desktop'
    status: 'done'
    note: 'M1 merged to main 2026-09-09 (PR #15); T6, the real printer run, was cancelled on 2026-09-13 because no printer exists to run it against'
  - ref: 'm2-facture-customers-and-credit'
    status: 'done'
    note: 'M2 merged to main 2026-09-10 (PR #19); two refactors (T10, T11) run on the M3 branch; comptable questions open (R8)'
  - ref: 'm3-stock-in-expenses-reports'
    status: 'done'
    note: 'M3 built as a loop on 2026-09-10 (06:33 to 20:18) and merged to main in PR #20 on the local gates, CI being blocked by GitHub billing; ten tasks plus the design system and the nine screens on the kit'
  - ref: 'm4-team'
    status: 'done'
    note: 'M4 built and merged to main on 2026-09-11 (PR #33, cf5236a), checked on the mirror because the org Actions budget is capped; ten tasks, each reviewed on its own, then a whole-milestone review and a second round over its fixes. The three findings that shaped it: two permissions enforced by nothing, five reads carrying the shop money left open after the sweep meant to close them, and a refusal that rolls back leaving no row. A cashier drives the browser suite now.'
  - ref: 'm5-first-release-v1'
    status: 'done'
    note: 'M5 closed 2026-09-13 (PRs #68-#74: matrix nsis/dmg/appimage, org publish via ORG_RELEASE_TOKEN, thermal ESC/POS route + ISO 8859-15 fix, review gaps, R9 verdicts, Dinar rename com.dinar.app). Cert and updater key stay with Samir as open holds.'
  - ref: 'm6-phone-in-the-shop'
    status: 'done'
    note: 'M6 closed 2026-09-15 (PR #83, c7ad157, 9/9): LAN+mdns, QR 60s single-use, Expo scaffold, list/revoke, thin till+queue, print spool, Maestro pair-and-sell, password reset, closing sweep. Full CI green on mirror before merge. The three unknowns it left (queue idempotency, device middleware, QR TOCTOU) are closed in M7.'
  - ref: 'm7-paired-phone-proven'
    status: 'done'
    note: 'M7 closed 2026-09-15 (PRs #89-#92, loop 20260915-m7-loop): atomic QR claim (BEGIN IMMEDIATE), device gate (X-Dzpos-Device off-loopback), phone session (device-gated login), sale idempotency (fingerprint dedupe + queue session binding), mobile auth cleanup, Maestro pair-and-sell on a real phone over Tailscale, closing sweep. Loop is done, manual test green.'
---
# dz-pos: from the money core to a first shop

The full text is `docs/roadmap.md` in the repo; this entry tracks which
milestone is in flight. Order (Samir, 2026-09-08): M0 money core and the
first real screen; M1 a cash sale with a printed ticket; M2 facture,
customers and credit; M3 stock in, expenses, reports; M4 team; M5 first
release; M6 the phone in the shop. Cloud mode waits on open decision 1.
No dates; a milestone closes when its demo runs on a real machine.

M0 was the three existing plans (money module, legal and tooling research,
repo tooling). M1 carries its tasks; M2 to M6 are stub plans that receive
tasks when their milestone starts.
