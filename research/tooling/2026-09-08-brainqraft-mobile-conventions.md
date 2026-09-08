# brainqraft-mobile conventions (2026-09-08)

Read on Samir's Fedora laptop at `~/Developer/freelance/brainqraft-mobile`
over Tailscale SSH. It is a client project, so nothing here is code, brand
values, assets or strings. What follows is the shape of the project and the
rules it writes down, with the file each rule lives in, so dz-pos can take
the shape without taking the project.

Stack it runs on: Expo 53, React Native 0.79, React 19, Expo Router 5
(file-based), TanStack Query 5, React Hook Form + Zod, i18next, Jest
(`jest-expo`), Storybook on react-native-web, Maestro for e2e. No Tailwind,
no NativeWind, no styled-components, no cva.

## Folder structure

Rule, from `CLAUDE.md` "Project Structure": routing lives in `app/`,
cross-feature UI in `components/`, and everything owned by one feature in
`features/{feature}/`, with `components`, `screens`, `hooks`, `utils`,
`constants` and sometimes `context` and `data` under it.

```
app/                     routing only (Expo Router file tree)
components/              cross-feature UI, grouped by role
  buttons/ inputs/ indicators/ overlays/ layout/ ui/
context/                 React context providers
features/{feature}/
  screens/ components/ hooks/ utils/ constants/ context/ data/
hooks/                   app-wide hooks
lib/                     utilities, i18n init, query keys, validations
  __tests__/fixtures/
providers/               React providers (query client, etc.)
api-client/              generated from the backend OpenAPI spec, never edited
assets/images/{feature}/ artwork per feature, mirrors the features/ names
constants/               app constants
design/                  the token layer
locales/                 en.json, fr.json
```

Why it is worth keeping: a feature is one directory you can read end to
end, and a feature that goes away is one directory you delete. The 25
features carry between one and seven subfolders each, drawn from that same
short list of names.

Features do import each other, through the `@/features/{name}/{layer}` path
rather than a relative climb: the journey screen launches a game, the feed
mounts the game stack. What the project bans is narrower and is written
down in `CLAUDE.md`: "a game must not import another game's feedback
assets". Shared cues moved into a `shared/` folder under the feature that
owns the mechanism instead.

`components/ui/` is the generic layer (button, card, dialog, elevated
button). `components/buttons/`, `inputs/`, `indicators/`, `overlays/`,
`layout/` are the app-level shared pieces grouped by what they do, not by
which screen uses them. A component moves out of a feature into
`components/` the moment a second feature imports it.

Assets mirror the feature names: `assets/images/journey/`,
`assets/images/onboarding/`, `assets/images/settings/`. `CLAUDE.md` marks
one folder (`explore/tags/`) as an old export nothing imports any more,
which is the kind of note that only survives if the folder naming makes
the orphan obvious.

Naming, from the "File Naming Conventions" section of `CLAUDE.md`:

| Kind | Rule |
|---|---|
| Screen | `features/{feature}/screens/{ScreenName}.tsx` |
| Component | PascalCase file, `BackButton.tsx` |
| Hook | camelCase with `use` prefix, `useJourneys.ts` |
| Context | PascalCase with `Context` suffix |
| Test | `__tests__/` folder beside the source it tests |
| Story | beside the component, same basename, `.stories.tsx` |

## Design system layering

Three tiers in `design/`, generated from a Figma token export, plus a
barrel. From `.claude/skills/mobile-design-system/SKILL.md`:

| File | Tier | Holds |
|---|---|---|
| `design/primitives.ts` | 1 | raw colour ramps, one family per hue, steps 50 through 1000 with extra dark steps at 925 and 950. Marked "do not use directly". |
| `design/semantic.ts` | 2 | role tokens that reference primitives: `color`, `surface`, `textColor`, `borderColor`, `depth`, plus the mode-independent scales `spacing`, `radius`, `borderWidth`, `fontSize`, `lineHeight`, `fontFamily`, `letterSpacing`, and a `letterSpacingPx` helper |
| `design/theme.ts` | assembly | the single `theme` object, grouping the semantic exports under `colors`, `spacing`, `radius`, `borderWidth`, `fontSize`, `lineHeight`, `fontFamily`, `fontStyle`, `letterSpacing` |
| `design/index.ts` | barrel | re-exports `primitives`, everything semantic, `theme` and the `Theme` type |

