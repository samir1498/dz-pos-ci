// Cloudflare Web Analytics: free, cookieless, the only analytics this page
// may carry (context/plans/20260911-landing-page.md, L5: "the analytics
// question answered ... nothing else goes on the page"). Off by default, so
// a page that must not go public yet does not quietly start reporting
// visits the moment someone builds it: set DZPOS_CF_BEACON_TOKEN to the
// site's Web Analytics token (Cloudflare dashboard, Web Analytics, add a
// site, the snippet it gives carries `data-cf-beacon='{"token":"..."}'`)
// and rebuild. Unset, this is undefined and Base.astro renders no <script>
// tag at all, not a disabled one: a route someone builds without the
// variable ships nothing extra to a visitor or a Lighthouse run.
//
// Read from process.env, not import.meta.env: this module is imported by
// Base.astro's frontmatter, which runs in Node at build time (Astro's
// server-side render pass), and process.env reaches a variable under any
// name, where Vite's import.meta.env only forwards names prefixed PUBLIC_
// or VITE_ to that object by default.
export const CF_BEACON_TOKEN: string | undefined = process.env.DZPOS_CF_BEACON_TOKEN;
