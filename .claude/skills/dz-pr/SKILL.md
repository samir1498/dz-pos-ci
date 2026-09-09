---
name: dz-pr
description: Open a pull request in the dz-pos repo. Use whenever a dz-pos branch is ready for review or merge, or Samir says "PR it", "open the PR", "ship it". Covers branch naming, the gate run that must precede it, and the PR body sections including the fiscal assumptions list.
argument-hint: [<base-branch>]
disable-model-invocation: true
---

## Branch

`feature/<name>`, `fix/<name>`, `chore/<name>`, `docs/<name>` off
`origin/main`; inside a milestone, `m<N>/<task>` off the milestone branch
(`m1/2026-09-09`), and the milestone branch itself goes to `main` at its
checkpoint tasks. One session works this repo per machine, so no worktree; if
that changes, `CLAUDE.md` says what to do.

## Gates first, evidence in hand

Run these from the repo root and keep the tail of each output for the body:

```
just gates   # fmt, clippy, generated types check, cargo test, pnpm test, builds
just e2e     # Playwright against a fresh API and database
```

A green CI run is not tested. Say what you drove by hand or by script:
which screen, which mockup route, which HTTP call. If a gate did not run
(no display for Tauri, laptop offline), write that in the body rather than
leaving the checkbox ticked.

`cargo build` of the Tauri app on the WSL box is a heavy job under the
shared-box rule in `CLAUDE.md`; announce it before running it.

## PR body

No file paths in the body; names in backticks; no emojis; title under 70
characters. Every visible-UI change gets a `[Screenshot Placeholder]`.

```
gh pr create --base ${1:-main} --title "<type>(<scope>): <title>" --body "$(cat <<'EOF'
## Summary
<what and why, two to four lines>

## Changes
<grouped by area: core / api / desktop / mobile / shared / docs / design>

## Assumptions pinned
<one line per fiscal or business rule this change touches:
 rule, the constant it uses, the fixture that asserts it, and whether
 Anouar or a comptable has confirmed it. "none" if the change touches no rule.>

## Gates
- fmt / clippy / test: <pass, with counts>
- pnpm build / test: <pass, with counts>
- Driven by hand: <what, where, on which machine>
- Not run: <what and why>

[Screenshot Placeholder]

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

The "Assumptions pinned" section exists because every fiscal rule in
`docs/features.md` is an assumption until someone with a stamped facture
confirms it. A reviewer needs to see which ones a change leans on without
opening the fixtures.

## After opening

Post the URL. Merging is Samir's call unless he has said otherwise for the
branch; when he has, squash-merge, delete the branch, and log the close in
`context/` with the `ctx:` trailer on the squash commit.
