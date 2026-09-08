---
title: 'Frontend conventions'
slug: 'frontend-conventions'
status: 'active'
category: 'processes'
created: 20260908
tldr: 'Folder shape for both apps, the three-layer design package, the import rules eslint enforces, and which test runner owns which layer'
---
# Frontend conventions

Two apps read from one design package. `apps/desktop` is React 19 + Vite +
TanStack Router and Query inside Tauri. `apps/mobile` is Expo, not started
yet. The rules below are the same for both wherever they can be; where they
differ the row says which app it applies to.

Read alongside `coding-rules` (no `as`, i18n in `ar`/`fr`/`en`, logical CSS
properties). Provenance: `research/tooling/2026-09-08-brainqraft-mobile-conventions.md`.

## Folder shape

A feature is one directory. It owns its screens, its components, its hooks
and its pure helpers, and you can delete it in one `rm -r`.

`apps/desktop/src`:

```
routes/                  TanStack Router file routes, nothing else
features/{feature}/
  screens/               one file per screen, PascalCase
  components/            components only this feature uses
  hooks/                 query and mutation hooks for this feature
  utils/                 pure functions, the part that gets unit tested
components/              cross-feature UI, grouped by role, not by screen
  buttons/ inputs/ feedback/ overlays/ layout/
hooks/                   app-wide hooks
lib/                     query keys, formatting, the Tauri invoke wrapper
i18n/                    ar.json, fr.json, en.json, index.tsx
styles.css               imports the generated custom properties, nothing else
```

`apps/mobile/src` is the same tree, with `screens/` under each feature and
the router layer replacing `routes/`. Assets mirror the feature names:
`assets/images/{feature}/`.

Rules:

- A component moves out of `features/{a}/components` into `components/` the
  first time `features/{b}` needs it. It never gets imported across
  features from where it is.
- `routes/` holds routing. A route file renders one screen from
  `features/{feature}/screens` and does nothing else.
- Tests live in `__tests__/` beside the source, or as `x.test.ts` beside
  `x.ts`. Pick one per app and hold it; desktop uses the second today
  (`src/i18n/i18n.test.tsx`).
- Files: components PascalCase, hooks `useThing.ts`, everything else
  camelCase.

## The design package

`packages/design` (`@dzpos/design`) holds the tokens for both apps. Three
layers, one source of values.

| File | Layer | Holds |
|---|---|---|
| `src/primitives.ts` | 1 | the raw ramps: `stone`, `teal`, `red`, `amber`, `blue`, keyed by step. Not a component API. |
| `src/semantic.ts` | 2 | role tokens that point at a primitive step, plus the scales: `space`, `radius`, `shadow`, `fontFamily`, `fontSize`, `layout`. This is the layer that gets edited when a role changes. |
| `src/theme.ts` | assembly | the single `theme` object with every reference resolved to a value. What TypeScript imports. |
| `src/css.ts` | emitter | turns the same semantic layer into the `:root` and `[dir="rtl"]` custom-property blocks. |
| `src/index.ts` | barrel | exports `theme`, `Theme`, the semantic groups and `toCss`. It does not re-export `primitives`. |

`design/shared/tokens.css` is the hand-written source of the values today,
and the mockups in `design/` load it directly. `src/css.ts` emits the same
variable names and the same values, and `src/css.test.ts` fails if the two
sets differ by one name or one value. So the mockups and the apps cannot
drift while both exist. When the mockups go, `tokens.css` is generated from
the package and the test becomes a snapshot.

`theme` is a plain object, not a hook. It works at module scope, inside a
`StyleSheet.create` on mobile and inside a `useMemo`-free helper on the
desktop.

Where each app reads tokens, one answer per app so there are not two ways:

- **Desktop reads the CSS custom properties.** Stylesheets and the Tailwind
  `@theme` block use `var(--surface-card)`. TypeScript imports `theme` only
  where a value has to be computed in JS: a canvas, a chart, an inline
  style that depends on data.
- **Mobile reads `theme`.** There are no custom properties in React
  Native.

Rules for the package:

- Tier 1 is never imported by a component in either app. The only importer
  of `src/primitives.ts` is `src/semantic.ts`.
- The package is platform-neutral. No `react-native` import, no
  `Platform.OS`, no `document`. A per-platform font family map belongs in
  `apps/mobile`, not here.
