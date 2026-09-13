// This page's identity as a Cloudflare Pages project, named in one place so
// it cannot drift: astro.config.ts reads SITE_URL for `site` (every
// canonical link, alternate link, sitemap entry and og:url is built from
// it), and the justfile's `landing-deploy` reads PAGES_PROJECT with a bare
// `node` one-liner so the two never name different projects. Change the
// domain here once Anouar picks one; until then this is the pages.dev
// address the L0 plan named.

import type { Locale } from "../i18n/types";

export const PAGES_PROJECT = "dinar-landing";

export const SITE_URL = `https://${PAGES_PROJECT}.pages.dev`;

/** Where each locale's route actually lives (astro.config.ts's i18n
 * routing: fr unprefixed, the other two under their own segment). */
export const ROUTE_PATH: Readonly<Record<Locale, string>> = {
  fr: "/",
  en: "/en/",
  ar: "/ar/",
};

/** `hreflang` on <link rel="alternate">, the same codes astro.config.ts
 * passes to @astrojs/sitemap's `i18n` option, so the page's own alternate
 * links and the sitemap's never disagree about what to call a locale. */
export const HREFLANG: Readonly<Record<Locale, string>> = {
  fr: "fr-FR",
  en: "en-US",
  ar: "ar",
};

/** `og:locale`: Facebook's own locale list names `fr_FR`, `en_US` and
 * `ar_AR` (generic Arabic, no country); there is no `fr_DZ` or `ar_DZ` on
 * that list to prefer instead. */
export const OG_LOCALE: Readonly<Record<Locale, string>> = {
  fr: "fr_FR",
  en: "en_US",
  ar: "ar_AR",
};

/**
 * Where the installers live: releases on the org repo, cut by the release
 * workflow (docs/architecture.md § Release) from tags built on the public
 * mirror. The filenames are the stable names the build job renames each
 * installer to before upload, so these URLs never carry a version and the
 * landing page needs no update per release. `releases/latest` resolves to
 * the newest non-draft, non-prerelease release -- which is why the first
 * builds (unsigned, draft while the certificate is missing) do not move
 * these links: drafts are invisible to `latest` by design.
 */
export const RELEASE_ORG = "Dinar-dz/dz-pos";

export const DOWNLOAD_FILES = {
  windows: "Dinar-Setup.exe",
  macos: "Dinar.dmg",
  linux: "Dinar.AppImage",
} as const;

export type DownloadOs = keyof typeof DOWNLOAD_FILES;

export const downloadUrl = (os: DownloadOs): string =>
  `https://github.com/${RELEASE_ORG}/releases/latest/download/${DOWNLOAD_FILES[os]}`;

export const RELEASE_PAGE = `https://github.com/${RELEASE_ORG}/releases/latest`;
