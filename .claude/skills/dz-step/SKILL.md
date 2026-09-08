---
name: dz-step
description: Work dz-pos one step at a time. Use at the start of every dz-pos session, whenever Samir says "next", "go", "step by step", "where were we", "what now", or feels lost about where to start; and whenever a task is about to grow past one PR.
argument-hint: [next|status|done]
---

Starting a new product is hard because everything looks like the first
thing. This skill makes the next thing small and singular: one step in
flight, one PR per step, Samir looks at it, then the next one.

## The ladder

`context/progress/now.md` holds the ladder: a numbered list of steps,
the one in flight marked, the rest in order. Read it first. If it has no
ladder, build one from the active plans (`ctx plan list --status active`)
and write it there before doing anything else. The ladder changes when
Samir says so or when a step teaches us something; rewriting it is cheap,
so keep it honest rather than keep it stable.

## One step

A step is the smallest unit that leaves something Samir can see, run, or
read on its own: a rule doc with the article quoted, a newtype with its
fixtures and a passing proptest, one endpoint plus the screen that calls
it. If the description needs "and" twice, split it.

Every step runs the same way:

1. Open with three lines: `Step N of M`, what it produces, what Samir
   will look at when it is done. No plan for the steps after it.
2. Do only that step. Something discovered on the way goes into the
   ladder or a plan task, not into this PR.
3. One branch, one PR (`dz-pr`), gates in the body. Money, roles or
   deletion also get `dz-review` before the PR is called ready.
4. Close with where to look (a file, a URL, a command to run) and the
   next step's one-line name. Then stop. Samir says "go" or changes the
   ladder; nothing starts on its own.

## Commands

- **`status`** (or a session start, or "where were we"): read `now.md`,
  say which step is in flight, what is done in it, what remains, and what
  Samir needs to look at. Two to four sentences.
- **`next`** / "go": mark the next step in flight in `now.md`, then run
  the step.
- **`done`**: the PR merged. Tick the step in `now.md`, log one progress
  line (`dz-context update`), name the next step, stop.

## Why the stop matters

Samir reviews between steps, and a review is only possible when the diff
is one idea. Three steps merged while he was away is three reviews he
cannot do, and the second one may have built on a mistake in the first.
Waiting is the feature.
