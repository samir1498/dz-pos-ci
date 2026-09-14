# frontend-design verdict — 2026-09-14

Trial: ran `frontend-design` skill against the first real screen that landed after the mockups (till `apps/desktop/src/routes/index.tsx` + `packages/design` tokens, `apps/desktop/src/components/ui/*` shadcn kit).

What it does: asks for a distinctive, production-grade component with tokens in three tiers (primitive → semantic → component), then generates it. Comparable to our `packages/design` (primitive/semantic/component) + `apps/desktop/src/theme.css` + `components/ui` already in repo since D2 (Comptoir/Registre/Observe).

Verdict: **not adopted as a separate skill for dz-pos.**

- The till, products, dashboard, and facture screens were already built on `@dzpos/design` tokens (three themes, `data-theme` switch, vendored Plex + JetBrains Mono, Lucide) and the shadcn kit installed via CLI (`button`, `input`, `card`, `dialog`, `table`, etc.) with the `no bare input/button` eslint gate. `frontend-design` would have regenerated a similar kit, not extended it.
- The skill is useful as a reference for a new screen that needs a one-off distinctive component (e.g., a marketing card), but the product's own `dz-mockup` HTML mockups + `design/desktop/app.js` + `money.js` remain the source of truth for layout and fiscal copy, and the kit enforces consistency — a generated one-off would drift.
- Keeps `~/.claude/settings.json` disabled (installed 2026-09-11, never enabled). No install to keep.

If a future screen needs a bespoke marketing component, re-trial on that screen; otherwise the existing tokens + kit + `theme.test.ts` + `role.test.ts` gates are the design system.
