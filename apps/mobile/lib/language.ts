/** Which of the three languages this phone speaks, and where that is kept.
 *
 * The strings themselves live in `@dzpos/shared` so the desktop can move
 * onto the same dictionaries later. What is here is the part that is only
 * true of a phone: the choice survives a reboot, the first launch guesses
 * from the handset's own locale, and Arabic is a layout fact as well as a
 * string one.
 *
 * Plain TypeScript on purpose. `vitest.config.ts` only collects
 * `tests/lib/**\/*.test.ts`, because the screens are driven by Maestro against a
 * real server; anything in this feature that a test should hold had to land
 * here rather than inside the provider.
 */

import { I18N_RTL, isLang, type Lang } from "@dzpos/shared";

const STORAGE_KEY = "dzpos:lang";

/** What a phone opens in when nothing else says otherwise.
 *
 *  French, not English. The shops are Algerian and their paperwork is in
 *  French; English is in the product because the three dictionaries are,
 *  not because anyone asked for it at a counter. */
export const DEFAULT_LANG: Lang = "fr";

let memory: Lang | null | undefined;

async function store() {
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { default: AsyncStorage } = await import(
    "@react-native-async-storage/async-storage"
  );
  return AsyncStorage;
}

export async function saveLang(lang: Lang): Promise<void> {
  memory = lang;
  try {
    await (await store()).setItem(STORAGE_KEY, lang);
  } catch {
    // in-memory fallback already updated
  }
}

/** The language this phone was told to use, or null if nobody has said.
 *
 *  A value that is not one of the three reads as null rather than as
 *  itself. The store is a string store: a build that once wrote a fourth
 *  code, or a hand-edited device, would otherwise hand a language nothing
 *  in `LANGS` answers to straight into `translate`. */
export async function loadLang(): Promise<Lang | null> {
  if (memory !== undefined) return memory;
  try {
    const raw = await (await store()).getItem(STORAGE_KEY);
    memory = isLang(raw) ? raw : null;
    return memory;
  } catch {
    memory = null;
    return null;
  }
}

/** Drops the warm copy so the next `loadLang` reads the store again.
 *
 *  For the tests. Without it a round-trip check passes on the value still
 *  sitting in `memory` from the save, and would go on passing with the
 *  store write deleted. Nothing in the app calls it: a sign-out keeps the
 *  phone's language, because the next person to pick it up is standing at
 *  the same counter. */
export function forgetCachedLang(): void {
  memory = undefined;
}

/** The handset's own locale, or null where the engine cannot say.
 *
 *  Hermes on React Native 0.86 carries `Intl`, so this answers on a real
 *  phone. A build without it, and the web preview under react-native-web
 *  in a browser with no ICU, land on null and take the default rather than
 *  throwing on the first frame. */
export function deviceLocale(): string | null {
  try {
    if (typeof Intl === "undefined") return null;
    return new Intl.DateTimeFormat().resolvedOptions().locale;
  } catch {
    return null;
  }
}

/** The language to open in on a phone that has never been told.
 *
 *  A locale is a tag, not a language: `ar-DZ`, `fr_FR`, `en`. Only the part
 *  in front of the separator is a language code, and a tag naming one the
 *  product does not carry falls to the default. Asking a cashier to choose
 *  a language on a screen written in a language they may not read is the
 *  wrong order, so the handset gets the first guess. */
export function preferredLang(locale: string | null | undefined): Lang {
  if (locale === null || locale === undefined) return DEFAULT_LANG;
  const base = locale.split(/[-_]/)[0]?.toLowerCase();
  return isLang(base) ? base : DEFAULT_LANG;
}

/** Whether this language reads right to left. */
export function readsRightToLeft(lang: Lang): boolean {
  return I18N_RTL.has(lang);
}

/** Whether the app has to be reopened before the layout matches the
 *  language.
 *
 *  React Native flips a screen through `I18nManager`, a native flag that
 *  Android reads once, at process start. Calling `forceRTL` writes it and
 *  it survives, so the flip costs exactly one restart, on the launch after
 *  the change. Until then the sentences are Arabic and the back arrow is
 *  still on the left, which is why the settings screen has to say so
 *  rather than pretend the change landed. */
export function restartNeeded(lang: Lang, laidOutRightToLeft: boolean): boolean {
  return readsRightToLeft(lang) !== laidOutRightToLeft;
}
