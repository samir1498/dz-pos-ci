---
title: 'The demo video: Playwright records, Remotion cuts'
slug: 'the-demo-video-playwright-records-remotion-cuts'
status: 'active'
category: 'marketing'
created: 20260916
tldr: 'Record the real app with the Playwright suite that already drives it, then use Remotion only to caption the clips and cut them together. No reconstruction of the UI inside Remotion — a demo that is drawn rather than recorded is a demo of a drawing.'
priority: 70
tasks:
  - id: 'T0'
    desc: 'disk: /mnt/c at 60 GB free on 2026-09-16 evening, enough for pnpm install in dinar-remotion, one Android system image and the renders; just disk before every heavy step'
    status: 'done'
  - id: 'T1'
    desc: 'a demo Playwright project: 1920x1080 video on, fr only, slower pace, own testDir, ignored by the three language projects'
    status: 'done'
  - id: 'T2'
    desc: 'five scene specs: ring a sale, the ticket, credit, the day, Arabic; recorded not asserted, fr, 1920x1080'
    status: 'done'
  - id: 'T2b'
    desc: 'an Android emulator on the Windows SDK (Anwender, cmdline-tools 3.0, no system image yet): one x86_64 image, one AVD named dinar, adb reachable from WSL over the host address; Expo Go on it loading Metro from this box'
    status: 'done'
  - id: 'T2c'
    desc: 'a Maestro flow for the demo (pair by the typed path, tap the name, four digits, ring a sale) run against the emulator with adb screenrecord around it; the clip lands beside the Playwright ones'
    status: 'done'
  - id: 'T3'
    desc: 'just demo-clips: run the project, ffmpeg webm to mp4, stable names under dinar-remotion/public/recordings, gitignored; DZPOS_DEMO gates the project so a bare playwright run never records'
    status: 'done'
  - id: 'T4'
    desc: 'Remotion scenes: OffthreadVideo per clip in a Series, captions in the product''s own tokens, cut points as constants not generated JSON'
    status: 'pending'
acceptance: []
---
# The demo video: Playwright records, Remotion cuts

Samir's call, 2026-09-16, and it is the right one. The first scaffolding of
`dinar-remotion` assumed the scenes would be built in Remotion — React
components imitating the till. That is how you end up shipping a video of an
app that does not exist: every pixel is a second implementation of a screen,
it drifts the day someone changes a colour, and nothing in it is evidence.

`apps/desktop/e2e` already drives the real app end to end — twenty specs
across `fr`, `en` and `ar`, against a real `crates/api` on a real seeded
shop file. Playwright records video natively. So the footage is free, it is
the real product, and it is re-recordable the day a screen changes.

Remotion's job shrinks to what it is actually good at: titles, captions,
lower thirds, transitions, and deciding where one clip ends and the next
begins.

## Shape

```
apps/desktop/e2e/demo/*.spec.ts   one spec per scene, recorded not asserted
        ↓ just demo-clips
dinar-remotion/public/recordings/*.mp4  stable names, committed? no — see below
        ↓
dinar-remotion/src/scenes/*.tsx    <OffthreadVideo> + captions
        ↓ pnpm render
out/dinar-demo.mp4
```

## T1 — A `demo` Playwright project that records

A project in `apps/desktop/playwright.config.ts` separate from the three
language ones, so `just e2e` does not start writing video on every run:

- `video: { mode: "on", size: { width: 1920, height: 1080 } }`
- viewport 1920×1080, one language (`fr` — it is the shop's language)
- a slower default action pace, because a demo at test speed is unreadable
- `testDir` pointed at `e2e/demo`, and `testIgnore` on the language projects
  so a demo spec never runs as a test

These specs are not tests. They assert almost nothing; they perform. Keeping
them in their own folder is what stops someone reading a missing assertion
as a gap in coverage.

## T2 — The scenes, one spec each

1. **Ring a sale.** Add three things to the basket, take cash, hand over
   change. The amount includes the TVA and the droit de timbre, which is the
   whole point — that is the thing an Algerian shopkeeper cannot get from a
   foreign till.
2. **The ticket.** The printed document, with the fiscal identifiers and the
   stamp on it.
3. **Credit.** A customer buys on the book, their balance moves, a payment
   settles it.
4. **The day.** The dashboard: what came in, what is owed, what is short.
5. **Arabic.** The same till, switched, right to left. One shot, no
   narration needed.

The phone is the one thing Playwright cannot record. Samir's call on
2026-09-16: an Android emulator on this box's Windows side, driven by
Maestro, with `adb shell screenrecord` around the flow. The recording is
then one command like the others, `just demo-phone`, not a hand-held clip.

What it took on this box, evening of 2026-09-16, so nobody rediscovers it:

- The SDK under `C:\Users\Anwender\AppData\Local\Android\Sdk` had
  cmdline-tools 3.0 and no image. `sdkmanager.bat` with `JAVA_HOME` at
  `C:\Users\collaborator\.jdks\corretto-17.0.13` installed
  `system-images;android-34;google_apis;x86_64`, `platforms;android-34`,
  `emulator` (37.1.11) and `platform-tools`; the licence prompts need a
  file of `y` lines piped in, and the package names only keep their quotes
  when the call is in a `.cmd` file, not on a `cmd.exe /c` line from WSL.
- The AVD is `dinar`, profile `pixel_4` (this SDK has no `pixel_6`
  profile). WHPX is usable on this AMD box; `emulator.exe -avd dinar
  -gpu swiftshader_indirect` boots in under a minute.
- WSL cannot reach the Windows adb server: even started with `-a` it is
  behind the Windows firewall, and there is no admin to open it. So adb is
  `adb.exe` called from WSL (the Windows PATH is on WSL's PATH), and
  Maestro runs on the Windows side too: the Linux install copied to
  `C:\Users\Anwender\.maestro`, `maestro.bat` with the same JDK 17
  (`C:\Users\Anwender\dz-maestro.cmd` sets it up;
  `scripts/maestro-windows.sh` is the wrapper `just demo-phone` uses on WSL
  by default). Maestro 2.10.0 runs fine there.
- `10.0.2.2` from the emulator reaches Windows, not WSL. `adb reverse`
  does, through WSL's localhost relay, with one catch: Metro listens on a
  dual-stack `::` socket, which the relay mirrors onto Windows as `[::1]`
  only, and `adb reverse` connects to `127.0.0.1`. `scripts/ipv4-front.py 8082 8081`
  is an IPv4 listener in front of Metro; `adb reverse tcp:8081 tcp:8082`
  and `adb reverse tcp:4317 tcp:4317` make `localhost` on the device mean
  this box. Metro runs with `REACT_NATIVE_PACKAGER_HOSTNAME=localhost` and
  `EXPO_PUBLIC_API_URL=http://localhost:4317`.
- Expo Go 57.0.9 from `expo-go-releases` on GitHub (the versions API at
  `exp.host/--/api/v2/versions` names the apk), `adb install`, opened with
  `am start -a android.intent.action.VIEW -d exp://localhost:8081`. The
  first open shows Expo's developer-menu card over the app, and the card
  hides the screen from the accessibility tree; the flow taps `Continue`
  before reading anything.
- This box's `.dev/dev.db` had no credential set (the seed predates
  passwords); `POST /auth/first-setup` claimed the owner with the seed's
  password and `POST /users/1/pin` gave them the seed PIN, so the flow's
  defaults hold here as on the laptop.

One blemish to deal with in the cut: Expo Go draws its own floating
"Tools" button over the top right of every frame. A development client
built without the dev menu would remove it, which is a bigger job than
masking that corner in Remotion.

## T3 — Clips into the Remotion project

`just demo-clips`: run the demo project, then move each `video.webm`
Playwright wrote to a stable name under
`dinar-remotion/public/recordings/`, which is where the Remotion project
already looks. `DZPOS_DEMO=1` is what makes the project exist: a bare
`playwright test` selects every project it can see, and a scene rewrites
the shop it performs in.

Convert to mp4 on the way. Remotion seeks a webm far less reliably than an
mp4, and a demo render that drops a frame in the middle of the sale is a
render nobody can trust. `ffmpeg -i in.webm -c:v libx264 -crf 18 out.mp4`.

The clips are **not** committed. They are tens of megabytes, they are
regenerated by one command, and a binary that changes every recording is
exactly what a repo should not carry. `public/recordings/` is in `.gitignore`
and the README says which command fills it.

## T4 — The Remotion side, which is now small

`<Series>` of scenes; each scene is `<OffthreadVideo>` plus whatever text
goes over it. The tokens copied from `packages/design` are already in
`src/tokens.ts`, so a caption is set in the product's own type and colour.

Cut points are constants in the scene file, not a generated JSON timeline —
that is one of the ObserveOne mistakes the agent already wrote down in
`NOTES.md` and avoided.

No voiceover in v1. Captions read fine muted, which is how a video on a
landing page is watched.

## Disk

`/mnt/c` was at 9.2 GB free on the morning of 2026-09-16 and 60 GB free by
the evening, so the plan is unblocked. `dinar-remotion` has still never had
`pnpm install` run in it. `just disk` before every heavy step; a system
image is about 1.5 GB, a render a few hundred MB.

## Why not just screen-record it by hand

Because it has to be redone. Every price change, every new screen, every
language, and the demo is stale. A recording the test suite produces is one
command away from current, and it is the same run that proves the flow
works.