The rule, quoted from the skill: "Never hardcode colors (or
spacing/radius/font sizes) in components." And on tier 1, quoted from
`primitives.ts`: "Do not reference these directly in components — consume
the semantic `theme` (design/theme.ts) instead."

Components import one thing, `import { theme } from "@/design"`. The skill
is explicit about why that import is a plain value and not a hook: "`theme`
is a **static import** (not a hook), so it works at module level inside
`StyleSheet.create({...})` too." That single sentence is what keeps the
whole app on one styling mechanism. A hook-shaped theme would force every
component into a function body and half of them would give up and inline a
hex.

Worth keeping: the tier-1 file carries a comment on every hand-added value
saying it is hand-added and why the Figma export will never produce it. So
a regeneration does not quietly delete it, and nobody adds a second one
next to it.

Also worth keeping: where a role once pointed at the wrong ramp step,
`semantic.ts` carries a comment beside that entry naming the step the
design file binds it to, saying which step an earlier export had, and
telling the next person to re-check the pairing after a regeneration. A
role that lands one step too dark renders every label in that role wrong
and nothing fails, so the note is the only thing that catches it twice.

Two gaps the skill names rather than papers over: shadow offsets exist as
Figma effect styles but the code has only the colours, and the 19-style
text ramp is reassembled by hand at each use site. Both are listed as "a
known gap to add", with the instruction to add them "when authorized, not
silently". Naming a gap in the skill file is better than a TODO in the
code, because the next person reads the skill before writing.

## Import rules

**Path aliases.** `tsconfig.json` sets `baseUrl: "."` and two aliases,
`@/*` and `~/*`, both mapping to the repo root. Every import in the docs
uses `@/`. Two aliases for one target is a wart, see "what not to copy".

**Import order.** `eslint.config.js` sets `import/order` to `warn` with
groups `builtin, external, internal, parent, sibling, index`,
`alphabetize: { order: "asc", caseInsensitive: true }` and
`"newlines-between": "always"`. Worth keeping because it makes an import
block reviewable without thinking, and it survives a merge.

**Never call the analytics SDK.** From `CLAUDE.md` under Analytics:
"**Always use `useTrackEvent()` (`hooks/useEventTracking.ts`)** — never
call `posthog.capture` or `appsFlyer.logEvent` directly." One hook fans out
to two sinks and applies the opt-out flag. The skill
`.claude/skills/track-events/SKILL.md` repeats it as the first line. Worth
keeping as a shape: any cross-cutting side effect gets one call site, and
the rule that says so is written where the person adding the second call
site will read it.

**Never hardcode a Figma pixel.** From `~/Developer/freelance/AGENTS.md`,
one level above the project, under Hard Rules: "never copy a raw Figma
pixel (e.g. `55`, `231`, `32`) into StyleSheet. Use `theme.spacing` /
`theme.radius` tokens, `%`, `flex`, or `gap`. If a Figma value has no
token, use the closest token or a percentage of the container — do not
invent a one-off number." Worth keeping because it turns a judgement call
into a lookup, and because it makes the design system's coverage gaps
visible instead of absorbing them into one-off numbers.

**Never write a query key as a literal.** From `CLAUDE.md`: keys come from
a factory in `lib/queryKeys.ts`. The trap it documents is the reason:
`getQueryData` matches keys by exact hash while `invalidateQueries` matches
by prefix, so a stray extra element breaks cache reads while invalidation
keeps working, and nothing fails loudly. Worth keeping verbatim; dz-pos
will hit the same edge the first time a query key grows a filter.

