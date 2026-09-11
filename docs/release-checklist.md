# What has to be true before v1.0 ships

Every gate and every open question for the first release, each with its
state, who holds it, and where its answer gets written when it arrives. The
milestone is `docs/roadmap.md` § M5 and the mechanics are
`docs/architecture.md` § Release; this page is the one that says what is
still missing and from whom.

Read the states as: **done** means it shipped and something holds it: a
test for code, a quoted article for a claim about the law.
**waiting** means the answer is somebody else's and the work cannot start
without it. **open** means it is ours and not started.

## Built and holding

| What | Where it lives | Held by |
|---|---|---|
| The version, the commit and the build date, from one place, on the About screen and at the head of the log file | `crates/core/src/build_info.rs`, `crates/api` route `/build-info` | a test that fails if a second spelling of any of the three appears |
| A copy of the shop's file before a new build migrates it, and a refusal to start if the copy cannot be written | `crates/core/src/services/backup.rs::before_upgrade`, `crates/api/src/lib.rs::open_and_upgrade` | `crates/api/tests/upgrade_from_a_previous_version.rs`, five tests |
| A content security policy on the window and the launch token out of the page global | `apps/desktop/src-tauri/`, `tauri.conf.json` | a test that reads the policy off the real built page, and one that a navigation away is refused |
| A tag on `main` as the only thing that publishes a release | `.github/workflows/release.yml`, `.github/scripts/release-gate.sh` | fourteen cases in that script's own test |
| The text that puts the NIF on a facture, and the absence of one for the article d'imposition | `docs/features.md` party identifiers row, from `research/legal-fiscal/2026-09-08-facture-and-ticket.md` | loi 04-02 art. 34 and LF 2006 art. 42, quoted from the Journal Officiel PDFs in that note's `sources/` |

## Waiting on somebody

| What has to be true | Who holds it | Why it cannot start without them | Where the answer goes |
|---|---|---|---|
| The product has its final name | Anouar | The name is baked into the installer and the bundle identifier. Changing it after the first release means every shop reinstalls rather than updates, so it has to be right before the first tag. | `features.md` open decision 2, then one commit that renames everything at once |
| A Windows code-signing certificate is bought | Anouar | Without one, every customer who installs sees a warning that the software is from an unknown publisher. Buying one takes days, which is why it sits at the top of this list rather than at the end. | the release workflow's secrets; the workflow already builds unsigned and marks the release a draft when the secret is absent |
| Somebody holds the key that signs updates | Anouar and Samir | The key never enters the repository and the build signs with a secret. Until it exists there is no update to sign, and until somebody owns it there is nobody to regenerate it if it leaks. | `docs/architecture.md` § Release |
| A comptable has confirmed every fiscal row | Samir, through a comptable | The rows are read off the texts and the fixtures agree with each other, which is not the same as an Algerian accountant saying they match practice. The article d'imposition is the sharpest of them: it prints on every facture in circulation and no text asks for it, so only an accountant can say whether leaving it off would cost a shop anything. | `docs/features.md`, each row's own note (gate R8) |
| A real printed facture from an Algerian shop has been seen | Anouar | The templates are golden files. They are frozen against what we believe a facture looks like, not against one somebody has held. | `docs/features.md` open decision 4 |
| A native speaker has read the Arabic | Anouar, or somebody he names | The amount in words on a facture is a legal phrase and the app writes it. The public page's Arabic needs the same read. | gate R6, and `apps/landing/src/i18n/ar.ts` |
| Nobody can publish a release by rewriting the workflow on a branch | Samir, in the repository settings | A manual run uses the workflow file from the branch it is dispatched from. The guarantee holds for the file as written and not for a branch that rewrites it, and nothing requires review on `.github/workflows/` today. | a ruleset or a code owner on that folder |

## Ours and not started

| What | Why it matters | Where |
|---|---|---|
| A shopkeeper can see why the app would not start | A Windows release build has no console, so a refusal goes nowhere: the window never opens and nothing is said. The pre-upgrade copy made that easier to reach, because a full disk now stops the app. | M5 task T10 |
| The copies beside the shop file can be restored from a screen | The copy taken before a restore and the copy taken before an upgrade are both a file swap by hand. The restore route accepts the name of a daily copy and no other kind. | M5 task T11 |
| Sonar quoted as a gate, or dropped | It is only a gate if Rust support on the team server was verified. Until somebody checks, it is neither. | `docs/architecture.md` § Testing matrix |

## What no gate can cover

Most of this milestone cannot be demonstrated from the build machine. An
installer needs Windows, a signature needs a certificate that does not
exist, and an update needs a previous release to update from. Each piece is
built against a stand-in here and proven properly the day the real thing
exists. A task that says it is done and means it compiles is not done, and
the rows above say which is which.
