---
title: 'A calendar for the clinic''s book'
date: 2026-09-23
status: 'draft'
tldr: 'Five ways to render the doctor''s day/week grid were compared on
  licence, bundle size, right-to-left support, first-day-of-week and
  hidden-day options, React 19 support, restyling and maintenance:
  Schedule-X, FullCalendar, react-big-calendar, the lighter vkurko/calendar
  ("Event Calendar"), and a hand-built CSS Grid with date-fns. FullCalendar''s
  free plugins (core, dayGrid, timeGrid, interaction) are MIT, cover every
  "must" in the brief including a documented Arabic right-to-left mode and
  an exact firstDay/hiddenDays match for a Sunday-start, Friday-and-Saturday-
  closed week, and are the only set of the five where drag-to-move is not
  paywalled. The catch is a plain, dated default look that needs real
  restyling work, and v7''s move to the Temporal API pulls in a
  temporal-polyfill dependency (about 20KB gzipped) unless the app''s
  webview already has Temporal built in, which is not yet confirmed for
  Tauri''s bundled webviews as of this writing. The fallback is the
  hand-built grid: no library risk and full token control from day one,
  paid for in weeks of grid, overlap and drag code Dinar would otherwise
  get for free.'
---
# A calendar for the clinic's book

## What the desktop already has

`apps/desktop/package.json` pins React 19.2.7, TanStack Router 1.170 and
TanStack Query 5.101, Tailwind 4.3 (via `@tailwindcss/vite`), and
`@dzpos/design` as the styling layer. `@dzpos/design` is not a shadcn
component drop-in; it is a hand-written token pipeline (`primitives.ts` →
`semantic.ts` → `theme.ts` → `css.ts`) that resolves to CSS custom
properties consumed through a `data-theme` attribute, four themes deep
(comptoir, registre, observe, observe-dark). No date library (date-fns,
dayjs, luxon, moment) is installed anywhere in the repo yet; whichever
option is picked either brings its own or the hand-built one adds date-fns
fresh.

## The five compared

### 1. Schedule-X

