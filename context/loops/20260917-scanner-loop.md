---
title: 'Scanner loop: a scanner is a keyboard, and every model types differently'
slug: 'scanner-loop'
status: 'done'
category: 'loops'
created: 20260917
tldr: 'Runs the barcode scanner plan task by task on the WSL box: the pure burst rule, the hook, the till, an e2e spec that simulates every model behaviour headless, and Stryker over the rule. Samir tests the two rows a browser cannot reach, on the laptop and a phone, at the end.'
plan: 'barcode-scanners-every-model-behaves-differently'
---
# Scanner loop

Closed 2026-09-21: every row a browser can reach is merged. The one task left
on the plan is the pair a browser cannot reach, Samir's laptop with a virtual
keyboard on the fr layout and a phone as a Bluetooth scanner, and it lives on
the plan as his by-hand pass, not here.

Anouar, 2026-09-17, through Samir: test the barcode scanners, there are
several models, and nobody knows which. The plan is
`barcode-scanners-every-model-behaves-differently`; this file is how it is
run.

Samir's two rulings, 2026-09-17, before the loop started:

- The till only. Not the product form, not purchases. A scanner is used at
  the counter all day and everywhere else occasionally.
- A code that matches nothing says so, in a line under the search box, and
  the digits stay where they are. Nothing is created, nothing is cleared.

## The order

1. T2: `apps/desktop/src/lib/scan.ts`. Pure, no React, time passed in:
   `feed(state, {key, code, at})` returns the next state and a scan when the
   burst ends, and `barcodeKeys(code)` gives the forms a code can match
   under (UPC-A against the EAN-13 on file). Unit tests on the boundaries,
   not near them. Its own PR.
2. T3: `apps/desktop/src/hooks/useScanner.ts` calling `feed` with
   `performance.now()`, wired into `till.tsx`. The exact-barcode branch
   leaves `onSearchKey` in the same commit: with both in place a scanner
   that sends Enter adds the product twice. Unknown code shows the line.
3. T4: the e2e spec, every row of the matrix that runs headless. The idle
   path through `page.clock`, the layout mismatch through a CDP
   `Input.dispatchKeyEvent` with `key` and `code` disagreeing. One scan is
   one add is an assertion here, because no unit test can see it.
4. T5: Stryker over `src/lib/scan.ts` alone, `coverageAnalysis: "perTest"`,
   a `just mutate` recipe. Survivors are killed or written into the plan as
   equivalent with the reason.
5. T6: handed to Samir with the exact commands: the laptop on its `fr`
   layout driving a `uinput` virtual keyboard, and an Android phone running
   `hid-barcode-scanner` as a Bluetooth wedge, both against the Tauri
   window rather than a browser tab.

Each code step is a branch and a PR, `just gates` before merge, `ctx:`
trailer naming the task. Context bookkeeping goes straight to `main`.
`just claim` before any cargo, `just disk` before the Stryker install.

## What would make this loop wrong

- Reading `code` in preference to `key`. It fixes the US-scanner-on-a-French-
  keyboard case and breaks the correctly configured French scanner, which is
  the same bug pointing the other way. `key` first, `code` only when what
  `key` produced cannot be a barcode.
- Emitting a scan for anything the idle timer sees. A fast typist's word is
  not a barcode; only a plausible code ends a burst by silence.
- Trusting the headless run. Chromium is not WebKitGTK and not WebView2.
  The suite proves our handling of `key` against `code`; it does not prove
  the webview hands us `code` at all. T6 is not optional.

## Not in this loop

The phone's camera scanning, which is ML Kit and a different path. Printing
labels, which ships. Configuring a scanner by scanning the setup codes in
its manual: a shop can do it, and we must not need them to.

## Where things are

- The plan and the behaviour matrix:
  `context/plans/20260917-barcode-scanners-every-model-behaves-differently.md`.
- The till's current behaviour: `apps/desktop/src/routes/till.tsx`,
  `onSearchKey` at ~line 430, the search box refocused at `:189`, `:203`,
  `:242`, `:538`.
- The laptop for T6: `samir@100.111.55.62`, `X11 Layout: fr`,
  `python-evdev`, `ydotool` and `xdotool` installed, `/dev/uinput` root-only
  until a udev rule gives the `input` group access.
