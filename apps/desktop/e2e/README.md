# Browser e2e for the desktop screens

ObserveOne is the recorded e2e tool for this product (decision 2026-09-08);
Playwright here is the interim local driver.

## Run it

```
just e2e                          # the whole suite, in fr, then en, then ar
just screenshot                   # only the screenshot tests, in fr and ar
pnpm desktop e2e --project ar     # one language, both spec files
```

`just e2e` and `just screenshot` are loops in the `justfile`: each language
is a separate `pnpm desktop e2e --project <lang>` invocation, not three
Playwright projects passed to one invocation. See "Three languages" below
for why. `pnpm --filter dzpos-desktop exec playwright test --headed
--project fr` works for a look at what the browser is doing (needs a
display, so run that on the laptop).

Chromium is not in the repo. Once per machine:

```
pnpm --filter dzpos-desktop exec playwright install chromium
```

## What the run starts

`playwright.config.ts` brings up two servers and stops both when the run
ends. Neither reuses an already-running one, so `just api` and `just dev`
can stay up on their own ports while a suite runs.

| Server | Port | Notes |
| --- | --- | --- |
| `dzpos-api` | 4319, or `DZPOS_E2E_API_PORT` | `--db e2e/.artifacts/e2e.db`, deleted before every run; `DZPOS_API_TOKEN` set to a token made for the run |
| Vite | 5174, or `DZPOS_E2E_WEB_PORT` | started with `VITE_API_URL` pointing at the API port and the same token as `VITE_API_TOKEN` |

A second checkout on the same box (a worktree per task in the M1 loop)
passes its own port pair so two suites can run at once.

The database is thrown away, so the first assertion is always the empty
state. The API base URL reaches the app through `VITE_API_URL` and the
launch token through `VITE_API_TOKEN`, both of which `src/api.ts` already
reads; no source file knows about the test. The token is random per run,
the way the desktop makes one per launch, so the suite also proves the
browser is sending it.

The same variable serves the SSH case: when the browser runs on the
laptop and the API on the WSL box, start Vite with `VITE_API_URL` set to
the box's Tailscale address and port (`http://100.x.y.z:4317`), otherwise
the app falls back to `http://127.0.0.1:4317` on the laptop and finds
nothing there.

The API side of the same case is `--allow-origin`: the server names the
browser origins it answers (the dev Vite port and the Tauri ones), so a
browser on the laptop is refused until its origin is passed, one value,
`scheme://host[:port]` with no path: `just api 4317 .dev/dev.db
http://100.111.55.62:5173`. The e2e config passes its own Vite port the
same way.

The first run after a clean checkout compiles `dzpos-api`, which takes
minutes. The API webServer has a ten minute start timeout for that.

## Three languages

`playwright.config.ts` defines one project per UI language: `fr`, `en`,
`ar`. A project's `use.storageState` sets `dzpos-lang` in localStorage for
the app's origin before the page's first script runs, the way a person
would have it already chosen (`src/i18n/index.tsx`, `initialLang`); its
`use.locale` matches (`fr-FR`, `en-US`, `ar-DZ`). `products.spec.ts` and
`settings.spec.ts` read their expected strings from
`src/i18n/{fr,en,ar}.json` through `messages.ts`, keyed off
`test.info().project.name`, so the same test body runs three times with
three different words and still checks the real UI text, not a hardcoded
sentence.

All three projects share the one webServer pair (one API, one Vite) and
the one SQLite file `playwright.config.ts` deletes at the start of a run.
That is enough for one project; it is not enough for three in the same
invocation, because the products suite's first test asserts the table is
empty, and the second and third language to touch a shared database would
find rows the first one left. `just e2e` and `just screenshot` work around
this by looping: one `pnpm desktop e2e --project <lang>` per language, each
its own Playwright invocation with its own webServer lifecycle and so its
own deleted-then-fresh database. Running `pnpm desktop e2e` with no
`--project` executes all three projects in one invocation against one
shared database instead, and the empty-table assertion fails from the
second language on; use the looped `just e2e` or a single `--project`.

## Files

- `products.spec.ts`: three tests on the products screen;
  `settings.spec.ts`: two on the settings screen (store block, dated
  régime). `messages.ts` is the shared loader both use for
  `src/i18n/{fr,en,ar}.json`, keyed off the running Playwright project, so
  a reworded message fails the test instead of quietly passing.
- `screenshots/products.png` (fr), `screenshots/products-ar.png` and
  `screenshots/settings-ar.png` (ar): committed, 1280x800, full page. `en`
  keeps no screenshot; the two languages above are enough to show the
  layout and the RTL mirror.
- `.artifacts/`: gitignored, holding the temp database and failure traces.
  A failure's trace and its Playwright report name the project (the
  language) the failing test ran under.

## What the suite checks and where

- The rate is checked twice: on the request body (`postedRates`,
  `putBodies`) and on the row the API answers with, which the table shows
  in its TVA column. A server storing 19 % for a posted 9 % fails the
  second check.
- The edit test adds its own product, then proves the whole product is
  sent back (all eleven fields, `toEqual`) and that the row re-reads from
  the API, not from what was typed. The quantity rides along unchanged:
  the core ignores it on an update because the stock ledger owns it.
- An empty name is caught by the form's own validator before any request
  goes out, so the visible message is `error_name_required` from the UI.
  The API's `validation` code has no path to this form; the test counts
  the POSTs to prove it.