**A feature does not import another feature's private assets.** See the
folder structure section. The rule is not "features never import each
other", which the code does not follow; it is that a feature's internal
assets are not another feature's to reach for, and the shared ones get
promoted to a `shared/` folder first.

**Selectors at module scope.** From the journey hooks section: "**Selectors
must be declared at module scope.** An inline arrow gets a fresh identity
every render, which defeats TanStack's memoization of the selected result."
Worth keeping because it is a real re-render bug with no visible symptom
until a list gets long.

**No native folders.** `AGENTS.md`: "DO NOT TOUCH `brainqraft-mobile/android/`
— generated, not source." Anything native goes in `app.json` or `plugins/`.
The dz-pos parallel is `src-tauri/gen` and anything Expo prebuild writes.

## Styling rules

React Native `StyleSheet` or inline styles, fed by `theme`. The skill opens
by ruling the alternatives out by name so a stale doc cannot revive them:
"There is **no NativeWind, Tailwind, cva, or styled-components** — ignore
any older docs that say otherwise."

Colours, spacing, radius, border width, font size, line height and font
family all come from `theme.*`. Two conversions are documented at the use
site rather than baked into the token:

- `letterSpacing` tokens are Figma percentages; React Native wants pixels,
  so `letterSpacingPx(percent, fontSize)` converts at the call. The token
  stays faithful to the design file and the conversion is visible.
- `fontFamily` is a family name string, not a numeric weight, because the
  app loads named faces. On Android the family alone does not select the
  weight, so `semantic.ts` also exports pre-built `fontStyle` objects that
  pair family with weight and a scaled line height. Comment in the file:
  "on Android, fontFamily alone doesn't select the weight; fontWeight must
  be paired with it."

Line heights are 1:1 with font size in the token scale because that is what
Figma says, and the skill says so plainly rather than fixing it quietly:
"line-heights equal their font-size (ratio 1.0, tight) — faithful to
Figma. If body text feels cramped, that's why." Worth keeping the habit:
when the token is inconvenient but correct, document the inconvenience.

The Figma-to-code pipeline lives in
`.claude/skills/figma-to-code/SKILL.md` as four ordered stages: plan (a
colocated map of code component to Figma node), pre-flight (flag Figma
variables bound to a stale remote library before they leak into code),
implement (tokens only, reuse existing components), verify (screenshot the
running screen against the Figma render). Its stated reason for existing is
that design to code was landing around 70% fidelity, and both causes were
mechanical: values eyeballed from a screenshot, and Figma values hardcoded
instead of mapped.

## Testing setup

Three layers, three tools, each with its own rule sheet.

**Unit, Jest with the `jest-expo` preset.** Config sits in `package.json`
under `"jest"`. `testMatch` is `**/__tests__/**/*.test.ts?(x)`,
`moduleNameMapper` maps `@/`, `setupFilesAfterEach` points at a
`jest.setup.ts` that mocks Reanimated and Gesture Handler.
`collectCoverageFrom` is a deliberate list (`lib/`, `hooks/`, `context/`,
`features/**/utils/`) with `__tests__/**` excluded so the fixtures file
stops counting itself. The skill
`.claude/skills/unit-test-suite/SKILL.md` is about writing suites that
catch regressions, and the parts worth carrying:

- Tests live in `__tests__/` beside their source. Shared fixtures live in
  one file and are reused, never redeclared in a second suite.
- Read the whole source first: "The `NOTE:` comments and JSDoc in this repo
  document real invariants... Those comments are the test list."
- One `describe` per exported function, preceded by a two to six line
  contract header saying what the function does, "a contract a reviewer can
  read instead of the implementation".
- Cover in order: happy path, edge cases, corrupt input, reference
  contracts (`toBe` where a function must return its argument unchanged),
  non-mutation of the input.
