# design/ — clickable mockups

Plain HTML + ES modules, no build. Same data, same money rules
(`shared/money.js` is the first draft of `packages/shared`), three
languages with RTL. Index page lists what is interactive vs static and the
defaults awaiting a decision.

- `index.html` — landing
- `desktop/` — Tauri app mockup (1366+)
- `mobile/` — React Native app mockup (390×844 frame)
- `shared/` — tokens.css (three-tier tokens), components.css, money.js,
  i18n.js, data.js (invented shop), doc.js (A4 facture + 80 mm ticket)

Local: `python3 -m http.server 8766` in this folder, open `/`.

Deploy (Samir's Cloudflare account, Pages project `dz-pos-design`,
token from `CLOUDFLARE_API_TOKEN`):

```
pnpm dlx wrangler@4 pages deploy design --project-name dz-pos-design --commit-dirty=true
```

Rules for editing: logical CSS properties only (no left/right); every
visible string through `t()`; money only through `money.js`.
