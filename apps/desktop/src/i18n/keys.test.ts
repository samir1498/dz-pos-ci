// Every visible string exists in ar, fr and en (coding rules). English is
// the key set; a key added to one file and forgotten in another fails here
// rather than showing English inside an Arabic screen.

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
