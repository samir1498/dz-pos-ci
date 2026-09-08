---
title: 'M5 first release v1'
slug: 'm5-first-release-v1'
status: 'paused'
category: 'milestone'
created: 20260908
tldr: 'Signed installer, updater, versioned migrations, release gates; stub'
priority: 20
tasks: []
acceptance: []
---
# M5: first release, v1.0

Stub. Tasks get added when M4 closes. The milestone text is
`docs/roadmap.md` § M5; the release mechanics are `docs/architecture.md`
§ Release.

Demo that closes it: Anouar installs from a signed Windows installer, reads
version, git hash and build date in About, updates in place, and a first
shop runs on it.

In: the bundle identifier changed once with the final name; the Tauri
updater and its signing key; the Windows code-signing certificate;
migrations tied to the app version with an automatic backup before each; a
tag on `main` builds the installer and the GitHub release; the support
bundle; an Arabic and RTL polish pass; `bon_de_livraison`.

Release gates: every fiscal row confirmed by a comptable (R8); a real
printed facture seen (open decision 4); the Arabic words file reviewed by
a native speaker (R6); the NIF/AI article cited (R3); Sonar only if Rust
support was verified.

Blocked by: the final name and the certificate (Anouar).