- Spacing, radius, font size and control heights are numbers in TypeScript.
  The CSS emitter appends `px`. React Native gets the number it needs and
  the browser gets the unit it needs, from one entry.
- A value with no role gets no token. If a screen needs a number the scale
  does not have, widen the scale or use `%`, `flex` or `gap`. A one-off
  literal in a component is the thing this package exists to stop.
- A hand-added value that the source cannot produce carries a comment
  saying it is hand-added and why. Otherwise the next regeneration eats it.

## Import rules

Enforced by eslint `no-restricted-imports` in each app, not by review.

| Rule | Restriction |
|---|---|
| Components import `theme`, never primitives | ban the patterns `@dzpos/design/primitives` and `**/design/src/primitives` |
| Screens do not fetch | ban `useQuery` / `useMutation` / `useSuspenseQuery` from `@tanstack/react-query`, with an override that re-allows them inside `features/*/hooks/**` and `src/lib/**` |
| Screens do not call Tauri | ban `invoke` from `@tauri-apps/api/core` outside `src/lib/**` (desktop only) |
| One place per cross-cutting side effect | when a second one appears (logging, telemetry), it gets one hook and a ban on the SDK, the same shape as the two above |

Two rules `no-restricted-imports` cannot express:

- **Every visible string goes through `t()`.** `eslint-plugin-i18next`'s
  `no-literal-string` is the check; until it is installed this is review
  plus the i18n test that asserts the three locales carry the same key set.
- **No hardcoded pixel.** No lint rule reads intent. The habit is: a number
  in a style is a token lookup or it is a mistake.

Import order is `import/order` with the groups `builtin, external,
internal, parent, sibling, index`, alphabetised, blank line between groups.
Path alias is `@/` and only `@/`.

Query keys come from a factory in `lib/queryKeys.ts`, never written as
literals at a call site. `getQueryData` matches a key by exact hash while
`invalidateQueries` matches by prefix, so an extra element breaks cache
reads while invalidation keeps working and nothing fails loudly.

Selectors passed to a query hook are declared at module scope. An inline
arrow has a fresh identity every render, which throws away the memoisation
the selector was for.

## Testing

| Layer | `apps/desktop` | `apps/mobile` | `packages/design` |
|---|---|---|---|
| Unit, pure functions | vitest, `src/**/*.test.ts` | Jest, `jest-expo` preset | vitest, `src/**/*.test.ts` |
| Component | vitest + `@testing-library/react`, jsdom | Jest + `@testing-library/react-native` | n/a |
| End to end | Playwright against the Vite dev server | Maestro flows in `.maestro/`, on a real build | n/a |
| In production | ObserveOne, once there is a shop running it | ObserveOne | n/a |

Coverage is collected from `lib/`, `hooks/` and `features/**/utils/`. A
screen is covered by its e2e flow, not by a snapshot.

What a test has to do to count, on top of `quality-gates`:

- One `describe` per exported function, with a two to six line contract
  header above it saying what the function does and the edge case that
  shapes it. A reviewer reads that instead of the implementation.
- Cover in order: happy path, then empty and single-element and
  `null`/`undefined`, then corrupt input, then non-mutation of the input.
- Pin a product constant literally. `expect(STAMP_CAP).toBe(...)` shows an
  economy change in the diff; `expect(total).toBe(STAMP_CAP)` passes
  whatever the constant becomes.
- Anything reading `Math.random` gets a seeded spy plus a fuzz pass over
  a few hundred seeds. Fixed seeds miss guard removals.
- e2e selectors are `data-testid` on the desktop and `testID` on mobile,
  never visible text. Every label goes through i18n in three languages, so
  a text selector is a locale-dependent failure waiting to happen. Add an
  id only when a flow needs it, and list it in the flow's README table.

## Open

- `as const` on a token object is an `as`, and `coding-rules` bans `as`
  without an exception. The package is written without it, using explicit
  interfaces. Confirm that reading, or carve out `as const` in
  `coding-rules` and the eslint config can allow it.
- `design/shared/tokens.css` has a header comment naming
  `packages/shared/design/tokens.json` as the future home. The package
  landed at `packages/design`. Decide which name stands before either file
  is regenerated.
