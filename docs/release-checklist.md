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
| A shopkeeper is told why the app would not start | `apps/desktop/src-tauri/src/startup_failure.rs`, called from `main.rs` before the window exists | four tests on the message, and the Windows job compiles and links the message box |
| A restore that has to migrate the copy it puts in place copies it first | `crates/api/src/lib.rs::restore` step 9, through `open_and_upgrade` | `restoring_a_copy_that_is_behind_leaves_a_copy_of_it_before_it_is_migrated` |
| Every ledger column a day filter reads carries a moment from the shop clock, not the file's UTC default | `crates/core/src/services/audit.rs::record`, `repos::debt::append`, `repos::supplier_debt::append` | the refusal test in each repo, and `a_row_is_stamped_from_the_shop_clock_and_not_from_the_file` |
| A content security policy on the window and the launch token out of the page global | `apps/desktop/src-tauri/`, `tauri.conf.json` | a test that reads the policy off the real built page, and one that a navigation away is refused |
| A tag on `main` as the only thing that publishes a release | `.github/workflows/release.yml`, `.github/scripts/release-gate.sh` | eighteen cases in that script's own test |
| A shop can ask the app to check for the next version, and see one of three answers; the update is only ever signed and installed with the shop's own yes | `apps/desktop/src-tauri/src/updater.rs`, `tauri.conf.json`'s `plugins.updater`, the About screen's `UpdateCheckPanel` | `updater_config_tests` (the pubkey is still the named placeholder, every endpoint is https), the shape test and the two manifest-`size` tests in `updater.rs`, and the screen's own tests for each answer and the install dialog |
| The release workflow refuses to publish an unsigned updater manifest; a release with no signing key still ships, with no manifest. The two keys are coupled in practice: a build with a Windows code-signing certificate configured but no updater signing key stops with an error instead of shipping an installer the release would wrongly call signed, since `--no-sign` would have silently skipped both | `.github/workflows/release.yml` (`gate` job's `updater` output, `build` job's manifest step and its "Build the Windows installer (signed)" step) | four of the eighteen cases above, `release-gate.test.sh` §9 |
| One command from settings writes a zip a shop can send, carrying the log, the versions and the schema and no customer name, price or credential | `crates/core/src/services/support.rs`, the settings screen's support panel | a test that lists what went in and refuses anything outside the list |
| All three kinds of copy beside the shop file are listed on the backups screen and any of them can be put back | `apps/desktop/src/components/BackupsPanel.tsx`, `crates/api` restore route | the panel's own tests, and a restore route that picks the folder from the shape of the name rather than from the caller |
| A value on an Arabic page keeps the order it is read in, on the screens and on the six printed documents | the templates under `crates/core/templates/`, the till, the customer fiche and the Excel panel | `crates/core/tests/print_bidi.rs`, which runs the bidi algorithm over all forty-four golden pages rather than reading the markup for a rule |
| The product name is Dinar and the bundle identifier is `com.dinar.app` | `apps/desktop/src-tauri/tauri.conf.json`, window title, landing copy, `docs/features.md` decision 2 | crate and path names stay `dz-pos`; changing the identifier after a tag would be a data-path migration |
| Every date box and the one month box read in the shop's language and the shop's order | `apps/desktop/src/components/ui/date-field.tsx` and `month-field.tsx`, with the calendar arithmetic in `apps/desktop/src/lib/date-segments.ts` | the kit's own tests: a day the month does not have is brought back to the last one it does, the 29th of February stands in a leap year and not in the year before, and the field's label names the whole control rather than one box of it |
| Sonar on the team server with the official Rust plugin | `just sonar` to sonar.observeone.com, project `dz-pos`, gate ObserveOne way | Rust plugin 1.5.0; runs on this machine, not in CI |
| The text that puts the NIF on a facture, and the absence of one for the article d'imposition | `docs/features.md` party identifiers row, from `research/legal-fiscal/2026-09-08-facture-and-ticket.md` | loi 04-02 art. 34 and LF 2006 art. 42, quoted from the Journal Officiel PDFs in that note's `sources/` |

## Waiting on somebody

| What has to be true | Who holds it | Why it cannot start without them | Where the answer goes |
|---|---|---|---|
| A Windows code-signing certificate is bought | Anouar | Without one, every customer who installs sees a warning that the software is from an unknown publisher. Buying one takes days, which is why it sits at the top of this list rather than at the end. | the release workflow's secrets; the workflow already builds unsigned and marks the release a draft when the secret is absent |
| Somebody holds the key that signs updates | Anouar and Samir | The key never enters the repository and the build signs with a secret. Until it exists there is no update to sign, and until somebody owns it there is nobody to regenerate it if it leaks. | `docs/architecture.md` § Release |
| A comptable has confirmed every fiscal row | Samir, through a comptable | The rows are read off the texts and the fixtures agree with each other, which is not the same as an Algerian accountant saying they match practice. The article d'imposition is the sharpest of them: it prints on every facture in circulation and no text asks for it, so only an accountant can say whether leaving it off would cost a shop anything. | `docs/features.md`, each row's own note (gate R8) |
| A real printed facture from an Algerian shop has been seen | Anouar | The templates are golden files. They are frozen against what we believe a facture looks like, not against one somebody has held. | `docs/features.md` open decision 4 |
| A native speaker has read the Arabic | Anouar, or somebody he names | The amount in words on a facture is a legal phrase and the app writes it. The public page's Arabic needs the same read. | gate R6, and `apps/landing/src/i18n/ar.ts` |
| Nobody can publish a release by rewriting the workflow on a branch | Samir, in the repository settings | A manual run uses the workflow file from the branch it is dispatched from. The guarantee holds for the file as written and not for a branch that rewrites it, and nothing requires review on `.github/workflows/` today. | a ruleset or a code owner on that folder |

## Ours and not started

Nothing in this milestone. What is left is waiting on somebody, above.

## What no gate can cover

Most of this milestone cannot be demonstrated from the build machine. An
installer needs Windows, a signature needs a certificate that does not
exist, and an update needs a previous release to update from. Each piece is
built against a stand-in here and proven properly the day the real thing
exists. A task that says it is done and means it compiles is not done, and
the rows above say which is which.
