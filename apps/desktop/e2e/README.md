# Browser e2e for the desktop screens

ObserveOne is the recorded e2e tool for this product (decision 2026-09-08);
Playwright here is the interim local driver.

Every route sits behind a sign-in screen since M4 T4 (PIN pad or password,
`src/components/SignInScreen.tsx`). Every spec but `signin.spec.ts` gets
past it through `./auth`, which signs the fixed development owner in over
the API before a test's `context` or `request` fixture is handed to it;
`globalSetup.ts` is what makes that owner's credential real on a fresh
database. See "Files" below for both.

## Run it

```
just e2e                          # the whole suite, in fr, then en, then ar
just screenshot                   # writes twenty-five of the twenty-six screenshots
pnpm desktop e2e --project ar     # one language, every spec file
```

`just screenshot` runs `-g screenshot` (the tests with "screenshot" in
their title) under `--project fr` then `--project ar`. Under fr that
writes `products.png`, `customers.png`, `dashboard.png`, `till.png`,
`till-credit.png`, `documents-avoir.png` and the kit's four; the other six
titles either say "... screenshot(s) in Arabic" or, for `purchases.spec.ts`
and `suppliers.spec.ts`, write an Arabic-only file without saying so, so
they match the grep in both runs but write a file only when `currentLang()`
is `ar`, and the fr run of them does nothing observable. Under ar all
fifteen write: `products-ar.png`, `dashboard-ar.png`, `settings-ar.png`,
`till-ar.png`, `customers-ar.png`, `suppliers-ar.png`,
`supplier-statement-ar.png`, `purchases-ar.png`, `expenses-ar.png`,
`till-credit-ar.png`, `documents-avoir-ar.png`, `stock-recount-ar.png`,
and the theme pair `theme-comptoir-ar.png` and `theme-observe-ar.png`.
The customers test writes two of them: the list and, before it walks back
to it, `customer-account-ar.png`, the page one row's name opens; the
suppliers test does the same with `supplier-statement-ar.png`.

`products.spec.ts`, `customers.spec.ts`, `zzz-dashboard.spec.ts`,
`till.spec.ts`, `till-credit.spec.ts` and
`till-reversals-and-quotations.spec.ts` are the six that write under both
languages, so their titles carry no "in Arabic": the products screen is
the reference shot of a list, the dashboard is the one screen whose whole
point is a picture, and the till, the credit sale and the documents list
each now show a French visitor the real screen instead of the Arabic one.

The kit's four (`kit-comptoir.png`, `kit-registre.png`, `kit-observe.png`,
`kit-observe-dark.png`) are the exception to that pattern: they are written
under fr and skipped in the other two projects, because the page they
photograph is the component kit rather than a screen and its point is the
four themes, not the three languages. One picture per theme, from
`kit.spec.ts`, of the `/kit` page, which exists in a dev build only.

