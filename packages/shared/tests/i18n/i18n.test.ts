// What holds three dictionaries in step.
//
// A missing key on the phone is a button with no label on a cashier's
// screen, which is worse than an English word on a French one: the fallback
// in `translate` makes sure it is the second and never the first. These
// tests are what make it neither, by failing the gates before a build.

import { describe, expect, test } from "vitest";

import { DOMAINS, LANGS, isKey, isLang, translate, type Lang } from "../../src/i18n/index";
import { en } from "../../src/i18n/en";
import { format, type Lang as MessageLang, type Message } from "../../src/i18n/message";

/** Every language, typed, rather than a list of strings narrowed back with a
 *  ternary. The ternary this replaces read `code === "fr" || code === "ar" ?
 *  code : "en"`, so a fourth language would have been checked as English
 *  under its own name: eighteen test runs quietly re-checking a dictionary
 *  that was already green, and the new one checked by nothing. The test lens
 *  named it on 2026-09-20 and proved it with a fake `es`. */
const LANG_CODES: Lang[] = Object.keys(LANGS).filter(isLang);

/** The languages that are checked *against* English, which is every one
 *  except English.
 *
 *  Three describes below hand each language the English dictionary and
 *  compare. For English that is the dictionary compared with itself, so
 *  those rows passed whatever anyone did to them: a key deleted, a plural
 *  flattened, a placeholder dropped. Three green rows that were green
 *  before the file was written. English is not unchecked as a result, it
 *  is what the other two are measured by, and the centimes lens's thread
 *  on 2026-09-20 is what showed the rows were empty: dropping `{amount}`
 *  from English's `till_change_due` fails `fr` and `ar` and never `en`. */
const MEASURED_AGAINST_ENGLISH: Lang[] = LANG_CODES.filter((code) => code !== "en");

/** Every leaf sentence in a dictionary, with the key it came from. A plural
 *  is several. */
function sentences(dictionary: Record<string, Message>): [string, string][] {
  return Object.entries(dictionary).flatMap(([key, message]) =>
    typeof message === "string"
      ? [[key, message] as [string, string]]
      : Object.entries(message).map(([form, text]) => [`${key}.${form}`, text] as [string, string]),
  );
}

function isPlural(message: Message): message is Exclude<Message, string> {
  return typeof message !== "string";
}

/** One comparison, both directions. A key English has and French does not is
 *  a button with no label; a key only French has is a string no screen can
 *  ask for, because `Key` comes from English. Comparing the sorted lists
 *  says both, and a failure prints which keys and which way round, where two
 *  membership filters said it in two tests and each said half. */
describe("every language carries English's keys and no others", () => {
  test.each(MEASURED_AGAINST_ENGLISH)("%s", (lang) => {
    expect(Object.keys(LANGS[lang]).sort()).toEqual(Object.keys(en).sort());
  });
});

/** A key that is present and empty is the failure this package exists to
 *  prevent, and membership says nothing about it. Proved by blanking
 *  Arabic's `action_back`: twenty-five green, and a back button with no
 *  label on a cashier's screen. */
describe("no sentence is blank", () => {
  test.each(LANG_CODES)("%s", (lang) => {
    const blank = sentences(LANGS[lang])
      .filter(([, text]) => text.trim() === "")
      .map(([where]) => where);
    expect(blank).toEqual([]);
  });
});

/** The merge is a spread, so a key spelled in two domains would be swallowed
 *  and the loser is the one nobody notices. */
describe("the merge loses nothing", () => {
  test.each(LANG_CODES)("%s", (lang) => {
    const domains: Record<string, Message>[] = Object.values(DOMAINS[lang]);
    const declared = domains.flatMap((domain) => Object.keys(domain));
    expect(new Set(declared).size).toBe(declared.length);
    expect(declared.length).toBe(Object.keys(LANGS[lang]).length);

    // And every key still carries its domain's own sentence. The counts
    // above cannot see a key written a second time in the barrel after the
    // six spreads: the name already exists, so nothing is added and nothing
    // is lost, only the value is quietly replaced. The test lens proved it
    // by overriding till_pay_cash in ar/index.ts.
    const merged: Record<string, Message> = LANGS[lang];
    for (const domain of domains) {
      for (const [key, sentence] of Object.entries(domain)) {
        expect(merged[key], `${lang} ${key}`).toBe(sentence);
      }
    }
  });
});

/** A count is six sentences in Arabic and two in English, and the list is
 *  CLDR's rather than ours. Asking `Intl.PluralRules` for it means a
 *  dictionary cannot carry a stale copy: Arabic needs zero, one, two, few,
 *  many and other, and a missing `few` prints the `other` sentence at three,
 *  which is the wrong sentence and not a blank one. */
describe("a plural entry carries every category its language uses", () => {
  test.each(LANG_CODES)("%s", (lang) => {
    const needed = new Intl.PluralRules(lang).resolvedOptions().pluralCategories;
    const dictionary: Record<string, Message> = LANGS[lang];
    for (const [key, message] of Object.entries(dictionary)) {
      if (!isPlural(message)) continue;
      const carried = Object.keys(message);
      expect(needed.filter((category) => !carried.includes(category)), `${lang} ${key}`).toEqual([]);
      // And nothing its language never asks for. A `two` form in English is
      // text no count can reach, which reads as coverage and is not.
      const used: readonly string[] = needed;
      expect(
        carried.filter((category) => !used.includes(category)),
        `${lang} ${key} carries a form its language never uses`,
      ).toEqual([]);
    }
  });

  /** And a key is a plural in every language or in none. English's one and
   *  other is the exception, so a sentence written flat in French because it
   *  reads fine at one and at two is a sentence that is wrong at zero. */
  test.each(MEASURED_AGAINST_ENGLISH)("%s agrees with English on which keys count", (lang) => {
    const dictionary: Record<string, Message> = LANGS[lang];
    const plurals = (entries: Record<string, Message>) =>
      Object.keys(entries)
        .filter((key) => isPlural(entries[key] ?? ""))
        .sort();
    expect(plurals(dictionary)).toEqual(plurals(en));
  });
});

