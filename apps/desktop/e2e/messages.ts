// Reads a language's JSON dictionary for the running Playwright project,
// so a spec's expected strings come from the language actually driving
// the page rather than always from fr.json. One cache entry per language
// keeps repeated t() calls inside a test from re-reading the file.

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "@playwright/test";

const here = fileURLToPath(new URL(".", import.meta.url));

export const LANG_CODES = ["fr", "en", "ar"] as const;
export type LangCode = (typeof LANG_CODES)[number];

function isLangCode(value: string): value is LangCode {
  return (LANG_CODES as readonly string[]).includes(value);
}

const cache = new Map<LangCode, Record<string, string>>();

function readDictionary(lang: LangCode): Record<string, string> {
  const cached = cache.get(lang);
  if (cached !== undefined) return cached;
  const file = path.join(here, "..", "src", "i18n", `${lang}.json`);
  const parsed: unknown = JSON.parse(readFileSync(file, "utf8"));
  if (typeof parsed !== "object" || parsed === null) {
    throw new Error(`${file} is not a JSON object`);
  }
  const messages: Record<string, string> = {};
  for (const [key, value] of Object.entries(parsed)) {
    if (typeof value !== "string") throw new Error(`${file}: ${key} is not a string`);
    messages[key] = value;
  }
  cache.set(lang, messages);
  return messages;
}

/** The Playwright project running the current test, as a language code.
 * Throws on a project the specs do not know how to translate for, rather
 * than silently falling back to French. */
export function currentLang(): LangCode {
  const name = test.info().project.name;
  // The demo project (e2e/demo, see playwright.config.ts) performs in the
  // shop's language; it is French with a camera on.
  if (name === "demo") return "fr";
  if (!isLangCode(name)) {
    throw new Error(`unexpected Playwright project "${name}", expected one of ${LANG_CODES.join(", ")}`);
  }
  return name;
}

/** The current project's translated string for `key`. */
export function t(key: string): string {
  return tIn(currentLang(), key);
}

/** `key` in a named language, for the moment a test switches the screen
 * away from the project's own and has to read what it now says. */
export function tIn(lang: LangCode, key: string): string {
  const value = readDictionary(lang)[key];
  if (value === undefined) throw new Error(`${lang}.json has no key ${key}`);
  return value;
}
