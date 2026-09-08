---
name: step-by-step
description: Deliver a multi-step walkthrough ONE step at a time, pausing for the user to confirm before sending the next. Use when the user says "step by step", "one step at a time", "one at a time", "walk me through", or signals fatigue / low bandwidth ("I'm tired", "can't process a lot of info"). Applies to any manual flow — publishing, setup, config, ops, debugging.
---

# Step by step (one at a time)

The user wants **minimum cognitive load**. Walk them through the process one step at a time.

## Rules
- Present **exactly ONE step per message.** Never list the full sequence, never pre-send later steps.
- Keep each step short: **what to do + where** (full clickable links / exact paths) — nothing else.
- **Stop and wait** for their confirmation ("done" / "next" / a pasted result) before the next step.
- No walls of text, no background, no caveats unless they block the current step.
- If a step produces something needed downstream (an ID, slug, URL, error), ask for **exactly that one thing**.
- If they hand back an error or result, resolve it, then continue from where you left off.
- Number the steps ("Step 1", "Step 2", …) so they feel progress; you may note the total count once.
- Do the codeable parts yourself; only hand the user the steps that genuinely require them (UI clicks, owner-only actions, auth).

## Key fact about this user
When this user says **"step by step", they mean literally one step at a time** — not a numbered list of all steps in one message. Defaulting to a full list is the mistake this skill exists to prevent.

## Per-step validation loop (for any non-mechanical step)
If a step is a judgment call, a decision, or a change to something that already exists (not a pure "click here" / "run this"), don't act inside the step. Instead, in this order:
1. **Explain** what the step/task actually asks, in plain terms.
2. **Check if it's already done** — read the real file/code/data; never infer this from a plan description or task title alone.
3. **State exactly what would need to change** — a proposal, not yet applied.
4. **Stop and wait** for the user to validate or cancel.

Never collapse these into "I checked, so I went ahead and did it" — that skips the user's own validate/cancel step, which is exactly the loop this section exists to keep them in.
