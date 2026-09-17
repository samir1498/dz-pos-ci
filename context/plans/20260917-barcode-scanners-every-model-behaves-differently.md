---
title: 'Barcode scanners: every model behaves differently'
slug: 'barcode-scanners-every-model-behaves-differently'
status: 'active'
category: 'feature'
created: 20260917
tldr: 'A scanner is a keyboard that types fast, and the models differ in three ways the till does not survive today: what they send after the code, whether anything is focused, and whether their layout matches the machine. Read the physical key, time the burst, and simulate all of it here before a real scanner is in the room.'
priority: 80
tasks:
  - id: 'T1'
    desc: 'This page: the behaviour matrix, how each behaviour is simulated and where it runs; kanban cards drafted for Samir'
    status: 'done'
  - id: 'T2'
    desc: 'src/lib/scan.ts: pure burst rule and barcode matching, time passed in, no React; unit tests on the boundaries'
    status: 'pending'
  - id: 'T3'
    desc: 'src/hooks/useScanner.ts on performance.now(), wired into till.tsx; the Enter-adds-an-exact-barcode branch leaves onSearchKey so one scan is one add'
    status: 'pending'
  - id: 'T4'
    desc: 'e2e spec: every row of the matrix that can run headless here, including the AZERTY case through CDP and the idle case through page.clock'
    status: 'pending'
  - id: 'T5'
    desc: 'Stryker mutation testing on src/lib/scan.ts alone, a just mutate recipe, every survivor killed or written down as equivalent with its reason'
    status: 'pending'
  - id: 'T6'
    desc: 'Samir by hand: the laptop with a uinput virtual keyboard on the fr layout, and an Android phone as a Bluetooth HID scanner, against the Tauri window'
    status: 'pending'
acceptance: []
---
# Barcode scanners: every model behaves differently

Anouar, 2026-09-17: test the barcode scanners, there are several models. He
does not know which, and the model numbers would not help much anyway.
Nearly every scanner sold in Algeria is a keyboard wedge: it types the code
into whatever has focus and sends a key at the end. What differs between
models is small, and each difference breaks the till as it stands.

## What the till does today

`apps/desktop/src/routes/till.tsx`, `onSearchKey`: the search box takes the
characters, Enter is the trigger, and an exact match on `barcode` adds the
product and clears the box. The box is refocused after a sale, after an add
and when the screen mounts (`till.tsx:189`, `:203`, `:242`, `:538`), so the
ordinary counter rhythm does keep focus where a scan can land.

That is one scanner's behaviour, configured one way, on a keyboard whose
layout matches the scanner's. Three things move.

**What the scanner sends at the end.** Enter is the common default. Tab is
the other common one, and some ship with no suffix at all. With no Enter
the code sits in the search box as a filter and nothing is added.

**Where the focus is.** The till puts it back in the right box after every
sale, but a scan while a dialog is open, or while the cashier is in the
customer box or the amount box, types the barcode into that field instead.

**Whether the layouts agree.** A scanner ships configured for a US keyboard
and the shop's machine is French. The scanner sends the physical key for
`1`; the OS reads it through the AZERTY map and hands the page `&`. The
whole code arrives as `&é"'(` and nothing ever matches. The laptop this is
demonstrated on is `X11 Layout: fr`, so this is not hypothetical.

And one that is not the scanner's fault: a UPC-A code is twelve digits, the
EAN-13 in the shop file is the same code with a leading zero. They must
match.

## The shape of the fix

A scanner is not a person typing. It sends eight or more characters with
gaps of tens of milliseconds and no thinking pauses, which is a signature
nothing at a counter produces by hand.

- **Time the burst, do not wait for Enter.** Characters arriving faster
  than `SCAN_MAX_GAP_MS` accumulate; the code is taken on Enter, on Tab, or
  after `SCAN_IDLE_MS` of quiet. A pure function owns this and takes the
  time as an argument, so a test can be exact about a boundary instead of
  sleeping.
