---
title: 'Security and provenance'
slug: 'security-and-provenance'
status: 'active'
category: 'processes'
created: 20260908
tldr: 'ISO 27001 controls every feature ships with; where a learned fact or a fiscal claim gets written'
---
# Security and provenance

## ISO 27001 (Anouar's directive, 2026-07-13)
Every feature ships with its controls, not as a later pass:
- least-privilege roles (owner, manager, cashier; the permission list is in
  `docs/features.md` §5)
- no credentials in code or in config committed to git
- audit log for sensitive actions: price change, discount override,
  delete, settings change, and every document records its user
- enforced retention and deletion
- a tested backup and restore path before cloud sync exists

## Provenance: claims need a home
- A fact learned about the repo or its tools (a crate quirk, a Cloudflare
  or Railway detail, a laptop path) goes in `context/references/`.
- A fiscal or business claim goes in `docs/features.md` next to its
  fixture name, marked assumption until a stamped facture or a comptable
  confirms it; then the row records who and when.
- A decision from Anouar or Samir goes in the plan or handoff it changed,
  with the date.
- Don't assert how the product behaves from memory. Open the file, quote
  the line. Personal `~/.claude` memory is not the team store; nothing on
  the laptop reads it.

