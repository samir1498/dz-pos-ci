// Every visible string exists in ar, fr and en (coding rules). English is
// the key set; a key added to one file and forgotten in another fails here
// rather than showing English inside an Arabic screen.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { expect, test } from "vitest";
import ar from "../../src/i18n/ar.json";
import en from "../../src/i18n/en.json";
import fr from "../../src/i18n/fr.json";

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
// first said "you are not allowed to do this", so a cashier refused an
// action was told about seeing. This reads the file as text instead.
//
// Nothing else in the repo would catch it: `just gates` has no JSON
// formatting check, so this one regex is the whole defence. It tolerates
// whitespace between the key and its colon because the first version did
// not, and a review proved a duplicate written `"app_name" : "..."` walked
// straight past it. That spacing is what a hand-resolved merge conflict
// produces, which is exactly how the doubled key got in.
const KEY_LINE = /^[ \t]*"([^"]+)"[ \t]*:/gm;

const spelledTwice = (raw: string): string[] => {
  const seen = new Set<string>();
  return [...raw.matchAll(KEY_LINE)]
    .map((match) => match[1])
    .filter((key) => (seen.has(key) ? true : (seen.add(key), false)));
};

// The regex is the guard, so the guard is itself checked against the two
// spellings a duplicate actually arrives in, rather than trusted.
test("the reader sees a doubled key however it is spaced", () => {
  expect(spelledTwice('{\n  "a": "1",\n  "a": "2"\n}')).toEqual(["a"]);
  expect(spelledTwice('{\n  "a": "1",\n  "a" : "2"\n}')).toEqual(["a"]);
  expect(spelledTwice('{\n  "a": "1",\n  "b": "2"\n}')).toEqual([]);
});

test.each(["ar", "en", "fr"])("%s spells every key exactly once", (lang) => {
  const raw = readFileSync(join(__dirname, "..", "..", "src", "i18n", `${lang}.json`), "utf8");
  expect(spelledTwice(raw)).toEqual([]);
});