`product-label-ar.png` is the sixteenth Arabic one and the recipe cannot take it: its
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
  `customers.spec.ts`, a company fiche opened with an opening debt in the
  panel over the list, corrected on the customer's own account page, and read
  back on the row after the walk back, plus a search by a piece of the name;
  `expenses.spec.ts`, two expenses filed in two categories, the month's
  total and the cash position of that month read back off the API, and an
  empty month answering zeros;
  `first-paint.spec.ts`, that `lang` and `dir` are right on
  `documentElement` before React mounts, read off the DOM rather than
  through an auto-retrying matcher;
  `launch-token.spec.ts`, that a token planted in the page global is not
  honoured (M5 T0 moved the launch token out of the page): every request
  carries `VITE_API_TOKEN`, and a wrong planted value 401s even sign-in;
  `products.spec.ts`, three tests on the products screen;
  `purchases.spec.ts`, an order of two products with extra costs received in
  two deliveries and then partly returned, and a cancellation refused once
  goods have arrived;
  `settings.spec.ts`, the store block and the dated régime, and it hands
  the shop back under the réel before it leaves;
  `settlement.spec.ts`, two credit sales settled oldest first, a payment
  above the debt refused, and the statement printed;
  `signin.spec.ts`, the sign-in screen itself, signed out on purpose (plain
  `@playwright/test`, not `./auth`): the PIN pad's two stages with a wrong
  PIN refused and the right one signing the owner in, backspace on an
  empty PIN returning to the id stage, the password form the same way, and
  a locked-out wait counted down on screen from a `retry_after_seconds` an
  intercepted `/auth/login` answers, never the real owner's row (five
  wrong PINs against it would lock it out for the rest of the run);
  `stock-recount.spec.ts`, a product opened with stock and sold from, then
  the recount run from the settings panel finding nothing to correct;
  `suppliers.spec.ts`, a supplier fiche opened with an opening debt, money
  paid against it and the movements it leaves;
  `theme.spec.ts`, each of the four themes chosen, kept in the shop file and
  still there after a reload, with the painted background read off the
  document element rather than only the attribute; it hands the shop back to
  Comptoir, the default, before it leaves, the way the settings spec hands the
  régime back, because it runs before the till specs and their screenshots
  are taken in the light theme;
  `till-cashier.spec.ts`, a cashier signed in on the till's own session
  (M4 T9), driving the refusals the permission table promises through the
  real API rather than a mock: a sale rung up (the one thing every role
  may do), a discount above the threshold and a typed-under price both
  refused by name, a credit sale past the limit refused and overridden,
  the staff list and the audit log refused at the server and not merely
  hidden from the sidebar, purchases and the dashboard the same way, the
  product list's cost fields redacted rather than closed, and every route
  carrying the shop's money refused in turn;
  `till-credit.spec.ts`, a credit sale warned at the threshold, refused
  past the limit and then overridden;
  `till-facture.spec.ts`, a facture on credit that leaves the ticket series
  alone, and a facture paid in cash carrying its TVA recap and its droit de
  timbre;
  `till-lock.spec.ts`, the brief's own proof for M4 T4, in three parts: a
  product scanned into the cart survives being locked from the topbar's
  user menu and read back after unlocking with the owner's password
  (what a session resumed from the `./auth` fixture's cookie unlocks with,
  no `AuthMethod` remembered on a window that never made a sign-in call of
  its own); the covered till answers to nothing while it is locked, a
  forced `.focus()` on the search box is a no-op, a scanned barcode lands
  nowhere, and F9 does not pay, even though the basket it would have paid
  is a real one; and a session with no remembered method offers both
  unlock doors rather than stranding a PIN-only cashier behind the
  password form;
  `till-reversals-and-quotations.spec.ts`, a partial avoir, a whole
  cancellation, the credit it leaves and a proforma;
  `till.spec.ts`, one whole cash sale from `/` landing on the till to the
  stock it moved;
  `zz-exports-and-labels.spec.ts`, the four workbooks read back as real
  spreadsheets, the product template downloaded, the committed fixture
  checked and applied, and the label of the product it created shown in the
  sandboxed frame. Named to run last among the `zz` files: it creates a
  product out of a file, and every spec that starts from an empty catalogue
  has to have run first;
  `zzz-dashboard.spec.ts`, a shop seeded with a catalogue, a day of trading,
  both debts, an order still open and a month of expenses, read back through
  `GET /dashboard` and `GET /dashboard/series` and compared figure by figure
  with what the screen paints. Named to run after everything: it fills the
  shop, and the specs above it start from an empty catalogue and an empty
  month. It seeds everything it reads rather than living off what they left,
  because `just screenshot` runs it alone against a database that was just
  deleted.
- `scanner.spec.ts`, every wedge-scanner behaviour reachable without a
  scanner on the desk: the burst rule, the suffix key each model sends after
  the code, the wrong box having focus, and an AZERTY layout driven through
  CDP. What a real head does is the one row of the matrix that waits for
  Samir's own hardware.
- `zzzz-facture-layouts.spec.ts`, the sentence a shop owner would say: pick a
  layout in settings, open a facture, and the page is the one that was
  picked. That each layout draws what décret 05-468 art. 3 asks for is
  covered by the goldens in `crates/core`, and the picker's own wiring beside
  the component; only the round trip is end to end. Named to run last for the
  same reason as the dashboard spec: it changes a shop-wide setting.
- `messages.ts` is the shared loader for `src/i18n/{fr,en,ar}.json`, keyed
  off the running Playwright project, so a reworded message fails the test
  instead of quietly passing. `api.ts` is where a spec that seeds its own
  rows finds the API port and the run's launch token.
