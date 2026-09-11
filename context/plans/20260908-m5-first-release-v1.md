---
title: 'M5 first release v1'
slug: 'm5-first-release-v1'
status: 'active'
category: 'milestone'
created: 20260908
tldr: 'Signed installer, updater, versioned migrations, release gates: ten tasks, written 2026-09-11 when M4 landed'
priority: 20
tasks:
  - id: 'T0'
    desc: 'A content security policy on the webview, and the launch token out of the page global it sits in today. `tauri.conf.json` carries `csp: null`, so one link that navigated the main frame to a remote page would hand the token over; no screen has such a link and the policy is what keeps it so. Written first because it is the only release item that is a hole rather than a piece of machinery, and because it is provable here. A test that the built page carries the header, and a test that a navigation away from the app origin is refused.'
    status: 'done'
  - id: 'T1'
    desc: 'Version, git short hash and build date embedded in the binary at build time and read back from one place: About shows all three, the log file heads every session with them, and the support bundle carries them. Answers the first of the release questions in `docs/architecture.md` § Release. A test that the three are present and not placeholders, and one that a debug build says so rather than pretending to be a release.'
    status: 'done'
  - id: 'T2'
    desc: 'Migrations tied to the app version, forward only, with an automatic backup of the SQLite file before any migration runs and a refusal to start if the backup could not be written. A previous-version open test: a file written by the last tag opens under the current build, migrates, and its documents and ledger read back identical. This is the task that decides whether an update can lose a shop its books, so it is the one that gets the deepest review.'
    status: 'done'
  - id: 'T3'
    desc: 'The support bundle: one command from settings that writes a zip a shop can send, carrying the log, the versions from T1, the migration history and the schema, and carrying no customer names, no prices and no credential. A test that lists what went in and refuses anything outside the list, because a support bundle that leaks a customer list is worse than no support bundle.'
    status: 'pending'
  - id: 'T4'
    desc: 'The five release questions in `docs/architecture.md` § Release answered in the page itself rather than left as questions, and the release gate list turned into a checklist that says the state of each gate and who holds it. Five of the gates are Samir''s and Anouar''s rather than mine (the comptable''s confirmation of every fiscal row, a real printed facture seen, a native speaker reading the Arabic, the article citation for NIF and AI on a facture, and Sonar only if Rust support was verified), so what this task produces is the page that says which of them are answered and which are waiting, not the answers.'
    status: 'pending'
  - id: 'T5'
    desc: 'The Arabic and RTL polish pass over every screen and every printed document: mirrored layouts, the numerals the spec asks for, dates and money reading right, the lock and sign-in screens included because they were the last built. What can be proven here is that nothing is clipped, reversed or left in French; what cannot is whether the wording is right, which is release gate R6 and needs a native speaker.'
    status: 'pending'
  - id: 'T6'
    desc: 'A tag on `main` is the only thing that builds the installer and publishes the GitHub release. Buildable here and not provable here: the Windows job runs on the organisation''s Actions minutes, which are capped, so it is exercised on the mirror and its first real run is the first tag. The workflow refuses to publish from anything but a tag on main, and says so in its own output rather than silently doing nothing.'
    status: 'done'
  - id: 'T7'
    desc: 'The Tauri updater: the endpoint, the signature check, the update flow the shop sees, and the key generated outside the repo with CI signing from a secret. The client half is provable here against a fake endpoint serving a signed and an unsigned manifest, and the test that matters is the second one: an unsigned or wrongly signed update is refused. Who holds the key is Anouar''s and Samir''s to decide before the key exists.'
    status: 'pending'
  - id: 'T8'
    desc: 'The name, once. The bundle identifier `com.dzpos.app`, the product name in the installer and the window title, the landing page''s copy and its sharing card, and the placeholder in the docs, all changed in one commit so the repo never half-carries two names. Blocked on Anouar; everything else in this milestone can be built before it arrives, and this task is deliberately last so it is a rename and not a rewrite.'
    status: 'pending'
  - id: 'T9'
    desc: 'Closing sweep: `docs/features.md` and `docs/architecture.md` rewritten to what shipped, the release gate checklist from T4 brought up to date with whatever moved while the milestone ran, the `dz-review` pass over the whole milestone with the money and deletion lenses deepest because an update that migrates a shop''s file touches both, the boss page, the checkpoint PR.'
    status: 'pending'
  - id: 'T10'
    desc: 'A shopkeeper sees why the app would not start. A Windows release build has no console, so the message that explains a refusal goes nowhere: the window never opens and nothing is said. That was already true of a failed migration and is now much easier to hit, because the pre-upgrade copy refuses to start on a disk that is full. A native message box before the window exists, naming the folder and what could not be written, and the same for a migration that fails. Raised by the data-safety review of the pre-upgrade copy, 2026-09-11.'
    status: 'pending'
  - id: 'T11'
    desc: 'The two kinds of copy that sit beside the shop file, the one taken before a restore and the one taken before an upgrade, are restorable from the settings screen. Today the restore route accepts the name of a daily copy and no other kind, so both are a file swap by hand. Needs a third list on the backups route, wording in three languages and a screen change. Also: the reopen inside a restore migrates an older copy in place without taking a pre-upgrade copy of it, which is a one-line change to a nine-step sequence and deserves its own review. Raised by the data-safety review, 2026-09-11.'
    status: 'pending'
acceptance: []
---
# M5: first release, v1.0

Ten tasks, written on 2026-09-11 when M4 landed on main. The milestone text
is `docs/roadmap.md` § M5 and the release mechanics are
`docs/architecture.md` § Release.

This milestone is different from the four before it, and the difference is
worth saying before the tasks: most of what it builds cannot be demonstrated
from this machine. An installer needs Windows, a signature needs a
certificate that has not been bought, and an update needs a previous release
to update from. So each task below says what can be proven here and what
cannot, and the ones that cannot are built against a stand-in and proven the
day the real thing exists. A task that says it is done and means it compiles
is not done.

T0 to T5 are unblocked and can be built now. T6 and T7 are buildable now and
provable later. T8 waits on the name. Nothing waits on T8 except T8.

Demo that closes it: Anouar installs from a signed Windows installer, reads
version, git hash and build date in About, updates in place, and a first
shop runs on it.

In: the bundle identifier changed once with the final name; the Tauri
updater and its signing key; the Windows code-signing certificate;
migrations tied to the app version with an automatic backup before each; a
tag on `main` builds the installer and the GitHub release; the support
bundle; an Arabic and RTL polish pass.

Not in, though the roadmap said so until 2026-09-11: the bon de livraison.
`docs/features.md` § Later parks it, and for a fiscal reason rather than an
effort one. Décret 05-468 articles 14 to 17 allow a delivery note only
together with a facture récapitulative and a wilaya authorisation, so a shop
issuing one on its own would be issuing a document it may not. The kind
stays in the model with no template and no screen, which is what the code
already does. The two pages disagreed for three days; the spec is the one
that holds and the roadmap now points at it.

Release gates: every fiscal row confirmed by a comptable (R8); a real
printed facture seen (open decision 4); the Arabic words file reviewed by
a native speaker (R6); the NIF/AI article cited (R3); Sonar only if Rust
support was verified.

Blocked by: the final name and the certificate (Anouar).

