---
title: 'Design system and branding'
slug: 'design-system-and-branding'
status: 'active'
category: 'milestone'
created: 20260910
tldr: 'Four themes (comptoir, registre, observe, observe-dark) switched by data-theme, a bilingual logo the language switch swaps, Lucide icons, a component kit with a lint that forbids bare HTML, every screen rewritten on it, the kit pushed to a Claude Design project; the landing page is its own plan (`landing-page`, 2026-09-11), replacing D5 here'
priority: 45
tasks:
  - id: 'D1'
    desc: 'The logo: one mark (a coin carrying a stroke that reads as the Latin D and the Arabic dal), a Latin wordmark and an Arabic wordmark set to the same height, weight and spacing so the pair reads as one identity; SVG sources in packages/design/assets (mark, wordmark-latin, wordmark-arabic, lockups on light and dark, favicon, 80 mm ticket header in pure black); the desktop''s language switch swaps the wordmark (fr and en Latin, ar Arabic) and the mark stays; drawn first as a page on the boss site for Samir to approve'
    status: 'done'
  - id: 'D2'
    desc: 'Three themes in the tokens package (Samir, 2026-09-10 11:54): the semantic layer gains a theme axis, Comptoir (light: stone paper, teal selection, brass on the pay button, ink sidebar), Registre (dark: ink green surfaces, paper text, brass accent) and Observe (ObserveOne''s dashboard look applied to the shop: cool grey ground, white cards, slate text, emerald, with a dark twin observe-dark); the generated theme CSS emits the shadcn/ui variable set (background, foreground, card, primary, muted, border, ring, sidebar, radius, plus font-numeric and the brass money accent) for :root and [data-theme=registre] and the Tailwind v4 @theme inline mapping, our semantic tokens staying the source; components.json, cva, tailwind-merge, clsx, lucide-react; fonts vendored through fontsource (IBM Plex Sans, IBM Plex Sans Arabic, JetBrains Mono), no external host, tested; the switch is only data-theme on the root and a block of variables per theme, no theme-conditional code in the app (a grep test enforces it), a select in settings persisted in the shop file; Comptoir is the default since 2026-09-10 (Samir: "make A the default design"), the machine's light-or-dark preference no longer picks one; the mockup tokens.css test extended to both themes; a grep test that fails the gates on raw palette utilities or hex literals in the app; Money, Wordmark and Icon as the first kit pieces'
    status: 'done'
  - id: 'D3'
    desc: 'The component kit is shadcn/ui installed through its CLI into apps/desktop/src/components/ui (button, input, label, select, checkbox, switch, table, card, badge, dialog, sheet, dropdown-menu, tabs, separator, skeleton, scroll-area, sidebar, breadcrumb, tooltip) on the D2 themes, plus the app shell (sidebar and topbar with the wordmark and the theme and language switches), page header, form field, status pill, empty state and data table built on them; an eslint rule that fails the gates on a raw input, button, select or table outside components/ui and on colour or pixel literals; every existing screen rewritten on the kit with its tests and e2e green and the screenshots retaken in both themes; RTL checked on every component (Radix follows dir; the sidebar and sheet sides flip); after purchases and the stock recount merge and before the dashboard; a fourth review lens per task from then on: the screenshot next to the mockup'
    status: 'done'
  - id: 'D4'
    desc: 'The Claude Design link: create the Dinar POS design-system project, push colors_and_type.css for both themes, the logo files, and one preview card per kit component (@dsCard markers) built from the kit''s own markup, so Samir iterates in claude.ai/design; the README in the project says which file is the source (the repo); the bundle pushed on 2026-09-10 is behind the repo on the ink 50/100 steps and the muted sidebar text role, the next push carries them and that /design-sync pulls changes back; the landing page has its own plan (`landing-page`, 2026-09-11), which replaces D5; 2026-09-10 12:00: project created (id 3a9ee73d-2515-4971-84b6-23b800df5630) and seeded with the README, the tokens with four theme blocks, the mark SVGs and five preview cards; the kit cards follow D3'
    status: 'in-progress'
  - id: 'D5'
    desc: 'Landing assets (Samir, 2026-09-10): product shots and screen collages for the Astro landing, made from the real screens once the kit lands: the e2e suite captures the till, the fiche, the dashboard and a facture in both themes and in fr and ar at 2x, a small script composes them into device mockups (a laptop frame, a phone frame for M6 later) and fanned multi-screen collages with the brand colours behind, exported as PNG and WebP under packages/design/assets/shots; the logo lockups, the palette and the type specimen exported beside them; superseded 2026-09-11 by the `landing-page` plan, which carries this work as its own L1'
    status: 'pending'
acceptance: []
---
# Design system and branding

Samir saw the milestone on the dev server on 2026-09-10 and found the screens
bare: the React screens use plain inputs, buttons and tables with a few spacing
classes, the tokens package and the mockup stylesheet exist but nothing imports
them and nothing checks that they do. Three brand directions were drawn on the
till screen (boss site, /design/); Samir chose A (Comptoir, warm paper, calm
green) and B (Registre, ink green, brass, monospace amounts) as two themes with
a switcher, asked for the full branding assets (logo, icons), a logo that reads
alike in Arabic and Latin with the language switch swapping the wordmark, and a
landing page in Astro later.

Order: the logo and the token work first (little overlap with the milestone
tasks in flight), the kit and the screen rewrite after purchases and the stock
recount merge and before the dashboard, so the dashboard is born on the kit.

Icons: Lucide (MIT, stroke icons, the set shadcn and ObserveOne use), never
hand-drawn; the logo is the one custom drawing.

Anouar, 2026-09-10, after seeing the three directions and finding them all
not good, suggested building on a hand-picked UI framework the way ObserveOne
is (shadcn/ui) with ObserveOne's theme as the example. Samir's decision the
same hour: shadcn/ui as the component base (it is React), our own themes on it,
with Observe borrowing ObserveOne's dashboard look as the third; a designer, if Anouar hires one later, edits the
theme file.

The Claude Design link: `/design-login` done 2026-09-10; a design-system
project for Dinar POS is created and the kit pushed into it (`colors_and_type.css`,
preview cards, the logo files) so Samir iterates there; he runs `/design-sync`
himself to pull changes back.
