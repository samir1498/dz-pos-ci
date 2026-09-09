# Browser e2e for the products screen

ObserveOne is the recorded e2e tool for this product (decision 2026-09-08);
Playwright here is the interim local driver.

## Run it

```
just e2e         # all three tests
just screenshot  # only the test that writes screenshots/products.png
```

Either recipe is `pnpm desktop e2e` under the hood, so
`pnpm --filter dzpos-desktop exec playwright test --headed` works for a look
at what the browser is doing (needs a display, so run that on the laptop).

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
| `dzpos-api` | 4319 | `--db e2e/.artifacts/e2e.db`, deleted before every run; `DZPOS_API_TOKEN` set to a token made for the run |
| Vite | 5174 | started with `VITE_API_URL=http://127.0.0.1:4319` and the same token as `VITE_API_TOKEN` |

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

## Files

- `products.spec.ts`: the two tests. Expected strings are read from
  `src/i18n/fr.json` at runtime, so a reworded message fails the test
  instead of quietly passing.
- `screenshots/products.png`: committed, 1280x800, full page.
- `.artifacts/`: gitignored, holding the temp database and failure traces.

## What the suite checks and where

- The rate is checked twice: on the request body (`postedRates`,
  `putBodies`) and on the row the API answers with, which the table shows
  in its TVA column. A server storing 19 % for a posted 9 % fails the
  second check.
- The edit test proves the whole product is sent back (every field, the
  wholesale price and the low-stock threshold included) and that the row
  re-reads from the API, not from what was typed.
- An empty name is caught by the form's own validator before any request
  goes out, so the visible message is `error_name_required` from the UI.
  The API's `validation` code has no path to this form; the test counts
  the POSTs to prove it.
