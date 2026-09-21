---
title: 'Frontend conventions'
slug: 'frontend-conventions'
status: 'active'
category: 'processes'
created: 20260908
tldr: 'Folder shape for both apps, the three-layer design package, the import rules (a convention until a lint rule exists), and which test runner owns which layer'
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
| `src/css.ts` | emitter | turns the same semantic layer into `:root` (Comptoir), one `[data-theme="<name>"]` block per other theme, and `[dir="rtl"]`. |
| `src/themeCss.ts` | emitter | writes `apps/desktop/src/theme.css`: the blocks above, the shadcn/ui variable set pointing at our roles, and the Tailwind v4 `@theme` map. `just theme` runs it; `themeCss.test.ts` fails the gates on a stale file. |
| `src/index.ts` | barrel | exports `theme`, `Theme`, `themes`, `THEMES`, `DEFAULT_THEME`, the semantic groups, `toCss` and `themeSelector`. It does not re-export `primitives`. |

The theme axis. Tier 2 carries four themes, not one: `comptoir` (light,
the default and what `:root` holds), `registre` (dark ink), `observe` (cool
grey, emerald brand, rounder) and `observe-dark`. A theme owns colour,
shadow and radius. Space, font size and the control heights are off the
axis, because a theme changes what the app is made of and never how much
room it takes. `themes` in `semantic.ts` is the map, and every theme names
every role the others do: `css.test.ts` compares the key sets and fails on a
role left out, which would otherwise be a light colour on a dark surface on
the one screen nobody opened. It also holds each theme to a contrast floor,
4.5:1 for body text on its own background and 3:1 for a status colour or
the text on a filled control.

`DEFAULT_THEME` is Comptoir, the theme a shop that has never chosen opens on; the operating system's light-or-dark setting is not consulted (Samir, 2026-09-10). The other
three are picked by hand.

The switch is `data-theme` on `<html>` and nothing else. Every theme is a
block of CSS variables, so a component wears `bg-background` and
`text-muted-foreground` and never learns which theme is on. No
`theme === "x" ? a : b`, no theme-conditional class list, no per-theme
component: `apps/desktop/src/theme.test.ts` greps for exactly that and
allows a theme name in four files only (the provider, the switcher, their
test, the generated CSS). Adding a fifth theme is an entry in `THEMES`, a
group of role values, and a label key in the three i18n files.

The kit is shadcn/ui (`apps/desktop/components.json`, style new-york,
cssVariables, lucide), so shadcn's names are the emitted API and our roles
are the source. Components are installed with the CLI; never run
`shadcn init`, which rewrites `styles.css`. See "The kit" below for what
the CLI gets wrong on the way in.

## The kit

Two folders, one rule about which is which.

| Folder | What lives there | Who wrote it |
|---|---|---|
| `src/components/ui/` | shadcn/ui, one file per component. `button`, `input`, `label`, `select`, `checkbox`, `switch`, `textarea`, `table`, `card`, `badge`, `dialog`, `sheet`, `dropdown-menu`, `tabs`, `separator`, `skeleton`, `scroll-area`, `sidebar`, `breadcrumb`, `tooltip`, `sonner`, `chart`, `date-field`, `input-otp`, `month-field`. The folder is the list; a count here drifts every time the CLI adds one. | The CLI, then corrected by hand. Re-add one with `pnpm dlx shadcn@latest add <name>` and redo the corrections below. |
| `src/components/*.tsx` | Ours, on top of them: `AppShell`, `PageHeader`, `FormField`, `StatusPill`, `EmptyState`, `DataTable`, `MoneyInput`, `PayButton`, plus D2's `Money`, `Wordmark`, `Icon`, `ThemeSwitcher`. | Us. A screen imports from here first and reaches into `ui/` only for a control the kit has no opinion about. |

`src/kit/KitPage.tsx` is every component in every state on one page, at
`/kit` in a dev build only (`routes/kit.tsx` throws `notFound()` otherwise,
and the import is dynamic so the page leaves the shipped bundle). It is what
a reviewer compares against the mockups, and `e2e/kit.spec.ts` photographs
it once per theme into `e2e/screenshots/kit-<theme>.png`.

