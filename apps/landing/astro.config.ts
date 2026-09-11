import { mkdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

import { renderTokensCss } from "./src/lib/tokens";

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
  integrations: [dzposTokens()],
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
