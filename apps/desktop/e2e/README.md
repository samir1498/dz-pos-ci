# Browser e2e for the desktop screens

ObserveOne is the recorded e2e tool for this product (decision 2026-09-08);
Playwright here is the interim local driver.

## Run it

```
just e2e                          # the whole suite, in fr, then en, then ar
just screenshot                   # writes thirteen of the fourteen screenshots
pnpm desktop e2e --project ar     # one language, every spec file
```

`just screenshot` runs `-g screenshot` (the tests with "screenshot" in
their title) under `--project fr` then `--project ar`. Under fr that
writes only `products.png`; the other twelve say "... screenshot(s) in Arabic"
in their titles, so they match the grep in both runs but write a file only
when `currentLang()` is `ar`, and the fr run of them does nothing
observable. Under ar all twelve write: `products-ar.png`, `settings-ar.png`,
`till-ar.png`, `customers-ar.png`, `suppliers-ar.png`, `purchases-ar.png`,
`expenses-ar.png`, `till-credit-ar.png`, `documents-avoir-ar.png`,
`stock-recount-ar.png`, and the theme pair `theme-comptoir-ar.png` and
`theme-observe-ar.png`.

`product-label-ar.png` is the fourteenth and the recipe cannot take it: its
test needs the import test above it in the same file to have run, and
`-g screenshot` picks tests, not files, so under the grep the product it
photographs does not exist. It is written by a full `just e2e --project ar`
instead. That is also why its picture carries the products every other spec
left behind, and why retaking it means running the whole suite.

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

A second checkout on the same box (a worktree per task) passes its own
port pair so two suites can run at once.

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
`use.locale` matches (`fr-FR`, `en-US`, `ar-DZ`). The spec files read
their expected strings from
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

- The spec files, in the order one run walks them (files run in name
  order, one worker, one database, so a file can leave the shop in the
  state the next one starts from):
  `backups.spec.ts`, one test that a copy taken before a product is added
  loses that product when it is restored, leaving the shop empty;
  `customers.spec.ts`, a company fiche opened with an opening debt and then
  corrected, plus a search by a piece of the name;
  `expenses.spec.ts`, two expenses filed in two categories, the month's
  total and the cash position of that month read back off the API, and an
  empty month answering zeros;
  `first-paint.spec.ts`, that `lang` and `dir` are right on
  `documentElement` before React mounts, read off the DOM rather than
  through an auto-retrying matcher;
  `products.spec.ts`, three tests on the products screen;
  `purchases.spec.ts`, an order of two products with extra costs received in
  two deliveries and then partly returned, and a cancellation refused once
  goods have arrived;
  `settings.spec.ts`, the store block and the dated régime, and it hands
  the shop back under the réel before it leaves;
  `settlement.spec.ts`, two credit sales settled oldest first, a payment
  above the debt refused, and the statement printed;
  `stock-recount.spec.ts`, a product opened with stock and sold from, then
  the recount run from the settings panel finding nothing to correct;
  `suppliers.spec.ts`, a supplier fiche opened with an opening debt, money
  paid against it and the movements it leaves;
  `theme.spec.ts`, each of the four themes chosen, kept in the shop file and
  still there after a reload, with the painted background read off the
  document element rather than only the attribute; it hands the shop back to
  "follow the machine" before it leaves, the way the settings spec hands the
  régime back, because it runs before the till specs and their screenshots
  are taken in the light theme;
  `till.spec.ts`, one whole cash sale from `/` landing on the till to the
  stock it moved;
  `till-credit.spec.ts`, a credit sale warned at the threshold, refused
  past the limit and then overridden;
  `till-facture.spec.ts`, a facture on credit that leaves the ticket series
  alone, and a facture paid in cash carrying its TVA recap and its droit de
  timbre;
  `till-reversals-and-quotations.spec.ts`, a partial avoir, a whole
  cancellation, the credit it leaves and a proforma;
  `zz-exports-and-labels.spec.ts`, the four workbooks read back as real
  spreadsheets, the product template downloaded, the committed fixture
  checked and applied, and the label of the product it created shown in the
  sandboxed frame. Named to run last: it creates a product out of a file,
  and every spec that starts from an empty catalogue has to have run first.
- `messages.ts` is the shared loader for `src/i18n/{fr,en,ar}.json`, keyed
  off the running Playwright project, so a reworded message fails the test
  instead of quietly passing. `api.ts` is where a spec that seeds its own
  rows finds the API port and the run's launch token.
- The fourteen committed screenshots, 1280x800, full page: `products.png`
  (fr) and, in ar, `products-ar.png`, `settings-ar.png`, `till-ar.png`,
  `customers-ar.png`, `suppliers-ar.png`, `purchases-ar.png`,
  `expenses-ar.png`, `till-credit-ar.png`, `documents-avoir-ar.png`,
  `stock-recount-ar.png`, `theme-comptoir-ar.png` and
  `theme-observe-ar.png`, plus `product-label-ar.png`, which only a full run
  writes. The theme pair is the settings screen in the light theme and in
  Observe, which is what a theme is for: the same screen, two palettes, one
  stylesheet.
  `en` keeps none; the two languages above are enough to show the layout
  and the RTL mirror.
- `.artifacts/`: gitignored, holding the temp database and failure traces.
  A failure's trace and its Playwright report name the project (the
  language) the failing test ran under.

