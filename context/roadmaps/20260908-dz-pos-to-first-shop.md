---
title: 'dz-pos to first shop'
slug: 'dz-pos-to-first-shop'
status: 'active'
category: 'roadmap'
created: 20260908
period: '2026-H2'
tldr: 'Seven milestones from the money core to a phone in the shop; full text in docs/roadmap.md'
priority: 90
entries:
  - ref: 'money-module-centimes-tva-stamp-amount-in-words'
    status: 'done'
    note: 'M0 closed 2026-09-09 with PR #14 merged (c3f6384)'
  - ref: 'legal-fiscal-and-tooling-research-for-dz-pos'
    status: 'in-progress'
    note: 'M0: R3, R6, R7, R8, R9 open; R8 and R6 gate M2, R3 gates M5'
  - ref: 'repo-tooling-skills-and-rules-for-dz-pos'
    status: 'in-progress'
    note: 'open: laptop clone, Sonar Rust check'
  - ref: 'm1-sale-and-ticket-on-one-desktop'
    status: 'done'
    note: 'M1 merged to main 2026-09-09 (PR #15); T6, the real printer run, waits for Samir at the laptop'
  - ref: 'm2-facture-customers-and-credit'
    status: 'done'
    note: 'M2 merged to main 2026-09-10 (PR #19); two refactors (T10, T11) run on the M3 branch; comptable questions open (R8)'
  - ref: 'm3-stock-in-expenses-reports'
    status: 'done'
    note: 'M3 built as a loop on 2026-09-10 (06:33 to 20:18) and merged to main in PR #20 on the local gates, CI being blocked by GitHub billing; ten tasks plus the design system and the nine screens on the kit'
  - ref: 'm4-team'
    status: 'in-progress'
    note: 'M4: ten tasks written 2026-09-11 (users and PINs, permissions, sessions, the M1-M3 carry-in gates, sign-in, what a cashier does not see, override permissions, audit log, users in settings, sweep); dz-review on every PR'
  - ref: 'm5-first-release-v1'
    status: 'planned'
    note: 'M5: blocked on the final name and the code-signing certificate (Anouar)'
  - ref: 'm6-phone-in-the-shop'
    status: 'planned'
    note: 'M6: Expo plugin and MCP trial start here'
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