- Pin product constants literally. `expect(DAILY_FLOOR).toBe(20)` so an
  economy change shows in the diff, not `expect(state.balance).toBe(DAILY_FLOOR)`
  which passes whatever the constant becomes. This is the same discipline
  dz-pos already applies to fiscal fixtures.
- Seed `Math.random` with a spy, then fuzz over roughly 250 seeds. The skill
  records a real catch: a guard removal was caught by the fuzz test only,
  every fixed seed passed.
- The mutation pass is not optional. Break the source three ways one at a
  time, record which tests fail, restore, and finish with an empty
  `diff --stat`. A surviving mutation is either a real gap or an equivalent
  mutant, and the two get different treatment.

**Storybook on the web**, via `@storybook/react-native-web-vite`.
`.claude/skills/storybook-stories/SKILL.md` scopes it: stories exist for
`components/**` only, never for `features/**`, because features pull in
navigation, contexts and server state. Stories are colocated with the same
basename. Titles are `<Namespace>/<ComponentName>` mapped from the folder.
Two stubs live in `.storybook/mocks/` for the router and one data hook, and
the preview file wraps every story in the same providers the app uses so
`t()` resolves. The skill's table of what `.storybook/main.ts` has to
re-create (SVG transform, path aliases, the Reanimated babel plugin)
because Vite is not Metro is the useful part: a second bundler means a
second copy of three build decisions, and drift between them is the whole
maintenance cost.

**End-to-end, Maestro.** Flows are YAML in `.maestro/`, run against a real
build, with test credentials sourced from a git-ignored `.maestro/.env` and
forwarded with `-e`. `.claude/skills/maestro-e2e/SKILL.md` is written in
French and is mostly a list of four traps that block a flow, which is the
right shape for an e2e skill:

1. A wrapping `Pressable` sets `accessible={true}` by default, which on iOS
   collapses its whole subtree into one accessibility element, so nothing
   under it is visible to the runner. The fix is `accessible={false}` on
   the full-screen dismiss wrapper, which changes no behaviour.
2. A React Native `Modal` lives in its own window on iOS, so whatever it
   covers leaves the hierarchy. A conditional bottom sheet therefore has to
   be dismissed inside a bounded `repeat`/`while` loop rather than assumed.
3. Auto-advancing OTP fields need one digit typed per box, because the
   focus jump is async and a whole string outruns it.
4. A disabled button is found, tapped, and the step goes green, but
   `onPress` never fires. The failure surfaces two steps later. "Quand un
   `extendedWaitUntil` échoue sur l'écran suivant, suspecter d'abord un
   bouton resté désactivé."

Selectors are always `testID`, never visible text, because labels go
through i18n and break with the device locale. The naming is
`<feature>-*`, and the README carries a table of every id a flow depends
on. Worth keeping in full for dz-pos: the app is trilingual, so a
text-based selector is guaranteed to break.

## Scripts and gates

From `package.json`:

| Script | Does |
|---|---|
| `lint` | `expo lint` (ESLint flat config) |
| `format` | `prettier --write "**/*.{ts,tsx,md}"` |
| `test`, `test:watch`, `test:coverage` | Jest |
| `storybook`, `build-storybook` | Storybook dev server and static build |
| `e2e`, `e2e:auth` | Maestro, sourcing `.maestro/.env` |
| `generate-client` | regenerate the OpenAPI client from the backend spec |
| `i18n:extract` | i18next-parser |
| `prepare` | `husky` |

`lint-staged` runs `eslint --fix` then `prettier --write` on staged
`ts,tsx,js,jsx`, and `prettier --write` on staged `json,md`. Husky wires
it. `commitlint.config.js` extends `@commitlint/config-conventional` with a
100-character header and body line cap, lower-case type and scope, and an
extended `type-enum` that adds `translation`, `security` and `changeset` to
the conventional list. dz-pos already has a commit-msg hook for the `ctx:`
trailer; the line-length caps are the part worth borrowing.

