// The woff2 files to preload, per script. Every weight here is one the CSS
// actually applies above the fold on every route: 400 (body copy), 500
// (.btn-primary in the hero), 600 (every h1/h2), 700 (the UA-bold <strong>
// in the header row, src/layouts/Base.astro). Imported with `?url` so Vite
// resolves each to its hashed build path instead of copying the file
// (there is no other way to get that path without duplicating the hash
// Vite already computes); src/pages/*.astro reads the array for its script
// and Base.astro turns it into <link rel="preload"> tags.
//
// Only the woff2 URL: every browser Lighthouse's mobile run targets (and
// every browser this page otherwise supports) takes woff2, and the woff
// fallback in fonts-latin.css / fonts-arabic.css is for user agents this
// preload list does not need to chase.

import latin400 from "@fontsource/ibm-plex-sans/files/ibm-plex-sans-latin-400-normal.woff2?url";
import latin500 from "@fontsource/ibm-plex-sans/files/ibm-plex-sans-latin-500-normal.woff2?url";
import latin600 from "@fontsource/ibm-plex-sans/files/ibm-plex-sans-latin-600-normal.woff2?url";
import latin700 from "@fontsource/ibm-plex-sans/files/ibm-plex-sans-latin-700-normal.woff2?url";
import arabic400 from "@fontsource/ibm-plex-sans-arabic/files/ibm-plex-sans-arabic-arabic-400-normal.woff2?url";
import arabic500 from "@fontsource/ibm-plex-sans-arabic/files/ibm-plex-sans-arabic-arabic-500-normal.woff2?url";
import arabic600 from "@fontsource/ibm-plex-sans-arabic/files/ibm-plex-sans-arabic-arabic-600-normal.woff2?url";
import arabic700 from "@fontsource/ibm-plex-sans-arabic/files/ibm-plex-sans-arabic-arabic-700-normal.woff2?url";

export const LATIN_FONT_URLS: readonly string[] = [latin400, latin500, latin600, latin700];
export const ARABIC_FONT_URLS: readonly string[] = [arabic400, arabic500, arabic600, arabic700];