MIT for the core (`@schedule-x/calendar`, `@schedule-x/react`), but its own
plugin list marks with a star which parts are premium, and the starred list
is exactly what a book screen would reach for: Drag and Drop, Resize,
Interactive Event Modal, Sidebar, Drag to create, and the resource/Gantt
views. The plain (non-interactive) Event Modal is free, so click-to-open
works without a licence; click-to-book and drag-to-move do not.
(https://schedule-x.dev/docs/calendar/plugins,
https://schedule-x.dev/premium)

- Bundle: `@schedule-x/calendar` 37.3KB gzip + `@schedule-x/react` 0.9KB
  gzip, about 38KB total (bundlephobia.com API, 2026-09-23, v4.8.0).
- RTL/Arabic: the language docs say translations can be added per locale
  and mention RTL languages, but the mechanism for mirroring the grid
  itself (not just translated strings) is not spelled out on the page
  (https://schedule-x.dev/docs/calendar/language).
- First day of week: yes, `firstDayOfWeek` (Temporal-style, 1=Monday to
  7=Sunday, so 7 gives a Sunday start). Hidden days: not documented as a
  built-in option; the closest built-in is `dayBoundaries` for closed
  *hours*, not closed *days*, a Friday/Saturday closure would need a
  custom view (https://schedule-x.dev/docs/calendar/configuration,
  https://schedule-x.dev/docs/calendar/advanced/custom-views).
- React 19: yes, `@schedule-x/react` peer range is
  `^16.7.0 || ^17 || ^18 || ^19` (npm registry, 2026-09-23).
- Restyling: ships a Material-flavoured default theme
  (`@schedule-x/theme-default`, 5.7KB gzip) meant to be overridden with CSS
  variables and classes; workable, but the aesthetic starting point is
  further from Dinar's ink/paper tokens than the others.
- Maintenance: `@schedule-x/calendar` published 2026-09-08 (v4.8.0);
  GitHub `schedule-x/schedule-x` has 56 open issues, last push
  2026-09-22, 2.6k stars, not archived (api.github.com, 2026-09-23).

### 2. FullCalendar

Dual-licensed: the standard plugins (core, `daygrid`, `timegrid`,
`interaction`, `react`) are MIT. Only the Scheduler bundle, Timeline
View, Vertical Resource View, and Print Optimization, all resource/
multi-column features, sits behind a commercial licence starting at
$480 (https://fullcalendar.io/license,
https://fullcalendar.io/docs/premium, https://fullcalendar.io/pricing).
A single-doctor day/week grid never touches that bundle: `dateClick`,
`eventClick`, `eventDrop` and `eventResize` (drag-to-move and resize) are
all in the free `interaction` plugin.

- Bundle (bundlephobia.com API, 2026-09-23, v7.1.0/v6.1.21): `react` 31.4KB
  + `daygrid` 7.8KB + `timegrid` 13.9KB + `interaction` 9.0KB gzip ≈ 62KB.
  v7's core runs on the Temporal API and lists `temporal-polyfill` as a
  peer dependency, itself 19.9KB gzip; unless the webview already has
  native `Temporal` (not yet verified for Tauri's WebView2/WebKit as of
  2026-09), the real total is closer to 82KB gzip. Pinning to the v6 line
  (plain `Date`-based, still MIT) avoids the polyfill at the cost of an
  older API surface.
- RTL/Arabic: a first-class, documented option, `direction: 'rtl'`, and
  the docs name Arabic and Hebrew directly
  (https://fullcalendar.io/docs/direction).
- First day of week: `firstDay` (0=Sunday). Hidden days: `hiddenDays`, an
  array of day indices, doubling as the Friday/Saturday closure
  (https://fullcalendar.io/docs/hiddenDays). Both are exact matches for
  the brief.
- React 19: yes, `@fullcalendar/react` peer range `^17 || ^18 || ^19`
  (npm registry, 2026-09-23, v7.1.0).
- Restyling: theming runs on `--fc-` prefixed CSS custom properties, which
  maps cleanly onto Dinar's own CSS-variable token pipeline, but the
  default look (thin borders, small serif-ish toolbar) reads as a 2015-era
  admin panel and needs a real pass to feel like the rest of Dinar.
- Maintenance: `@fullcalendar/react` published 2026-09-05 (v7.1.0);
  GitHub `fullcalendar/fullcalendar` has 1,135 open issues (a large
  project, proportionate to its 20.6k stars), last push 2026-09-05, not
  archived (api.github.com, 2026-09-23).

### 3. react-big-calendar

Fully MIT, nothing paywalled, the whole library is one free package
(https://github.com/jquense/react-big-calendar/blob/master/LICENSE).

- Bundle: 53.3KB gzip for the package itself (bundlephobia.com API,
  2026-09-23, v1.20.0); it takes a localizer (date-fns, dayjs, moment or
  Globalize) supplied by the app, adding a few more KB once tree-shaken.
- RTL/Arabic: an `rtl` prop exists and is used in the wild, but the
  project's own issue tracker has repeated reports of drag targeting the
  wrong column in RTL month view, last fixed around v1.8.2 (issues #1801
  and #2310, https://github.com/jquense/react-big-calendar/issues/1801,
  https://github.com/jquense/react-big-calendar/issues/2310); current
  1.20.0 should carry that fix but the history says RTL is a
  community-tested path, not the primary one.
- First day of week: set through the localizer's culture/locale, not a
  calendar prop. Hidden days: no built-in equivalent to `hiddenDays` , 
  the week view always renders seven columns; hiding two would mean
  filtering the range and re-deriving a custom view.
- React 19: yes, peer range `^16.14.0 || ^17 || ^18 || ^19`, and the
  project's own tracker has a "react 19 support" issue confirming it was
  asked for and closed (https://github.com/jquense/react-big-calendar/
  issues/2701; npm registry, 2026-09-23, v1.20.0).
- Restyling: plain `rbc-` prefixed classes, no CSS variable layer, every
  colour and spacing override is a CSS rule against those classes rather
  than a token swap, more manual than FullCalendar's variables.
- Maintenance: last published 2026-06-01; GitHub (now under
  `bigcalendar/react-big-calendar` after a repo move) has 118 open
  issues, 8.8k stars, not archived (api.github.com, 2026-09-23).

### 4. Event Calendar (vkurko/calendar), the lighter alternative

A FullCalendar-inspired, framework-agnostic rebuild: MIT, zero
dependencies, and used by the Bookly WordPress booking plugin on over
70,000 sites (https://github.com/vkurko/calendar,
https://raw.githubusercontent.com/vkurko/calendar/master/README.md).
Nothing is paywalled; there is no premium split at all.

- Bundle: `@event-calendar/build` (the all-plugins bundle) is 43.3KB
  gzip (bundlephobia.com API, 2026-09-23, v5.14.1); the README's own
  figure is "35KB brotli-compressed," a different compression measure for
  the same package.
- RTL/Arabic: not documented anywhere in the README's option list; no
  `dir` or `rtl` option was found, only `locale` for `Intl.DateTimeFormat`
  string formatting. Grid mirroring would be untested territory.
- First day of week: `firstDay` (0=Sunday). Hidden days: `hiddenDays`, an
  array of day indices, the same clean fit as FullCalendar, and drag/
  resize (the `Interaction` plugin) is free.
- React 19: no official React binding at all, only a vanilla
  `createCalendar`/`destroyCalendar` JS API and a Svelte 5 component. A
  React screen would wrap the vanilla API in a `useRef`/`useEffect`, which
  works but is glue code Dinar would own, not a maintained adapter.
- Restyling: the `theme` option maps every internal class name
  (`ec-day`, `ec-event`, `ec-slot`, and so on) to a name of your choosing,
  which is an unusually direct hook for retargeting Tailwind classes, on
  top of already using CSS Grid for a minimal DOM.
- Maintenance: latest release published 2026-09-22 (the day before this
  page), 28 open issues, 2.3k stars, not archived
  (api.github.com and registry.npmjs.org, 2026-09-23).

*Toast UI Calendar (`@toast-ui/react-calendar`) was checked and ruled out
quickly: MIT, but the GitHub repo `nhn/tui.calendar` has been archived
since 2024-06-24 and its last publish to npm was 2022-08-16
(api.github.com, 2026-09-23), dead, not just quiet.*

### 5. Hand-built: CSS Grid + date-fns

No library at all for the grid itself: a day column and a week's worth of
half-hour or 15-minute rows laid out with CSS Grid, appointments
positioned by `grid-row` spans computed from `date-fns` (`differenceIn
Minutes`, `startOfWeek`, `eachDayOfInterval`), closed hours and absence
blocks rendered as plain greyed cells, click handlers on empty cells and
on appointment blocks, drag done by hand with pointer events later if
wanted.

- Licence: none to track, it is Dinar's own code, plus `date-fns` (MIT,
  17.5KB gzip full, https://bundlephobia.com, but tree-shaken to the
  handful of functions this needs, realistically a few KB).
- RTL/Arabic: full control, `dir="rtl"` on the grid plus Tailwind's
  logical-property utilities does exactly what the rest of the app
  already does for Arabic, no library gap to work around.
- First day of week / hidden days: whatever the code says; a Sunday-
  start, Friday/Saturday-closed week is just the array of days that gets
  rendered, no option name to discover.
- React 19: trivial, it is plain React.
  restyling: not a question, it is written directly against
  `@dzpos/design` tokens from the first line.
- Maintenance: entirely on Dinar. No upstream release to track, no issue
  tracker to watch, but also no one else's bug fixes arrive for free.

## Recommendation

FullCalendar's free plugin set, `core`, `daygrid`, `timegrid`,
`interaction`, `react`, covers every "must" in the brief without paying
for anything: a documented `direction: 'rtl'` for Arabic, an exact
`firstDay`/`hiddenDays` match for a Sunday-start week with Friday and
Saturday closed, native `slotDuration` for a 15-minute grid, events of any
length, `dateClick` for booking an empty slot, `eventClick` for opening
one, and `eventDrop`/`eventResize` for the nice-to-have drag, all in the
MIT tier, the Scheduler paywall only ever appears if a second doctor's
column (a resource view) gets added later.

The catch: FullCalendar's default look is a dated admin-panel skin that
needs a real restyling pass against Dinar's tokens (the `--fc-` CSS
variables make that possible, not free), and v7's Temporal API brings in
a `temporal-polyfill` dependency worth about 20KB gzipped unless the
Tauri webview already ships `Temporal` natively, which has not been
checked yet on this project's target webviews. Total footprint lands
around 60-80KB gzip depending on that answer.

The alternative, if the restyle turns out to cost more than it looks like
today or the Temporal weight is unwelcome, is the hand-built CSS Grid
with `date-fns`: no license or bundle surprises and pixel-level token
control from day one, paid for in the weeks of grid-layout, overlap-
handling and drag code that FullCalendar otherwise hands over for free.
Event Calendar (vkurko/calendar) sits between the two, same free
`hiddenDays`/`firstDay`/drag fit as FullCalendar, smaller, but with no
official React binding and no documented RTL, so picking it would mean
writing the React wrapper and testing Arabic mirroring from scratch.
