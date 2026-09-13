---
title: 'Repo tooling, skills and rules for dz-pos'
slug: 'repo-tooling-skills-and-rules-for-dz-pos'
status: 'done'
category: 'other'
created: 20260908
tldr: 'TODO: add summary'
tasks:
  - id: 'T1'
    desc: 'Project skills in .claude/skills: dz-context, dz-pr, dz-money, dz-review, dz-mockup, laptop-dev, git-commit-convention, dont-sound-like-ai; symlinked into ~/.claude/skills'
    status: 'done'
  - id: 'T2'
    desc: 'CLAUDE.md rules: heavy jobs, no worktree ceremony, provenance homes, TS/CSS rules, ISO 27001 line'
    status: 'done'
  - id: 'T3'
    desc: 'Laptop setup: clone, pnpm install, webkit dev packages (Samir), warm cargo build'
    status: 'done'
  - id: 'T4'
    desc: 'Sonar: verify Rust support on the team server before promising a gate'
    status: 'done'
  - id: 'T5'
    desc: 'CI clippy runs with --all-targets like the justfile and quality-gates prescribe (.github/workflows/ci.yml)'
    status: 'done'
  - id: 'T6'
    desc: 'dz-review gains the three sections pc-review (laptop, ~/.claude/skills/pc-review) carries: what every finding must state, a fix proven by a test that fails without it, the traps list; quality-gates gets the proptest/fast-check paragraph from property-fuzz-tests'
    status: 'done'
  - id: 'T7'
    desc: 'coding-rules: no hardcoded Figma pixel or hex in UI code, tokens from design/shared/tokens.css or percentages (rule copied from the freelance workspace AGENTS.md)'
    status: 'done'
  - id: 'T8'
    desc: 'CI is light and post-merge on the personal mirror samir1498/dz-pos-ci: fmt, desktop eslint, release-gate script. Compiles, clippy, cargo test, pnpm test/build, Windows and coverage stay on the machine (just gates). Dinar-dz PRs do not start Actions. just ci copies main to the mirror after merge.'
    status: 'done'
acceptance: []
completed_at: '2026-09-13'
---
# Repo tooling, skills and rules for dz-pos

## Goal

TODO: define goal

## Scope

TODO: define scope