Lint and format dependencies: `eslint` 9 flat config, `eslint-config-expo`,
`@typescript-eslint/parser` and plugin, `eslint-plugin-import`,
`eslint-plugin-react`, `eslint-plugin-react-hooks`,
`eslint-plugin-react-compiler`, `eslint-plugin-prettier`,
`eslint-config-prettier`, `prettier`, `husky`, `lint-staged`,
`@commitlint/cli` and `@commitlint/config-conventional`.

The ESLint rules it actually sets, on top of the Expo config:
`prettier/prettier` warn, `react-hooks/rules-of-hooks` error,
`react-hooks/exhaustive-deps` warn, the `import/order` block above,
`react/jsx-filename-extension` limited to `.tsx`,
`react/react-in-jsx-scope` off, `import/prefer-default-export` off, and the
react-compiler recommended config appended.

There is no `no-restricted-imports` rule. The "never import primitives
directly" and "never call the analytics SDK directly" rules are prose in
`CLAUDE.md` and the skills, enforced by review. That is the one place
dz-pos should do better than the source: both rules are exactly what
`no-restricted-imports` exists for.

## What not to copy

Client-specific or otherwise wrong for dz-pos.

- **Every colour value, ramp name and role assignment.** The hues, the
  brand roles, the quest and game accent colours, the hand-added backdrop
  scalar: all client brand. dz-pos has its own in `design/shared/tokens.css`.
- **Fonts and font logic.** The loaded families are the client's. The
  Android weight-pairing trick and the rounded-substitute reasoning are the
  transferable part.
- **`Platform.OS` inside `semantic.ts`.** It couples the token layer to
  React Native, which is fine for an app and wrong for a shared package
  that a Vite desktop build also imports. Keep the platform branch in
  `apps/mobile`, not in `packages/design`.
- **Dark-only, single mode.** The token layer has no light mode and no
  theme switch. dz-pos is a light-first point-of-sale used under shop
  lighting, and its tokens are already written that way.
- **The generated-file header comments.** `// AUTO-GENERATED from Figma
  export` is a lie in a hand-written file. dz-pos has no Figma pipeline;
  the source of the values is `design/shared/tokens.css` and the file
  should say so.
- **`components/ui/colors.ts`**, a shadcn slot-name adapter over `theme`.
  It exists because the generic primitives arrived from a shadcn-shaped
  library. dz-pos has no such import, so a second naming scheme over the
  same tokens would be pure cost.
- **PostHog and AppsFlyer, and the whole analytics section.** dz-pos is
  offline-first and sells to shops that will not accept a product
  analytics SDK. The transferable rule is the shape: one hook, never the
  SDK.
- **The `~/*` alias.** Two aliases pointing at the same root. Pick `@/` and
  stop.
- **Expo Router's `app/` tree and the `(authenticated)/(tab)` grouping.**
  The desktop is TanStack Router with a `routes/` directory, and the mobile
  app's own routing is not decided yet.
- **`i18next-parser` with `keepRemoved: false`.** The skill itself warns it
  deletes any key it cannot see in the code, which kills dynamically built
  keys. dz-pos has three locales and fiscal strings; hand-edit the JSON.
- **The OpenAPI client generation.** dz-pos generates its shared types from
  Rust with ts-rs into `packages/shared`, which is already decided in
  `docs/architecture.md`.
- **Skills written in French.** Two of the five read are French. The dz-pos
  skills are English.
- **The per-game JSDoc template** (eleven required sections in a component
  header). It fits a codebase of 50 game components with intricate gesture
  logic. dz-pos's coding rules cap comments at three lines.

## What dz-pos takes

Deliverable 2, `context/processes/20260908-frontend-conventions.md`, is the
dz-pos version: the folder shape for `apps/desktop` and `apps/mobile`, the
three-tier `packages/design` with the CSS custom properties generated from
the same source as the TypeScript theme, the import restrictions as ESLint
rules rather than prose, and the testing table.