- `auth.ts` is the door every spec but `signin.spec.ts` goes through: its
  `test` overrides both the `context` and the standalone `request`
  fixtures to sign the fixed development owner in through the API before
  either is handed to a test, so a same-site httpOnly cookie is already on
  the jar before the first `page.goto` and before the first seeding call a
  spec makes with `request`. `globalSetup.ts` is what makes that owner
  signable-in at all: a fresh e2e database seeds the owner with no usable
  credential (`pin_hash = '!unset'`, `password_hash = NULL`), so this
  writes a real argon2id hash for the fixed PIN and password onto that row
  once `webServer` has migrated `tempDb`, before any spec runs. The hash
  strings are not computed there; see the file's own header for where they
  came from and why `dzpos-seed` is not run against the real e2e database.
- `kit.spec.ts` drives the shell and the component kit: the sidebar reaching
  every screen with the topbar naming it, the sidebar's side read off its
  geometry rather than its class list, a narrow window folding it into a
  sheet, the theme switch in the topbar, and the `/kit` page with each
  overlay opened. It runs before the settings and theme specs, so it hands
  the shop back to Comptoir, the default, before it leaves. It writes the four
  kit screenshots under fr.
- The twenty-six committed screenshots, 1280x800, full page: `products.png`,
  `customers.png`, `dashboard.png`, `till.png`, `till-credit.png`,
  `documents-avoir.png` and the kit's four (`kit-comptoir.png`,
  `kit-registre.png`, `kit-observe.png`, `kit-observe-dark.png`) in fr; in ar,
  `products-ar.png`, `dashboard-ar.png`, `settings-ar.png`, `till-ar.png`,
  `customers-ar.png`, `customer-account-ar.png`, `suppliers-ar.png`,
  `supplier-statement-ar.png`, `purchases-ar.png`,
  `expenses-ar.png`, `till-credit-ar.png`, `documents-avoir-ar.png`,
  `stock-recount-ar.png`, `theme-comptoir-ar.png` and
  `theme-observe-ar.png`, plus `product-label-ar.png`, which only a full run
  writes. The theme pair is the settings screen in the light theme and in
  Observe, which is what a theme is for: the same screen, two palettes, one
  stylesheet.
  `en` keeps none; the two languages above are enough to show the layout
  and the RTL mirror. `till.png`, `till-credit.png` and `documents-avoir.png`
  exist so `apps/landing`'s generator (`src/lib/shots.ts`) has a French
  source for the hero and two of the four feature pieces instead of only
  the Arabic one.
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
| `keypad` | `Keypad.tsx` | The pad's keys are digits, which read the same in every language and appear again in the amounts around them. |
| `till-totals` | `-till/cart.tsx` | The totals block; a few named amounts rather than a table, so there is no role and name to ask for. |
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
| `till-override-dialog` | `till.tsx` | The ask before forcing a credit-blocked sale through; the spec confirms it rather than answering a browser box. |
| `documents-sheet` | `documents.tsx` | The iframe holding the page the core rendered; an iframe has no accessible text. |
| `customer-fiche` | `-customers/fiche.tsx` | The panel the form opens in; a test waits for it before it types into a field the list also has. |
| `customer-close-reason` | `-customers/fiche.tsx` | Absent until a fiche with an account behind it is being closed, so a test counts it. |
| `customer-balance` | `customers_.$id.tsx` | The card carrying what is owed; the same figure appears again in the ledger below it. |
| `customer-payment` | `customers_.$id.tsx` | One payment row; the amounts inside it are numbers in three locales. |
| `customer-pay-dialog` | `customers_.$id.tsx` | The dialog the payment is typed into, waited on before the amount field. |
| `customer-adjust-dialog` | `customers_.$id.tsx` | The ask before a debt correction is written; the spec confirms it rather than answering a browser box. |
| `customer-statement` | `customers_.$id.tsx` | The statement iframe; same reason as the sheet above. |
| `customer-debt-slip` | `customers_.$id.tsx` | The debt slip iframe. |
| `customer-debt-slip-button` | `customers_.$id.tsx` | The button that opens it, beside a second button with a translated label. |
| `purchase-status` | `purchases_.$id.tsx` | The state the order is in; its word and a column header of the lines table read the same in English ("Received"). |
| `expenses-month` | `expenses.tsx` | The month group: two number boxes plus the month's name from the dictionary, never the browser's own picker. |
| `expenses-month-month`, `expenses-month-year` | `expenses.tsx` | The two boxes a flow fills; one month input cannot be asserted per segment. |
| `expenses-total` | `expenses.tsx` | An amount, so its text is a number in three locales. |
| `expenses-add` | `expenses.tsx` | Opens the entry panel. The empty state offers a second button with the same words, so the name is ambiguous and the id is not. |
| `expenses-table` | `expenses.tsx` | The month's list, or the empty state in its place. The rows are the second `rowgroup` inside it; `DataTable` has no id of its own per row. |
| `expense-sheet` | `expenses.tsx` | The entry panel. |
| `expense-form` | `expenses.tsx` | The form inside the panel; a flow waits on it to disappear to know the row was filed. |
| `expense-category` | `expenses.tsx` | The category trigger. It opens a listbox rather than a native select, so the option is clicked by its own word once the trigger is open. |
| `expense-amount` | `expenses.tsx` | An amount field, translated label, and `col_amount` reads the same word. |
| `expense-date` | `expenses.tsx` | A date field, and `col_date` reads the same word. |
| `expense-note` | `expenses.tsx` | A note field, and `col_note` reads the same word. |
| `cash-position` | `expenses.tsx` | The figures panel; every figure inside it is a number in three locales. |
| `cash-in-total` | `expenses.tsx` | An amount, and both sides carry a row labelled "total". |
| `cash-out-expenses` | `expenses.tsx` | An amount, and the same word labels the screen itself. |
| `cash-out-total` | `expenses.tsx` | The other row labelled "total". |
| `cash-net` | `expenses.tsx` | An amount, and the one figure that goes below zero. |
| `card-in-total` | `expenses.tsx` | An amount, translated label. |
| `page-header` | `PageHeader.tsx` | The screen's own header block, so a count can be read without matching the same figure inside the list. |
| `products-search` | `products.tsx` | The search box; its label and its placeholder are translated and the placeholder repeats the barcode column's word. |
| `products-clear-filters` | `products.tsx` | Absent until a filter is set, so a test counts it, and it says the same words as the empty screen's own button. |
| `print-selected-labels` | `products.tsx` | Its label changes to the closing one once the sheet is open, so a name query matches only half the time. |
| `print-label` | `products.tsx` | Absent on a product being typed, so a test counts it; the sheet's button beside it reads almost the same. |
| `product-label` | `LabelPanel.tsx` | The iframe holding the page the core rendered; an iframe has no accessible text. |
| `import-table` | `ExportImportPanel.tsx` | The dry-run report, so its rows can be counted apart from the products list on the same screen. |
| `backups-table` | `BackupsPanel.tsx` | The list of copies. Its rows are read by role (the body is the second rowgroup); the table itself needs an id because the panel draws a second one under it. |
| `safety-copies-table` | `BackupsPanel.tsx` | The copies a restore took of what it replaced, same columns as the list above, so only the table tells them apart. |
| `backups-newest` | `BackupsPanel.tsx` | The date of the most recent copy, or the translated sentence for a shop that has none. |
| `backup-restore-dialog` | `BackupsPanel.tsx` | The second ask before a restore. The spec finds it by role; the id is there for a screenshot to point at. |
| `theme-switcher` | `ThemeSwitcher.tsx` | The select is rendered twice (topbar and settings panel) with the same translated label, so a label query matches two things. |
| `stock-recount-day` | `StockRecountPanel.tsx` | A day, or the translated sentence for a shop that has never recounted. |
| `stock-recount-checked` | `StockRecountPanel.tsx` | Absent until a run answers, so a test counts it; its text is a bare number. |
| `stock-recount-clean` | `StockRecountPanel.tsx` | Absent until a run has happened, and the sentence it holds is translated. |
| `stock-drift-table` | `StockRecountPanel.tsx` | The corrections the last run made. Absent when nothing was out, so a test counts it; its rows are read by role. |
| `stock-drift-difference` | `StockRecountPanel.tsx` | The signed quantity, beside two other quantities on the same row. |
| `shell-topbar` | `AppShell.tsx` | The bar itself, so a test can ask whether a switch is inside it rather than merely on the page. |
| `shell-title` | `AppShell.tsx` | The page's `h1`. Its text is the sidebar item's, so a role query by name would be circular. |
| `shell-day` | `AppShell.tsx` | The shop's day; absent until the server has answered, so a test counts it. |
| `shell-shop` | `AppShell.tsx` | The shop's name at the foot of the sidebar, where the user will stand. |
| `sidebar-trigger` | `sidebar.tsx` | The button that opens the sheet on a narrow window; its only text is off-screen and translated. |
| `nav-<screen>` | `AppShell.tsx` | One per sidebar item (`nav-till`, `nav-products`, …). The link's text is translated and repeats the topbar's. |
| `language-switcher` | `LanguageSwitcher.tsx` | The segmented control as a group; its three buttons name languages, not the group. |
| `figure-<name>` | `dashboard.tsx` | One figure card (`figure-sales`, `figure-margin`, `figure-expenses`, `figure-cash`, `figure-customer-debt`, `figure-supplier-debt`, `figure-open-purchases`). Every card is built out of the same words, so a text query matches four of them. |
| `figure-<name>-today` | `dashboard.tsx` | The day's amount on a card; an amount, so its text is a number in three locales. |
| `figure-<name>-month` | `dashboard.tsx` | The month's amount on the same card, which is the same shape of number a row above. |
| `figure-sales-count` | `dashboard.tsx` | The month's count of papers, a bare number beside two amounts. |
| `figure-<name>-total` | `dashboard.tsx` | What is owed, on the two debt cards. |
| `figure-<name>-parties` | `dashboard.tsx` | How many accounts are behind that debt; a bare number. |
| `dashboard-chart` | `dashboard.tsx` | The chart's box. It is an SVG recharts drew, with no accessible name of its own. Its `data-buckets` is how a test tells the day view from the week view: recharts draws a rectangle only for a bar with a height, so counting the bars counts the days the shop sold on rather than the days the chart covers. |
| `dashboard-chart-card` | `dashboard.tsx` | The card around it, so the legend can be looked for inside the chart rather than anywhere on the page. |
| `chart-grain-days` | `dashboard.tsx` | The day/week switch; both tabs are one translated word. |
| `chart-grain-weeks` | `dashboard.tsx` | As above. |
| `dashboard-low-stock` | `dashboard.tsx` | The low-stock card; its title is the same words as the pill inside it. |
| `low-stock-pill` | `dashboard.tsx` | One row's pill, for the same reason: `pill_low` and `dashboard_low_stock` read alike. |
| `dashboard-top-quantity` | `dashboard.tsx` | The busiest-products card; the two top tens carry the same columns and often the same products. |
| `dashboard-top-margin` | `dashboard.tsx` | The other one. |
| `kit-page` | `KitPage.tsx` | The dev-only kit page's root, waited on before the screenshots. |
| `kit-table` | `KitPage.tsx` | The kit's example table, so the money column can be measured without matching the empty one below it. |
| `kit-dialog-trigger` | `KitPage.tsx` | The four overlay triggers sit in one row with translated-looking French labels; ids keep the spec off their text. |
| `kit-sheet-trigger` | `KitPage.tsx` | As above. |
| `kit-menu-trigger` | `KitPage.tsx` | As above. |
| `kit-tooltip-trigger` | `KitPage.tsx` | As above. |
| `kit-toast-trigger` | `KitPage.tsx` | As above. |
| `signin-screen` | `SignInScreen.tsx` | The whole sign-in screen, so a test can wait for the door itself before reaching for a mode inside it. |
| `signin-pin-display` | `SignInScreen.tsx` | The typed id or PIN; masked in the PIN stage, so its own text cannot be asked for. |
| `signin-mode-switch` | `SignInScreen.tsx` | Toggles between the PIN pad and the password form; its own label changes with the mode. |
| `signin-name`, `signin-password` | `SignInScreen.tsx` | The password form's two fields. |
| `signin-submit` | `SignInScreen.tsx` | The password form's button; its translated label is "Sign in" like the screen's own title. |
| `signin-error`, `signin-retry` | `SignInScreen.tsx` | The refusal and the counted-down wait, mutually exclusive so a test reads whichever is on screen without a role query matching both a message and a countdown. |
| `lock-screen` | `LockScreen.tsx` | The overlay; a test asserts it covers the shell without asserting the shell is gone, which is the whole point of it. |
| `lock-pin-display`, `lock-password`, `lock-unlock` | `LockScreen.tsx` | The unlock control, one of a PIN pad or a password field depending on how the session was opened (`AuthMethod`). |
| `lock-mode-switch` | `LockScreen.tsx` | Only present when the session carries no remembered `AuthMethod` (resumed from a cookie): switches between the two unlock doors, so a cashier who only has a PIN is never stranded behind the password form. |
| `lock-error`, `lock-retry` | `LockScreen.tsx` | As `signin-error` / `signin-retry`, for the same reason. |
| `lock-switch-user` | `LockScreen.tsx` | Ends the session instead of unlocking it; a link beside a button, so a role query by name is not enough on its own. |
| `user-menu-trigger`, `user-menu` | `UserMenu.tsx` | The topbar's signed-in user; the trigger carries the name, which is data and not a translated word a test can ask for by text. |
| `user-menu-lock`, `user-menu-signout` | `UserMenu.tsx` | The two actions inside it. |