## The test ids

`frontend-conventions` asks for `data-testid` rather than visible text
where a flow needs a handle, and for every id a flow uses to be listed
here. These are all of them.

| Id | Where | Why not a text or role selector |
| --- | --- | --- |
| `regime-current` | `settings.tsx` | Two régime lines can read the same words; this one is the one in force. |
| `regime-planned` | `settings.tsx` | Absent until a change is dated ahead, so the test counts it. |
| `tiles` | `till.tsx` | The cart's −, + and ✕ buttons carry the product name too, so a name query without this scope matches four things. |
| `cart` | `till.tsx` | The mirror image of the above, for a query that means the lines rather than the grid. |
| `till-change` | `till.tsx` | An amount, so its text is a number in three locales. |
| `total-net-to-pay` | `till.tsx` | The one totals row a test reads by value; the label beside it is translated. |
| `till-customer-balance` | `till.tsx` | An amount, and the same figure appears on the fiche panel beside it. |
| `till-credit-limit` | `till.tsx` | An amount, translated label. |
| `till-near-limit` | `till.tsx` | Absent until the basket crosses the warning threshold, so a test counts it. |
| `till-limit-banner` | `till.tsx` | Absent until the limit refuses the sale. |
| `till-new-balance` | `till.tsx` | An amount the basket would leave behind. |
| `till-balance-after` | `till.tsx` | An amount the issued document left behind. |
| `till-document-number` | `till.tsx` | The number as the paper spells it, inside a translated confirmation. |
| `till-party-ids` | `till.tsx` | The buyer identifiers block; absent until a customer is picked. |
| `till-party-ids-missing` | `till.tsx` | Absent until an identifier a facture needs is missing. |
| `documents-sheet` | `documents.tsx` | The iframe holding the page the core rendered; an iframe has no accessible text. |
| `customer-payment` | `customers.tsx` | One payment row; the amounts inside it are numbers in three locales. |
| `customer-statement` | `customers.tsx` | The statement iframe; same reason as the sheet above. |
| `customer-debt-slip` | `customers.tsx` | The debt slip iframe. |
| `customer-debt-slip-button` | `customers.tsx` | The button that opens it, beside a second button with a translated label. |
| `customer-close-reason` | `customers.tsx` | Absent until a fiche with an account behind it is being closed, so a test counts it. |
| `purchase-status` | `purchases_.$id.tsx` | The state the order is in; its word and a column header of the lines table read the same in English ("Received"). |
| `expenses-month` | `expenses.tsx` | The month picker; its rendered text is the browser's own, in the browser's locale. |
| `expenses-total` | `expenses.tsx` | An amount, so its text is a number in three locales. |
| `expense-row` | `expenses.tsx` | One expense; the amounts inside it are numbers in three locales. |
| `expense-category` | `expenses.tsx` | A `<select>` inside a wrapping `<label>` takes the option texts into its own accessible name, so a label query matches nothing. |
| `expense-amount` | `expenses.tsx` | An amount field, translated label, and `col_amount` reads the same word. |
| `expense-date` | `expenses.tsx` | A date field, and `col_date` reads the same word. |
| `expense-note` | `expenses.tsx` | A note field, and `col_note` reads the same word. |
| `cash-position` | `expenses.tsx` | The figures panel; every figure inside it is a number in three locales. |
| `cash-in-total` | `expenses.tsx` | An amount, and both sides carry a row labelled "total". |
| `cash-out-expenses` | `expenses.tsx` | An amount, and the same word labels the screen itself. |
| `cash-out-total` | `expenses.tsx` | The other row labelled "total". |
| `cash-net` | `expenses.tsx` | An amount, and the one figure that goes below zero. |
| `card-in-total` | `expenses.tsx` | An amount, translated label. |
| `backup-row` | `BackupsPanel.tsx` | One copy in the list; its text is a filename and a date. |
| `backups-newest` | `BackupsPanel.tsx` | The one the restore button acts on. |
| `safety-copy-row` | `BackupsPanel.tsx` | The copy a restore takes of what it is about to replace. |
| `theme-switcher` | `ThemeSwitcher.tsx` | The select is rendered twice (topbar and settings panel) with the same translated label, so a label query matches two things. |
| `stock-recount-day` | `StockRecountPanel.tsx` | A day, or the translated sentence for a shop that has never recounted. |
| `stock-recount-checked` | `StockRecountPanel.tsx` | Absent until a run answers, so a test counts it; its text is a bare number. |
| `stock-recount-clean` | `StockRecountPanel.tsx` | Absent until a run has happened, and the sentence it holds is translated. |
| `stock-drift-row` | `StockRecountPanel.tsx` | One corrected product; the quantities inside it are numbers in three locales. |
| `stock-drift-difference` | `StockRecountPanel.tsx` | The signed quantity, beside two other quantities on the same row. |

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
- The till's expectation is a money fixture, not the screen: `till.spec.ts`
  reads the `till_cash_sale_two_rates` case out of
  `fixtures/money/tva_rounding_once_per_rate.json` and compares it column
  for column with the totals of the issued `SaleDto`, so a screen with a
  wrong preview and an API that agreed with it would still fail. It states
  the réel régime itself rather than depending on the settings suite having
  run before it, because a ticket under the IFU carries no TVA row.
