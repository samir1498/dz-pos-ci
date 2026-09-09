# Browser e2e for the products screen

ObserveOne is the recorded e2e tool for this product (decision 2026-09-08);
Playwright here is the interim local driver.

## Run it

```
just e2e         # both tests
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
| `dzpos-api` | 4319 | `--db e2e/.artifacts/e2e.db`, deleted before every run |
| Vite | 5174 | started with `VITE_API_URL=http://127.0.0.1:4319` |

The database is thrown away, so the first assertion is always the empty
state. The API base URL reaches the app through `VITE_API_URL`, which
`src/api.ts` already reads; no source file knows about the test.

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

## Two things the screen does not do yet

- The add form has no TVA rate field. `rate_bps` is posted as `null`, so
  the test fills name, selling price and stock only.
- An empty name is caught by the form's own validator before any request
  goes out, so the visible message is `error_name_required` from the UI.
  The API's `validation` code has no path to this form; the test counts
  the POSTs to prove it.