## Headings are queried inside `main`

The shell's topbar carries the page's `h1` and its text is the sidebar item's
name, which in six cases is the same word as the screen's own heading
("Produits", "Paramètres", "Clients", "Dépenses", "Documents",
"Fournisseurs"). Playwright's role queries match by substring, so an
unscoped `getByRole("heading", { name: t("products_title") })` finds two and
fails on the ambiguity rather than on anything being wrong. Those queries go
through `page.getByRole("main")`, which is the screen and not the frame.

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

## The screenshots are not byte-reproducible

Measured on 2026-09-11: reset every committed screenshot, run `just
screenshot` twice, compare the two runs against each other. Nine of the
nineteen files differ between two runs of *identical code*
(`customer-account-ar`, `dashboard`, the four `kit-*`, `purchases-ar`,
`supplier-statement-ar`, `theme-observe-ar`). The differences are tens of
bytes: font rasterisation and anti-aliasing, not layout.

So a byte diff on a screenshot is not evidence a screen changed, and two
agents plus one session have now spent real time treating it as if it were.
How to tell the two apart:

- **Tens or a few hundred bytes**: noise. Commit it or do not, it does not
  matter, but do not go looking for the change that caused it.
- **A kilobyte or more**: look at the picture. It is usually real. On
  2026-09-11 a `+1693` on `dashboard.png` was the audit log's new menu
  entry, and a `-24165` on `till.png` was the cashier spec inserting a sale
  ahead of `till.spec.ts` and moving the ticket number.

