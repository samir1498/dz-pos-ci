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
    desc: 'Two themes in the tokens package: the semantic layer gains a theme axis, Comptoir (light: stone paper, teal selection, brass on the pay button, ink sidebar) and Registre (dark: ink green surfaces, paper text, brass accent), both emitted as CSS custom-property blocks and as a Tailwind v4 @theme so utility classes come only from tokens; a theme switcher in settings persisted in the shop file (and the OS preference as the default); the mockup tokens.css test extended to both themes; JetBrains Mono for every amount; Lucide icons as the one icon set (build-vs-buy: MIT, stroke, RTL-safe), a Money cell component and a Wordmark component as the first two kit pieces'
    status: 'pending'
  - id: 'D3'
    desc: 'The component kit in apps/desktop/src/components on the tokens (app shell with sidebar and topbar, page header, card, table, form field and input, select, button variants, chips, status pill, money cell, empty state, dialog), matching the mockup stylesheet and the chosen directions; an eslint rule that fails the gates on a raw input, button, select, table or a colour or pixel literal outside the kit; every existing screen rewritten on the kit with its tests and e2e green and its screenshots retaken; after purchases and the stock recount merge and before the dashboard; a fourth review lens per task from then on: the screenshot next to the mockup'
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

Icons: Lucide (MIT, stroke icons, already the set in Samir's own brand kit),
never hand-drawn; the logo is the one custom drawing.

The Claude Design link: `/design-login` done 2026-09-10; a design-system
project for Dinar POS is created and the kit pushed into it (`colors_and_type.css`,
preview cards, the logo files) so Samir iterates there; he runs `/design-sync`
himself to pull changes back.