**What the CLI gets wrong, every time.** It resolved the `cn` import to a
package named `cn` on npm instead of `@/lib/utils`, and it appended a
hardcoded sidebar palette in `hsl()` plus a `.dark` class variant to
`styles.css`. Revert that file and fix the imports. Then, in the files it
wrote: strip every `dark:` variant (Tailwind's own dark variant answers to
the machine, so it fires on a dark laptop whose shop chose the light theme);
`bg-black/50` becomes `bg-scrim` and `text-white` becomes a foreground role;
content-side physical properties become logical (`text-start`, `ps-`, `pe-`,
`ms-`, `end-`, `border-s`); and `as React.CSSProperties` comes out, because
`src/css-vars.d.ts` already widens the type for every file.

The sheet and the sidebar keep a **physical** `side`, on purpose. A panel's
edge, its border and the half it slides in from all have to agree, and
`AppShell` computes the side from the page direction. Radix also keeps its
own direction context and defaults to `ltr` whatever the document says, so
the shell wraps everything in `Direction.Provider`; without it an Arabic
select takes the arrow keys backwards.

**The lint.** `apps/desktop/eslint.config.js` carries one rule: no `<input>`,
`<button>`, `<select>`, `<textarea>` or `<table>` in JSX outside
`components/ui/` and the kit. A bare element wears the browser's colour and
height and the platform's focus ring, and it looks like nothing in a diff,
which is why it is a rule rather than a review note. Tests are out of scope.
`just lint` runs it; `just gates` runs it second, after `fmt`, because it
needs no cargo.

**The allowlist.** Every file written before the kit is named in
`apps/desktop/src/lint/allowlist.json` with the screen it belongs to, so the
list reads as work left rather than as permission. Eighteen entries when the
kit landed. `src/lint/allowlist.test.ts` fails on an entry whose file is
gone, and on an entry whose file has nothing left to fix: rewrite a screen
on the kit and the gates make you delete its line in the same commit, which
is what makes the count reach zero.

Beside the lint, `src/tokens.test.ts` refuses `bg-[`, `text-[` and a hex
anywhere, a `dark:` variant anywhere, and a hardcoded size (`p-[13px]`,
`w-[240px]`) on a screen. The kit may still spell a size the scale has no
name for, once, with a comment saying why.

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
  `@theme` block use `var(--surface-card)`, and a component reaches them
  through the utility classes the generated map makes (`bg-card`,
  `text-muted-foreground`, `font-numeric`). A literal colour is a gate
  failure: `apps/desktop/src/tokens.test.ts` refuses `bg-[`, `text-[` and a
  hex outside the generated file. TypeScript imports `theme` only
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
| Screens do not call Tauri | ban `@tauri-apps/api/core` outside `src/lib/**`, so every command goes through one wrapper (desktop only) |
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
| End to end | ObserveOne is the e2e tool for this product (Samir, 2026-09-08, `context/progress/now.md`). Playwright against the Vite dev server is the interim local driver for tonight's screens. `docs/architecture.md`'s Testing matrix still lists tauri-driver + WebDriver under `xvfb-run` for the native window. | Maestro flows in `.maestro/`, on a real build | n/a |
| In production | ObserveOne, once there is a shop running it | ObserveOne | n/a |

Those three are not alternatives to pick between. ObserveOne is the tool the
product is committed to and it records the flows that matter; Playwright is
what a session can run locally right now against the web UI, and it stops
being needed once ObserveOne covers the same screens; tauri-driver is the
only one of the three that drives the real Tauri window, so it stays in the
architecture matrix until ObserveOne can do that, and one of the two pages
gets corrected when it can.

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

## How to adopt

Steps 1 to 3 below are done: `packages/design` is wired in (D2) and
`apps/desktop` has eslint with the one rule above (D3). What is left of this
section is step 4, and the import rules in step 3's block, which are written
against a `features/` tree the app does not have yet: they would fail on
every screen today and they land with the wave that rewrites the screens.

**1. Take the dependency.**

```
pnpm --filter dzpos-desktop add @dzpos/design --workspace
```