/** Every `{name}` a sentence carries has to be one its callers pass, and the
 *  way to hold that without listing call sites is to hold the placeholders
 *  themselves: a French sentence that says `{amount}` where English says
 *  `{name}` is a sentence that prints the placeholder. */
describe("the placeholders agree across languages", () => {
  const placeholders = (message: Message): string[] => {
    const texts = typeof message === "string" ? [message] : Object.values(message);
    const found = new Set<string>();
    for (const text of texts) {
      for (const match of text.matchAll(/\{(\w+)\}/g)) {
        const name = match[1];
        if (name !== undefined) found.add(name);
      }
    }
    return [...found].sort();
  };

  /** Form by form for a plural, whole for a flat sentence.
   *
   *  Not unioned across a plural's forms. A union is satisfied by one form
   *  keeping `{count}` while the rest drop it, which is a sentence that
   *  prints "sales waiting to be sent" with no number in front of it: the
   *  test lens proved that by stripping `{count}` from Arabic's `many`. So
   *  the union is gone rather than kept as a second, weaker test beside
   *  this one.
   *
   *  Three forms are exempt, and the reason is CLDR's own: `zero`, `one`
   *  and `two` each match exactly one number, so a language that has them
   *  can put the number in the word. Arabic says حرف واحد and حرفان, and
   *  "1 character" there would be the wrong sentence rather than a longer
   *  one; its `zero` form says there are none, which is the number.
   *
   *  `few`, `many` and `other` match ranges. Arabic's `few` is three to ten
   *  and its `many` is eleven to ninety-nine, so a sentence in one of those
   *  that does not name the number leaves the reader without it. */
  const MATCHES_ONE_NUMBER = ["zero", "one", "two"];

  test.each(MEASURED_AGAINST_ENGLISH)("%s", (lang) => {
    const dictionary: Record<string, Message> = LANGS[lang];
    for (const [key, message] of Object.entries(en)) {
      const theirs = dictionary[key];
      if (theirs === undefined) continue;
      const wanted = placeholders(message);
      if (typeof theirs === "string") {
        expect(placeholders(theirs), `${lang} ${key}`).toEqual(wanted);
        continue;
      }
      for (const [form, text] of Object.entries(theirs)) {
        if (MATCHES_ONE_NUMBER.includes(form)) continue;
        expect(placeholders(text), `${lang} ${key}.${form}`).toEqual(wanted);
      }
    }
  });
});

describe("format", () => {
  /** One claim, both halves: a name it was given goes in, and a name it was
   *  not is left as written. An empty gap would say the sentence was meant
   *  to be shorter; `{name}` on the screen says which variable to go and
   *  look at. */
  test("substitutes the variables it has and leaves the ones it does not", () => {
    expect(format("en", "Add {name}", { name: "Ciment" })).toBe("Add Ciment");
    expect(format("en", "Add {name}")).toBe("Add {name}");
  });

  /** English exercises one and other, which is the pair anyone would write
   *  by hand. Arabic is why nobody should: three is `few` and eleven is
   *  `many`, and neither has an English word. Both in one test because it
   *  is one claim, that the category comes from `Intl.PluralRules` and not
   *  from `count === 1`. */
  test("picks the sentence the count calls for, in a language with six", () => {
    const english = { one: "{count} sale", other: "{count} sales" };
    expect(format("en", english, { count: 1 })).toBe("1 sale");
    expect(format("en", english, { count: 3 })).toBe("3 sales");

    const arabic = { one: "واحدة", two: "اثنتان", few: "قليل", many: "كثير", other: "أخرى" };
    expect(format("ar", arabic, { count: 1 })).toBe("واحدة");
    expect(format("ar", arabic, { count: 2 })).toBe("اثنتان");
    expect(format("ar", arabic, { count: 3 })).toBe("قليل");
    expect(format("ar", arabic, { count: 11 })).toBe("كثير");
    expect(format("ar", arabic, { count: 100 })).toBe("أخرى");
  });

  /** A separate claim, and the one the dictionaries' own category test is
   *  meant to keep unreachable. */
  test("falls back to other when the category is not carried", () => {
    expect(format("ar", { other: "أخرى" }, { count: 3 })).toBe("أخرى");
  });
});

/** `Lang` is spelled twice: derived from `LANGS` in the barrel, and written
 *  out in `message.ts`, which the barrel imports and so cannot import back.
 *  Two spellings of one list is the drift this package was built to stop,
 *  so they are checked against each other rather than trusted. */
describe("the two spellings of Lang agree", () => {
  test("every language the barrel carries is one message.ts names", () => {
    const named: MessageLang[] = ["fr", "en", "ar"];
    const carried: Lang[] = LANG_CODES;
    expect([...carried].sort()).toEqual([...named].sort());
  });
});

describe("translate", () => {
  /** `isLang` is already exercised by `LANG_CODES` at the top of this file,
   *  so it is not asserted again here. `isKey` has no other caller yet and
   *  is the guard a screen will use on a key that came off the wire, so one
   *  line pins it. */
  test("answers in the language asked for, for a key it recognises", () => {
    expect(translate("en", "till_pay_cash")).toBe("Pay cash");
    expect(isKey("till_pay_cash")).toBe(true);
    expect(isKey("till_pay_cach")).toBe(false);
  });
});