- **Read the physical key when the characters are not plausible.** The
  event carries both what the OS decided (`key`) and which key was pressed
  (`code`). Build both strings; if the `key` string is not a plausible
  barcode and the `code` string is all digits, the layouts disagreed and the
  physical keys are the truth. Preferring `key` first is what keeps a
  correctly configured French scanner working, where blind `code` reading
  would break it.
- **Listen at the window, not at one box.** The scan lands wherever focus
  is. The till already has a window key listener for F9, so this is the same
  shape.
- **Do not eat what a person typed.** The idle path only fires for a string
  that looks like a barcode; a fast typist's `camembert` with a pause after
  it is not one. A code that matches nothing never clears the box.

Two integration traps, both from the review of this plan:

- One scan must be one add. A scanner that does send Enter would otherwise
  fire the hook's terminator and `onSearchKey`'s exact-barcode branch, and
  the product lands in the basket twice. The barcode branch leaves
  `onSearchKey`; Enter there keeps only "one thing is visible, add it".
- A prefix character defeats the plausibility check, because neither string
  is all digits any more. Non-alphanumerics are stripped from both ends
  before anything is judged.

## The behaviour matrix, and how each one is simulated

Every row except the last two runs on this box, headless, with no scanner in
the room.

| Behaviour | How it is simulated | Where it runs |
|---|---|---|
| Enter suffix (the common default) | Playwright types the code, presses Enter | e2e, here |
| Tab suffix | same, Tab instead | e2e, here |
| No suffix at all | `page.clock.install()` then `runFor(SCAN_IDLE_MS)` | e2e, here |
| Prefix character before the code (`*`, an AIM identifier) | prepend it to the typed string | unit + e2e |
| US-layout scanner on a French keyboard | CDP `Input.dispatchKeyEvent` with `key: "&"` and `code: "Digit1"`, which is exactly what the OS hands a page in that case | e2e, here |
| Inter-character speed, Bluetooth 10–30 ms against a slower USB unit | timestamps passed to the pure function | unit |
| A person typing, which must never be read as a scan | the same, with human gaps | unit + e2e |
| UPC-A scanned against an EAN-13 on file | `barcodeKeys` on both sides | unit |
| A scan while the focus is in another field or a dialog | Playwright focuses that field first | e2e, here |
| The real OS keymap through Tauri's webview | cannot be simulated here: Chromium is not WebKitGTK or WebView2 | laptop, by hand (T6) |
| A real wedge over Bluetooth HID | an Android phone running `hid-barcode-scanner`, which sends raw HID codes like a USB keyboard | laptop and phone, by hand (T6) |

What the headless run proves is that our code survives `key` and `code`
disagreeing. It does not prove that Tauri's webview delivers `code` the way
Chromium does. That is the whole reason T6 exists and is Samir's, not the
suite's.

Emulators considered before writing any of this, per the open-source-first
rule: `Fabi019/hid-barcode-scanner` (Android, Bluetooth HID, the closest
thing to a real scanner and the T6 tool), `ilyasozkurt/barcode-emulator-electron`
(injects keystrokes, cannot reach the keymap question), `theatrus/softwedge`
(serial to X11 key events, the uinput idea in older form). Nothing is
vendored; the Android app is installed on a phone and the rest is Playwright.

## Mutation testing

The burst rule is arithmetic on thresholds, which is exactly the code where
a test can pass while asserting nothing. Stryker runs over `src/lib/scan.ts`
and nothing else: a mutant inside a React hook or a route is noise, and the
desktop suite is too slow to run per mutant without `coverageAnalysis:
"perTest"`. Every survivor is either killed by a new test or written down
here as an equivalent mutant with the reason it cannot be killed.

## Not in this plan

The phone's own scanning is the camera and ML Kit, a different path that
this does not touch. Printing a barcode label already ships (`docs/features.md`
§4). Configuring a scanner by scanning its manual's setup codes is a thing
the shop can do and a thing we should not depend on.
