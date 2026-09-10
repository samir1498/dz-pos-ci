---
title: 'Design system and branding'
slug: 'design-system-and-branding'
status: 'active'
category: 'milestone'
created: 20260910
tldr: 'Two themes (Comptoir light, Registre dark) with a switcher, a bilingual logo the language switch swaps, Lucide icons, a component kit with a lint that forbids bare HTML, every screen rewritten on it, the kit pushed to a Claude Design project; landing page in Astro later'
priority: 45
tasks:
  - id: 'D1'
    desc: 'The logo: one mark (a coin carrying a stroke that reads as the Latin D and the Arabic dal), a Latin wordmark and an Arabic wordmark set to the same height, weight and spacing so the pair reads as one identity; SVG sources in packages/design/assets (mark, wordmark-latin, wordmark-arabic, lockups on light and dark, favicon, 80 mm ticket header in pure black); the desktop''s language switch swaps the wordmark (fr and en Latin, ar Arabic) and the mark stays; drawn first as a page on the boss site for Samir to approve'
    status: 'pending'
  - id: 'D2'
    desc: 'Two themes in the tokens package: the semantic layer gains a theme axis, Comptoir (light: stone paper, teal selection, brass on the pay button, ink sidebar) and Registre (dark: ink green surfaces, paper text, brass accent); the generated theme CSS emits the shadcn/ui variable set (background, foreground, card, primary, muted, border, ring, sidebar, radius, plus font-numeric and the brass money accent) for :root and [data-theme=registre] and the Tailwind v4 @theme inline mapping, the way ObserveOne''s src/index.css does (Anouar, 2026-09-10: the kit is shadcn/ui and ObserveOne''s theme is the design example; its shape is copied, not its colours); components.json, cva, tailwind-merge, clsx, lucide-react; fonts vendored through fontsource (IBM Plex Sans, IBM Plex Sans Arabic, JetBrains Mono), no external host; a theme switcher in settings persisted in the shop file (OS preference as the default); the mockup tokens.css test extended to both themes; a grep test that fails the gates on raw palette utilities or hex literals in the app; Money, Wordmark and Icon as the first kit pieces'
    status: 'pending'
  - id: 'D3'
    desc: 'The component kit is shadcn/ui installed through its CLI into apps/desktop/src/components/ui (button, input, label, select, checkbox, switch, table, card, badge, dialog, sheet, dropdown-menu, tabs, separator, skeleton, scroll-area, sidebar, breadcrumb, tooltip) on the D2 theme, plus the app shell (sidebar and topbar with the wordmark and the theme and language switches), page header, form field, status pill, empty state and data table built on them; an eslint rule that fails the gates on a raw input, button, select or table outside components/ui and on colour or pixel literals; every existing screen rewritten on the kit with its tests and e2e green and the screenshots retaken in both themes; RTL checked on every component (Radix handles direction; the sidebar and sheet sides follow dir); after purchases and the stock recount merge and before the dashboard; a fourth review lens per task from then on: the screenshot next to ObserveOne''s equivalent screen and the mockup'
    status: 'pending'
  - id: 'D4'
    desc: 'The Claude Design link: create the Dinar POS design-system project, push colors_and_type.css for both themes, the logo files, and one preview card per kit component (@dsCard markers) built from the kit''s own markup, so Samir iterates in claude.ai/design; the README in the project says which file is the source (the repo) and that /design-sync pulls changes back; the landing page in Astro is a later plan, not this one'
    status: 'pending'
  - id: 'D5'
    desc: 'Landing assets (Samir, 2026-09-10): product shots and screen collages for the Astro landing, made from the real screens once the kit lands: the e2e suite captures the till, the fiche, the dashboard and a facture in both themes and in fr and ar at 2x, a small script composes them into device mockups (a laptop frame, a phone frame for M6 later) and fanned multi-screen collages with the brand colours behind, exported as PNG and WebP under packages/design/assets/shots; the logo lockups, the palette and the type specimen exported beside them; the landing itself is a later plan in Astro'
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

Anouar, 2026-09-10 (after seeing the three directions and finding them all
not good): what ObserveOne taught is to pick the UI framework by hand and do
the design on it, or to hand Claude a design system already liked so it
reproduces that; ObserveOne is shadcn/ui (new-york, Radix, Tailwind v4,
Lucide, Inter through fontsource) with its own theme in src/index.css. So the
kit is shadcn/ui, ObserveOne's theme file is the shape to copy, Comptoir and
Registre are the two colour sets on it, and a designer, if Anouar hires one,
inherits shadcn variables.

The Claude Design link: `/design-login` done 2026-09-10; a design-system
project for Dinar POS is created and the kit pushed into it (`colors_and_type.css`,
preview cards, the logo files) so Samir iterates there; he runs `/design-sync`
himself to pull changes back.
