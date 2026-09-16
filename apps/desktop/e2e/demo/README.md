# Demo scenes

These specs perform; they do not test. Each one is a scene of the demo
video, recorded off the real desktop app against the real API on the e2e
shop file, in French, at 1920x1080, with a slower hand
(`playwright.config.ts`, project `demo`). The three language projects
ignore this folder, so a scene is never counted as coverage, and `just e2e`
never writes video.

Run them with `just demo-clips` from the repository root. It runs the
project, then converts every `demo-clips/<scene>.webm` to an mp4 under
`~/dinar-remotion/public/recordings/`, where the Remotion cut picks it up
(context plan `the-demo-video-playwright-records-remotion-cuts`). The webm
and the mp4 are both gitignored: a recording is one command away and a
binary that changes on every run does not belong in history.

One scene per file, numbered so they run in the order the video tells them:

- `01-ring-a-sale`: three things, cash, change, the ticket with TVA and stamp.
- `02-the-ticket`: a facture to a named company, its RC and NIS on the
  paper, A4 then A5.
- `03-credit`: a regular buys on the book, then pays 500 DA on their page.
- `04-the-day`: the dashboard, by day and by week.
- `05-arabic`: the till switched to Arabic, right to left, one sale in it.

`shop.ts` holds the products, the two customers, the regime and the
seller's block every scene puts in place; each helper is safe to call
twice, so the scenes share one shop file and any one of them runs alone.

The phone is not here. Playwright cannot drive it; a Maestro flow on an
Android emulator records that clip (`apps/mobile/maestro/demo.yaml`,
`just demo-phone`).