Then generate the custom properties into a file `styles.css` imports, with a
one-line script that calls `toCss()` and writes it. Until that script exists,
`design/shared/tokens.css` is still the file the browser reads, and the test
in `packages/design` is what keeps the two identical.

**2. Install the linter.**

```
pnpm --filter dzpos-desktop add -D eslint typescript-eslint \
  eslint-plugin-import eslint-plugin-react-hooks
```

**3. Add `apps/desktop/eslint.config.js`.** The rules block that carries
this page:

```js
rules: {
  "import/order": ["warn", {
    groups: ["builtin", "external", "internal", "parent", "sibling", "index"],
    alphabetize: { order: "asc", caseInsensitive: true },
    "newlines-between": "always",
  }],
  "react-hooks/rules-of-hooks": "error",
  "react-hooks/exhaustive-deps": "warn",
  "no-restricted-imports": ["error", {
    patterns: [
      {
        group: ["@dzpos/design/primitives", "**/design/src/primitives"],
        message: "Tier 1 is not a component API. Import { theme } from '@dzpos/design', or use the CSS custom property.",
      },
      {
        group: ["@tauri-apps/api/core"],
        message: "Call the wrapper in src/lib/tauri.ts. Only it talks to the command layer.",
      },
    ],
    paths: [
      {
        name: "@tanstack/react-query",
        importNames: ["useQuery", "useMutation", "useSuspenseQuery"],
        message: "A screen uses a hook from features/<feature>/hooks. Only that folder calls TanStack directly.",
      },
    ],
  }],
},
```

Then two overrides, one per folder, because the two folders lose different
bans. Neither switches `no-restricted-imports` off: tier 1 stays banned in
a query hook and in `lib/` as well.

A query hook is the layer allowed to call TanStack. It still has no reason
to reach the Tauri command layer directly, so it keeps that ban:

```js
{
  files: ["src/features/*/hooks/**"],
  rules: {
    "no-restricted-imports": ["error", {
      patterns: [
        {
          group: ["@dzpos/design/primitives", "**/design/src/primitives"],
          message: "Tier 1 is not a component API. Import { theme } from '@dzpos/design', or use the CSS custom property.",
        },
        {
          group: ["@tauri-apps/api/core"],
          message: "Call the wrapper in src/lib/tauri.ts. Only it talks to the command layer.",
        },
      ],
    }],
  },
}
```

`src/lib/` holds the wrapper itself and the query client, so it loses both
the TanStack and the Tauri ban and keeps only the primitives one:

```js
{
  files: ["src/lib/**"],
  rules: {
    "no-restricted-imports": ["error", {
      patterns: [
        {
          group: ["@dzpos/design/primitives", "**/design/src/primitives"],
          message: "Tier 1 is not a component API. Import { theme } from '@dzpos/design', or use the CSS custom property.",
        },
      ],
    }],
  },
}
```

Add `"lint": "eslint src"` to `apps/desktop/package.json` and put it in the
quality-gate chain.

**4. Migrate the screens, one agent per screen.** The order this section
used to give (button, then keypad, then product tile) was written before
shadcn/ui was chosen; the button and the surfaces now come from the kit, so
what is left is the screens themselves, and the queue is the allowlist in
`apps/desktop/src/lint/allowlist.json`, longest file first.

Two components in the mockups have no shadcn equivalent and are still to be
drawn on the kit when their screen is rewritten: the **numeric keypad**
(`.keypad` and `.num` in `design/shared/components.css`, which pins
`--touch-min`, `--control-h-lg` and the figure font, and is the thing a
cashier touches most) and the **product tile** (`.ptile` in
`design/desktop/desktop.css`, which pins the radius scale, the card surface
and the shadow scale).

A screen is migrated when its line is out of the allowlist, it has no
literal colour and no literal pixel, its flows have their `data-testid`, and
every visible string goes through `t()`.

## Open

- `as const` on a token object is an `as`, and `coding-rules` bans `as`
  without an exception. The package is written without it, using explicit
  interfaces. Confirm that reading, or carve out `as const` in
  `coding-rules` and the eslint config can allow it.
- `design/shared/tokens.css` has a header comment naming
  `packages/shared/design/tokens.json` as the future home. The package
  landed at `packages/design`. Decide which name stands before either file
  is regenerated.
