import { describe, expect, it, vi } from "vitest";

import AsyncStorage from "@react-native-async-storage/async-storage";

import {
  deviceLocale,
  forgetCachedLang,
  loadLang,
  preferredLang,
  restartNeeded,
  saveLang,
} from "../../lib/language";

// The same Map-as-disk the session store's tests use. Native is absent
// under vitest, so without a stand-in every write falls into the catch and
// a round trip would pass on the warm copy alone.
vi.mock("@react-native-async-storage/async-storage", () => {
  const disk = new Map<string, string>();
  return {
    default: {
      getItem: async (key: string) => disk.get(key) ?? null,
      setItem: async (key: string, value: string) => {
        disk.set(key, value);
      },
      removeItem: async (key: string) => {
        disk.delete(key);
      },
    },
  };
});

const STORAGE_KEY = "dzpos:lang";

describe("the stored choice", () => {
  /** Both directions of one claim: what the store gives back is a language
   *  this build carries. A phone that once ran a build with a fourth code,
   *  or one whose storage was edited, hands `translate` a key into `LANGS`
   *  that answers `undefined`, and every sentence on the screen goes
   *  blank. Reading it as "nobody has said" puts the phone on its default
   *  instead. */
  it("survives a restart, and a code this build does not carry does not", async () => {
    await saveLang("ar");
    forgetCachedLang();
    expect(await loadLang()).toBe("ar");

    await AsyncStorage.setItem(STORAGE_KEY, "es");
    forgetCachedLang();
    expect(await loadLang()).toBeNull();
  });

  /** A write that cannot reach the disk still changes the language for the
   *  session in front of the cashier. That is the whole point of the
   *  in-memory copy, and it only holds while the copy is set before the
   *  await rather than after it: a build that swapped the two lines would
   *  swallow the failure and leave the till in the old language with no
   *  sign anything had gone wrong. */
  it("still switches the language when the disk refuses the write", async () => {
    await saveLang("fr");
    vi.spyOn(AsyncStorage, "setItem").mockRejectedValueOnce(new Error("no native module"));
    await saveLang("ar");
    expect(await loadLang()).toBe("ar");
    vi.restoreAllMocks();
  });
});

describe("the first launch", () => {
  /** A locale is a tag, not a language code. The phones in an Algerian
   *  shop are set to `ar-DZ` or `fr-FR` far more often than to a bare
   *  code, so reading the tag whole would send every one of them to the
   *  default and the guess would never fire. */
  it.each([
    ["ar-DZ", "ar"],
    // Underscore, and Arabic rather than French: `fr_FR` would have read
    // as the default whichever way the tag was split, so it could not have
    // caught a build that matched the tag whole.
    ["ar_DZ", "ar"],
    ["AR-dz", "ar"],
    ["en", "en"],
    // French written out, not `DEFAULT_LANG`. Asserting against the symbol
    // is asserting that the code equals itself: the tests lens flipped the
    // constant to English on 2026-09-20 and all fourteen stayed green,
    // which is every phone with an unfamiliar locale opening in a language
    // no Algerian counter asked for.
    ["es-ES", "fr"],
    ["", "fr"],
    [null, "fr"],
  ])("%s opens in %s", (locale, expected) => {
    expect(preferredLang(locale)).toBe(expected);
  });

  /** The guess runs on the first frame, before anything is on screen, so
   *  a phone that cannot name its own locale has to open in French rather
   *  than fail to open.
   *
   *  Two ways it cannot: no `Intl` at all, which is a Hermes build without
   *  it, and an `Intl` that throws, which is a browser with no ICU data
   *  under react-native-web. The guard only covers the first; the second
   *  is why the try/catch is there, and testing only the guard left the
   *  catch free to be deleted. */
  it.each([
    ["an engine with no Intl", undefined],
    ["an Intl with no locale data", { DateTimeFormat: () => { throw new Error("no ICU"); } }],
  ])("takes French on %s", (_case, stub) => {
    vi.stubGlobal("Intl", stub);
    try {
      expect(deviceLocale()).toBeNull();
      expect(preferredLang(deviceLocale())).toBe("fr");
    } finally {
      vi.unstubAllGlobals();
    }
  });
});

describe("the layout and the sentences can disagree", () => {
  /** The whole truth table, because the wrong half of it is the expensive
   *  one: a phone left saying "reopen the app" after the layout has caught
   *  up teaches a cashier to ignore the notice, and then it says nothing
   *  on the launch where it matters. */
  it.each([
    ["ar", false, true],
    ["ar", true, false],
    ["fr", true, true],
    ["fr", false, false],
    ["en", false, false],
  ] as const)("%s on a %s layout needs a restart: %s", (lang, laidOutRtl, expected) => {
    expect(restartNeeded(lang, laidOutRtl)).toBe(expected);
  });
});
