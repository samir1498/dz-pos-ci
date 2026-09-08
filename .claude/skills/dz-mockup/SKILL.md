---
name: dz-mockup
description: Edit, serve, drive and deploy the dz-pos HTML mockups in `design/` (desktop Tauri mockup, mobile React Native mockup, shared tokens, money.js, i18n, invoice templates). Use when Samir asks to change a screen, add a flow, check the mockup, "deploy the design", or when a UI decision needs a clickable reference before code exists.
argument-hint: [serve|drive|deploy|<screen or change>]
---

The mockups are the design system until the apps catch up: `tokens.css`
and `components.css` are what the Tauri app's Tailwind theme and the RN
app's theme get generated from, and `money.js` is the first draft of the
shared money rules. Treat an edit here as a spec change, not a sketch.

Live: https://dz-pos-design.pages.dev (Cloudflare Pages, project
`dz-pos-design`, Samir's account).

## Editing rules

- Logical CSS only: `inline-start`, `margin-inline`, `padding-block`. Never
  `left`/`right`; Arabic flips the whole layout and a physical property is
  a mirrored bug that only shows in `ar`.
- Every visible string goes through `t("key")` and exists in all three
  dictionaries in `shared/i18n.js`. A missing key renders the key.
- Money only through `shared/money.js`; a total computed in a screen file
  is a second source of truth and the `dz-money` skill forbids it.
- A double-quoted string cannot hold `${t("x")}`; use a template literal.
  Syntax-check a module before serving:
  `node --input-type=module --check < design/desktop/app.js`.
- New screen: add the route in the app's `routes` table, the nav entry, the
  dictionary keys, and a line on `index.html` saying whether it is
  interactive or static.

## Serve locally

```
cd /home/samir/dz-pos/design && (python3 -m http.server 8766 >/dev/null 2>&1 & echo $! > /tmp/dz-mockup.pid)
```

Stop with `kill $(cat /tmp/dz-mockup.pid)` or `fuser -k 8766/tcp`. Never
`pkill -f 'http.server 8766'` in a chained command: the pattern matches the
shell running it, the shell dies (exit 144) and nothing after it runs. The
bracket form `pkill -f '[h]ttp.server 8766'` is safe if you must.

To put it on Samir's laptop screen instead, the `laptop-dev` skill has the
SSH and display details; the mockups need no build so `scp design/` and a
`python3 -m http.server` there is enough.

## Drive it headless (proof, not a screenshot)

`scripts/drive-desktop.mjs` and `scripts/drive-mobile.mjs` walk the main
flows with Playwright: add lines to the cart, pick a customer, open
payment, check stamp on cash against card, credit blocked over the limit,
change on a cash amount, switch language to `ar` and confirm `dir=rtl`,
open the A4 and ticket views. They print the numbers they read and exit
non-zero on any page error, console error or failed request.

```
node .claude/skills/dz-mockup/scripts/drive-desktop.mjs http://127.0.0.1:8766/desktop/ /tmp/desktop.png
node .claude/skills/dz-mockup/scripts/drive-mobile.mjs  http://127.0.0.1:8766/mobile/  /tmp/mobile.png
```

Both import Playwright by absolute path from the ObserveOne frontend's
`node_modules` on the WSL box, where Chromium is already cached. On another
machine either edit the import to a local `@playwright/test` install or run
`npx playwright install chromium` first. Adding Playwright as a dz-pos
dependency is a separate decision; do not do it inside a mockup change.

Expected totals for the scripted cart (Huile 920 + 2 × Sucre 110 + Eau 40,
cash, stamp on): HT 1180.00, TVA 19 % 182.40, TVA 9 % 19.80, TTC 1382.20,
stamp 13.82, net 1396.02, change on 2000.00 = 603.98. A different number
after an edit to `money.js` means the rule changed; update
`docs/features.md` and the fixtures, not the script.

## Deploy

```
cd /home/samir/dz-pos && pnpm dlx wrangler@4 pages deploy design --project-name dz-pos-design --commit-dirty=true
```

Needs `CLOUDFLARE_API_TOKEN` in the environment (it is set in Samir's shell
on the WSL box). A deploy is a publish; do it after the drive scripts pass,
and paste the deployment URL in the reply. Run the live URL through the
drive script once as well: what the CDN serves is the thing Samir opens.
