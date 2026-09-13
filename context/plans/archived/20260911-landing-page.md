---
title: 'Landing page'
slug: 'landing-page'
status: 'done'
category: 'product'
created: 20260911
tldr: 'The public page for the product: Astro, the branding, the real screens, three languages, on Cloudflare Pages'
priority: 40
tasks:
  - id: 'L0'
    desc: 'The site exists: `apps/landing`, Astro with the Tailwind v4 integration, reading `@dzpos/design` for its tokens so the page and the app cannot drift; the four theme blocks are not shipped here, the page is Comptoir with one dark band, because a visitor chooses nothing. Three languages as three routes (`/`, `/en`, `/ar`) with the Arabic one RTL, the vendored Plex families, the coin mark and the language-following wordmark from the kit. `just landing` serves it, `just landing-deploy` publishes to Cloudflare Pages (project `dinar-landing`), and the build joins `pnpm -r build` so a broken page fails the gates.'
    status: 'done'
  - id: 'L1'
    desc: 'The screens as product shots: a script that takes the committed e2e screenshots (`apps/desktop/e2e/screenshots/*.png`, twenty-three of them, French and Arabic) and composes the hero and the section images from them, in the browser frame the mockups use, one collage of several screens and several single shots; no hand-drawn fake UI anywhere on the page, every pixel is the real application. The script is rerun when a screen changes, and the result is committed so a visitor never waits on a build.'
    status: 'done'
  - id: 'L2'
    desc: 'The copy, French first, for a shopkeeper and not for a developer: what it is (the till, the facture, the stock, the suppliers, the dashboard), what it is not (no monthly cloud bill, no internet needed at the counter), the fiscal answers an Algerian owner asks first (a numbered facture in its yearly series, the stamp on cash, TVA per rate, the amount in words), and the three languages as a feature rather than a footnote. English and Arabic are translations of the same page, reviewed for Arabic by a native speaker before it goes public (R6 covers the printed words; this is the same rule for the page).'
    status: 'done'
  - id: 'L3'
    desc: 'The sections: a hero that shows the till and says what it does in one line; the four pieces (sell, invoice, stock, know); the fiscal block; a band of screens; who it is for; the price and how to get it, which is a form that writes to nothing until Anouar decides the model, so it collects a name, a phone and a wilaya and sends them to an address he reads. No fake testimonials, no invented shop names, no logos of shops that do not use it.'
    status: 'done'
  - id: 'L4'
    desc: 'The page must earn its load: no framework on the client except what a form needs, the fonts subset and preloaded, every image the right size in the right format with an explicit width and height, a Lighthouse pass over 95 on mobile, and the whole thing readable with JavaScript off. A test in the build that refuses an image without dimensions and a page over a size budget.'
    status: 'done'
  - id: 'L5'
    desc: 'Publish: the Cloudflare Pages project, the domain Anouar picks (or a pages.dev address until then), the sitemap and the Open Graph card drawn from the branding, the analytics question answered (Cloudflare Web Analytics is free and cookieless; nothing else goes on the page), and a line in the README and on the boss site saying where the page lives and how to change it. Before it publishes, the product shots are retaken: the support bundle added a button to the settings screen on 2026-09-12 and the committed picture of that screen predates it, so `settings-ar.png` and anything the shots script builds from it show a screen that is no longer the app. Nothing compares those pictures, so nothing failed; it needs a machine with disk to run the browser suite.'
    status: 'done'
acceptance: []
completed_at: '2026-09-11'
---
# The landing page

The public page for the product, built after the third milestone, when
there is a real application to photograph. Samir asked for it on
2026-09-11 alongside finishing the product.

It is Astro because the page is text and pictures with one form: nothing
on it needs a framework at the visitor's end, and the design plan already
said Astro for this. It lives in the same repository as `apps/landing`, so
the tokens, the logo and the fonts come from `@dzpos/design` and the page
cannot drift from the application it advertises.

Three rules decide most of the arguments:

- Every screen shown is a real screenshot the test suite took. No mockup,
  no invented data, no shop that does not exist.
- Every fiscal claim on the page is one `docs/features.md` already carries.
  A page that promises a facture the software does not print is the one
  mistake that costs a shop its trust.
- The Arabic page is not a machine translation. It waits for a native
  reader the way the printed amount in words does.

Open for Anouar: the final name (the product is still placeholder-named),
the price and the model, the domain, and who receives the form.

## Related

- `context/plans/20260910-design-system-and-branding.md` D5, which this
  plan replaces and extends.
- The screens gallery on the boss site, `https://dinar-reports.pages.dev/screens/`.
