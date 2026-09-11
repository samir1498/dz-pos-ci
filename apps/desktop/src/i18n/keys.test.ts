// Every visible string exists in ar, fr and en (coding rules). English is
// the key set; a key added to one file and forgotten in another fails here
// rather than showing English inside an Arabic screen.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { expect, test } from "vitest";
import ar from "./ar.json";
import en from "./en.json";
import fr from "./fr.json";

const reference = Object.keys(en).sort();

test.each([
  ["ar", ar],
  ["fr", fr],
])("%s carries exactly the English keys", (_lang, dict) => {
  expect(Object.keys(dict).sort()).toEqual(reference);
});

test.each([
  ["ar", ar],
  ["fr", fr],
  ["en", en],
])("%s leaves no string empty", (_lang, dict) => {
  const blank = Object.entries(dict).filter(([, value]) => value.trim() === "");
  expect(blank).toEqual([]);
});

// A duplicate key is invisible to every other test in this file, because
// they all read the parsed object and `JSON.parse` silently keeps the last
// one. On 2026-09-11 `error_forbidden` was in fr, en and ar twice, and the
// copy that won said "you do not have permission to see this" where the
// first said "you are not allowed to do this" — so a cashier refused an
// action was told about seeing. This reads the file as text instead.
test.each(["ar", "en", "fr"])("%s spells every key exactly once", (lang) => {
  const raw = readFileSync(join(__dirname, `${lang}.json`), "utf8");
  const spelled = [...raw.matchAll(/^\s*"([^"]+)":/gm)].map((m) => m[1]);
  const seen = new Set<string>();
  const twice = spelled.filter((key) => (seen.has(key) ? true : (seen.add(key), false)));
  expect(twice).toEqual([]);
});
