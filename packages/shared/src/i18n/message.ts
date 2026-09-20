// What a dictionary entry is, and how one becomes the sentence on a screen.
//
// Two things the desktop's `t` never had to do. The phone says "3 sales
// waiting to be sent", so an entry can carry a number, and it says it in
// Arabic, where that is six sentences and not two: a language's plural
// categories come from CLDR, and English's one/other is the exception
// rather than the shape to build on. `Intl.PluralRules` is the list, so
// the dictionaries do not carry a copy of it that can go stale.

/** The categories CLDR defines. `Intl.LDMLPluralRule` is the same list, but
 *  naming it here keeps a dictionary readable without the DOM lib. */
export type PluralCategory = "zero" | "one" | "two" | "few" | "many" | "other";

/** The three the product ships in. Named here rather than imported from the
 *  barrel because the barrel imports this file; the barrel's `Lang` is
 *  derived from `LANGS` and checked against this by a test, so the two
 *  cannot drift apart quietly. */
export type Lang = "fr" | "en" | "ar";

/** A sentence that changes with a count. `other` is required because every
 *  language has it and it is what a missing category falls back to, so an
 *  entry can never resolve to nothing. */
export type Plural = { readonly other: string } & {
  readonly [category in Exclude<PluralCategory, "other">]?: string;
};

/** One entry: a sentence, or a sentence per count. */
export type Message = string | Plural;

/** A domain's worth of entries. */
export type Messages = Readonly<Record<string, Message>>;

/** What a sentence can be given. `count` is also the number the plural
 *  category is chosen from, so `{count}` in the text needs no second
 *  variable.
 *
 *  `count` is a number and not a string, unlike every other variable. A
 *  string there would be substituted into the text and ignored by the
 *  category, so `{count: "3"}` would print the zero sentence with a 3 in
 *  it: in Arabic, "no sales waiting" above the number three. */
export type Vars = Readonly<Record<string, string | number>> & {
  readonly count?: number;
};

function isPlural(message: Message): message is Plural {
  return typeof message !== "string";
}

/** The plural category for a count, or `other` on an engine with no
 *  `Intl.PluralRules`.
 *
 *  Hermes has carried it since React Native 0.73 and this app is on 0.86,
 *  so the fallback is for a test runner or a trimmed build rather than a
 *  phone: it prints the English-shaped sentence instead of throwing, which
 *  is wrong in Arabic and readable in every language.
 *
 *  Not provable here. Every test in this package runs under Node's full
 *  ICU, never under Hermes, so a green suite says the dictionaries carry
 *  the right categories and nothing about whether a phone picks them. An
 *  incomplete Hermes would print the `other` sentence at one, two, three
 *  and eleven in Arabic with these gates green. That belongs on the same
 *  real device the plan already requires for right to left. */
function categoryOf(lang: Lang, count: number): PluralCategory {
  const rules = Intl.PluralRules;
  if (rules === undefined) return "other";
  const selected = new rules(lang).select(count);
  switch (selected) {
    case "zero":
    case "one":
    case "two":
    case "few":
    case "many":
    case "other":
      return selected;
  }
}

/** The sentence, with `{name}` replaced from `vars`.
 *
 *  A placeholder with nothing to put in it is left as written rather than
 *  blanked: "waiting for {count}" on a screen says which key and which
 *  variable to go and look at, and an empty gap says a sentence was meant
 *  to be shorter. */
export function format(lang: Lang, message: Message, vars: Vars = {}): string {
  const text = isPlural(message)
    ? (message[categoryOf(lang, vars.count ?? 0)] ?? message.other)
    : message;
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => {
    const value = vars[name];
    return value === undefined ? whole : String(value);
  });
}
