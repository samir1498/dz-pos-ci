import { mkdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import sitemap from "@astrojs/sitemap";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

import { renderTokensCss } from "./src/lib/tokens";
import { SITE_URL } from "./src/lib/site";

// Writes src/styles/tokens.css from @dzpos/design before anything else
// reads it. Runs as an integration (not a package.json pre-script) because
// this repo's pnpm does not run pre/post hooks for arbitrary script names
// (no `enable-pre-post-scripts` in .npmrc), and because Astro's config file
// is itself loaded through Vite, which is what lets a `.ts` config import
// @dzpos/design's TypeScript entry point at all.
const dzposTokens = () => ({
  name: "dzpos-tokens",
  hooks: {
    "astro:config:setup": () => {
      const stylesDir = fileURLToPath(new URL("./src/styles/", import.meta.url));
      mkdirSync(stylesDir, { recursive: true });
      writeFileSync(`${stylesDir}tokens.css`, renderTokensCss());
    },
  },
});

// https://astro.build/config
export default defineConfig({
  // Every canonical link, alternate link and sitemap entry below is built
  // from this; src/lib/site.ts is the one place the address itself lives.
  site: SITE_URL,
  integrations: [
    dzposTokens(),
    // @astrojs/sitemap over a hand-written XML file: it walks the built
    // routes itself, so a fourth route never needs a second place taught
    // about it, and its `i18n` option is what turns the three routes'
    // hreflang alternates into the sitemap's own <xhtml:link> entries
    // (docs.astro.build/en/guides/integrations-guide/sitemap, the `i18n`
    // option), the same three-language mapping Base.astro's <head> repeats
    // as <link rel="alternate">.
    sitemap({
      i18n: {
        defaultLocale: "fr",
        locales: { fr: "fr-FR", en: "en-US", ar: "ar" },
      },
    }),
  ],
  vite: {
    plugins: [tailwindcss()],
  },
  // Astro's own i18n routing (astro.build/en/guides/internationalization),
  // rather than a hand-rolled locale router: `/` stays French with no
  // prefix (the default), `/en` and `/ar` are the other two.
  i18n: {
    defaultLocale: "fr",
    locales: ["fr", "en", "ar"],
    routing: {
      prefixDefaultLocale: false,
    },
  },
});