There is a third reason a screenshot changes with nothing behind it, and it
is not noise: **`settings-ar.png` and its siblings carry a live clock.** The
backups block shows the day and time of the last backup, which is the moment
the suite ran, so those files change every hour whatever the code does.
Measured the same day: the whole difference between two runs of
`settings-ar.png` was `16:22` becoming `17:22`, 684 differing subpixels out
of 9.4 million, all of them inside one 35-row band. Nothing else on the page
moved. If a settings screenshot is the only thing in a diff, check the clock
before looking for anything else.

Telling the two apart is worth the minute it costs, because the numbers
overlap. Rasterisation noise on an Arabic page runs to a few hundred bytes,
not tens: `theme-observe-ar.png` moved 445 bytes on a run that changed
nothing, spread over 134 rows across the full height of the page. A change
scattered over the whole image is rasterisation; a change inside one band is
something on the screen.

The second of those is the thing to remember about this suite: every spec
shares one API process over one SQLite file, in filename order, so adding a
spec changes what every spec after it sees. That is by design (the README
above says why) and it means a new spec legitimately redraws other screens.

Whenever any file under `screenshots/` changes, run
`pnpm --filter dzpos-landing shots` and commit `apps/landing/public/shots/`
with it: the landing page's product shots are cut from these exact files and
`apps/landing/src/shots.test.ts` compares them byte for byte.
