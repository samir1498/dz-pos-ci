// The phone's dictionaries, one TypeScript object per domain, merged per
// language.
//
// TypeScript and not JSON because a key is then a type: `t("till_pay_cash")`
// is checked at build time and `t("till_pay_cach")` is a red squiggle rather
// than a blank label on a cashier's screen. Split by domain because the
// desktop's three JSON files of several hundred keys each are the shape this
// replaces, and a file per screen is what makes a missing string obvious.
//
// The desktop keeps its JSON for now. It has five hundred keys and moving
// them proves nothing new; this package is built on the phone, which has
// seventy, and the desktop migrates onto it as its own piece of work
// (context/plans, the-phone-in-three-languages).

import { ar, arDomains } from "./ar";
import { en, enDomains } from "./en";
import { fr, frDomains } from "./fr";
import { format, type Messages, type Vars } from "./message";

export type { Message, Messages, Plural, PluralCategory, Vars } from "./message";
export { format } from "./message";

/** The three the product ships in. Arabic reads right to left, which is a
 *  layout fact the phone has to act on and not only a string one. */
export const LANGS = { fr, en, ar };

/** Spelled out in `message.ts` too, because that file is imported by this
 *  one and cannot import back. `the two spellings of Lang agree` in
 *  `i18n.test.ts` is what keeps them one list. */
export type Lang = keyof typeof LANGS;
export const RTL: ReadonlySet<Lang> = new Set(["ar"]);

/** Every key, derived from English. */
export type Key = keyof typeof en;

export function isLang(value: string | null | undefined): value is Lang {
  return value !== null && value !== undefined && value in LANGS;
}

export function isKey(value: string): value is Key {
  return value in en;
}

/** The domains unmerged, per language, for the completeness test. */
export const DOMAINS = { fr: frDomains, en: enDomains, ar: arDomains };

/** One language's sentence for a key.
 *
 *  A key a language has not been given falls back to English rather than to
 *  nothing: a French screen showing an English word says which string is
 *  missing, and a blank says a button has no label. The gates catch it first
 *  (`i18n.test.ts`), so this is the second line and not the plan. */
export function translate(lang: Lang, key: Key, vars?: Vars): string {
  const dictionary: Messages = LANGS[lang];
  const message = dictionary[key] ?? en[key];
  return format(lang, message, vars);
}
