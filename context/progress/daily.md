---
type: 'daily'
---
# YYYY-MM-DD

## YYYY-MM-DD HH:MM UTC
- (intraday log entries)
## 2026-09-08 11:06 UTC

- Scaffolded context/ inside the repo; first plan money-module active (7 tasks). Mockups live at dz-pos-design.pages.dev; laptop-dev skill written.
## 2026-09-08 11:29 UTC

- Skills and rules landed (PR #1): 8 skills in .claude/skills, CLAUDE.md is an index over context/processes (5 pages) + context/repos/dz-pos; clippy denies unwrap/expect. Next: money module.
## 2026-09-08 12:03 UTC

- Research moved into research/ in this repo (PR #3). Stamp rule superseded by Code du timbre 2026 art. 100-I; plan legal-fiscal-and-tooling-research-for-dz-pos R1–R9 open. Samir reviewing.
## 2026-09-08 16:50 UTC

- EOD handoff written (docs/handoff-2026-09-08.md + handoffs/2026-09-08-eod). Next session opens in /home/samir/dz-pos; step in flight: Money newtype.
## 2026-09-08 18:09 UTC

- Night build started (Samir: "take the wheel", 2026-09-08 19:55). T1 Money newtype committed on core/money-newtype (1205a3d): Money(i64), Bps, pct half away from zero, fixtures tva_rounding_once_per_rate + money_no_float, 8 proptests, mutation check red on the exact-half case. Five tracks running in worktrees off that branch: A money rules (core/money-rules), B words (core/money-words), C migration+api+shared+screen+e2e (core/first-migration-api), D tooling T5-T7 (tooling/housekeeping), E research R3/R6/R7/R8 (research/legal-r3-r8). Integration branch night/2026-09-08; no PRs opened by the session (dz-pr is user-only). (@night-build, @money)
## 2026-09-08 18:18 UTC

- Night build 20:17: T1 reviewed (two lenses) and fixed in 49e9e1a (Bps bounded to 10 000, no Default on Money, fixture and proptest gaps closed). Track D (tooling T5-T7) reviewed, three lost clauses restored, merged into night/2026-09-08. Traps: Agent worktrees start from main; a shared CARGO_TARGET_DIR across worktrees serves stale artifacts for same-named crates. Track F added on Samir's request: brainqraft-mobile conventions into a research page, a frontend-conventions process and a packages/design scaffold. (@night-build)
## 2026-09-08 18:40 UTC

- Night build 20:40: tracks A (money rules), B (words), E (research R3/R7 closed, R6/R8 progress) and F (frontend conventions + packages/design) delivered and pushed. Reviews: A Rust centimes clean by hand recomputation, three mockup defects and two proptest gaps in fix round; B fr/en clean, round-trip proptest missing, in fix round; E and F reviews running; C (migration/api/screen) still building. (@night-build)
## 2026-09-08 19:21 UTC

- Night build 21:25: tracks G (Playwright e2e + products screenshot) and E fix round 1 (11 citation findings) merged into night/2026-09-08. Money plan T1-T6, T8 done, T7 (gates + PR) in progress; research R3, R7 done. Remaining: track C fix round 1 (16 findings), final whole-branch review, full gates, handoff. (@night-build, @money, @research, @e2e)
## 2026-09-13 07:19 UTC

- Product name decided: Dinar. M5 T8 (rename once) is unblocked; still last so it is a rename, not a rewrite. (@m5, @naming)
## 2026-09-13 07:28 UTC

- pc-ctx MCP wired for Cursor: .cursor/mcp.json + .mcp.json pinned to @pc-ctx/mcp@0.8.0 (0.9.1 npm publish is broken). (@tooling, @mcp)
## 2026-09-13 07:32 UTC

- pc-ctx MCP was up but bound wrong: ${workspaceFolder} did not expand, so it missed context/. Fixed .cursor/mcp.json to absolute /home/samir/dz-pos/context — needs one more Cursor restart. (@tooling, @mcp)
## 2026-09-13 07:38 UTC

- No thermal printer available. M1 T6 stays blocked: ESC/POS + byte golden are the software half; the live USB print cannot be demoed until hardware exists. (@m1, @printer)
## 2026-09-13 07:40 UTC

- design-system: dropped D4 (Claude Design sync). D5 marked done — landing shots live under landing-page, which is already closed. (@design)
## 2026-09-13 07:49 UTC

- Starting M5 T7 (Tauri updater): finish worktree feat/a-shop-can-take-the-next-version — rebase on main, add client refuse of unsigned/wrong signature, gates, PR. (@m5, @t7)
## 2026-09-13 09:46 UTC

- M5 T7 (Tauri updater) committed and PR #57 opened: About checks on demand, install only on yes, unsigned/wrong signatures refused. just gates green on fedora-wsl. e2e not run (11G on C:, no display). Next: merge #57, then T8 rename to Dinar. (@m5, @t7)
## 2026-09-13 09:54 UTC

- CI cut down: PRs do not start Actions; compiles stay on the machine; after merge, just ci copies main to samir1498/dz-pos-ci for rustfmt + desktop eslint + the release-gate script (PR). Mirror run also refused to start: samir1498 is on the same spending-limit message as Dinar-dz. Needs a billing bump or a third account before the light job actually runs. (@ci, @tooling)
## 2026-09-13 10:01 UTC

- Sonar wired like ObserveOne: project dz-pos on sonar.observeone.com (private, ObserveOne way gate). Rust plugin 1.5.0 is on the server (85 rules; rust:S1481 was the wrong id). just sonar scans locally, not CI. Grok/Claude/Cursor MCP sonarqube handshakes (18 tools). Needs a new Grok session to call the tools. (@sonar, @tooling)
## 2026-09-13 10:47 UTC

- CI actually ran on samir1498: made dz-pos-ci public (private was on the same spending cap as the org). Light job green in 1m7s: fmt, desktop eslint, release-gate script. https://github.com/samir1498/dz-pos-ci/actions/runs/34752693882 (@ci)
## 2026-09-13 11:03 UTC

- CI split on samir1498/dz-pos-ci (public, free): Restricted green in 58s; Full rust+web green (Windows/coverage stay main-only). Dinar-dz still starts no runner. just ci copies the branch; just ci <branch> full asks for the long one. (@ci)
## 2026-09-13 11:32 UTC

- Merged PR #57 (updater, M5 T7) and PR #58 (Restricted+Full CI on samir1498/dz-pos-ci, Sonar). Next is M5 T8: one commit renaming the product to Dinar. (@m5, @ci)
## 2026-09-13 11:44 UTC

- Closing loop started. Real thermal printer dropped (M1 T6 cancelled). Laptop clone+pnpm running over Tailscale. M5 T8 Dinar rename next, then T9 sweep. Skip R6/R8. (@loop, @m5, @m1)
## 2026-09-13 11:59 UTC

- Closing loop: M1 T6 cancelled (no printer). Laptop clone+pnpm on fedora, core/api/seed cargo warm; webkit still Samir. T8 Dinar rename committed (com.dinar.app). T9 checklist/roadmap sweep committed. PR opened. (@loop, @m5, @m1, @laptop)
## 2026-09-13 14:10 UTC

- Nine finished plans archived (money, M1–M5, tooling, design, landing). Left in the store: research 6/9 (R6 native Arabic, R8 comptable, R9 tooling trials) and M6 paused. M5 tasks are done; a signed installer is not. Landing is built, not published. (@archive)
## 2026-09-13 17:27 UTC

- MCP rebound to dz-pos via local opencode.json (global pc-ctx removed; observeone got its own local). ESC/POS T2 shipped as PR #66 (file + TCP:9100 senders, USB/route later). dz-review of first-setup (fc08f0a) done via 3 lenses: 1 real gap (no password reset route over HTTP — staff office passwords unsettable, owner password unrotatable; needs product call), rename residue + 6 test gaps fixed as PR #67 (mirror green; role guard needed one allowlist entry). Research still 6/9, M6 paused. (@mcp, @escpos, @review, @auth)
## 2026-09-13 18:43 UTC

- Release loop shipped as PR #68 (matrix win/mac/linux on mirror, org publish via PAT, stable names, landing Download fr/en/ar with OS sniff). Proven: dry-run built all three, Dinar-Setup.exe pulled off the artifact; branch CI green. Left: Samir creates ORG_RELEASE_TOKEN on the mirror (fine-grained PAT, contents read+write on Dinar-dz/dz-pos), then just release vX. Plan 4/5, T2 pending token. (@release, @landing)
## 2026-09-13 19:07 UTC

- ORG_RELEASE_TOKEN set on the mirror from Samir's own login (push on the org repo; boss account untouched). Docs corrected to say so + PAT replacement note (PR #69). Plan 5/5. Next: just release vX when Samir names the version. (@release)

