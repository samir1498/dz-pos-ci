// The language every screen draws in, held above the router.
//
// Above it, like the session, because the auth gate and the pairing screen
// both have words on them and both render before anything signed in does.
// A cashier who cannot read the sign-in screen cannot reach the setting
// that would fix it.
//
// Two facts come out of here and they are not the same fact. `lang` is what
// the sentences are in and it changes the moment someone picks it.
// `laidOutRightToLeft` is what React Native's native layout is doing, which
// on Android only changes when the process starts again. `restartNeeded`
// is the gap between them, and the settings screen shows it rather than
// letting a half-flipped till look like a bug.

import { translate, type Key, type Lang, type Vars } from "@dzpos/shared";
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { I18nManager } from "react-native";

import {
  DEFAULT_LANG,
  deviceLocale,
  loadLang,
  preferredLang,
  readsRightToLeft,
  restartNeeded,
  saveLang,
} from "../lib/language";

type LanguageState = {
  /** The language the sentences are in. */
  lang: Lang;
  /** False until the stored choice has been read off disk. */
  ready: boolean;
  /** One sentence, in the language in force. */
  t: (key: Key, vars?: Vars) => string;
  /** Remember a new one. The sentences change on the next render. */
  setLang: (lang: Lang) => Promise<void>;
  /** What the language says the layout should be. */
  rightToLeft: boolean;
  /** What the native layout is actually doing, this process. */
  laidOutRightToLeft: boolean;
  /** The two above disagree, so the app has to be reopened once. */
  restartNeeded: boolean;
};

const LanguageContext = createContext<LanguageState | null>(null);

export function useTranslation(): LanguageState {
  const value = useContext(LanguageContext);
  if (value === null) throw new Error("useTranslation outside LanguageProvider");
  return value;
}

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(DEFAULT_LANG);
  const [ready, setReady] = useState(false);

  // Read once, at module-independent mount, the same shape SessionProvider
  // uses. A phone that has never been told takes its own locale, so the
  // first screen a cashier sees in an Algerian shop is already in French
  // or Arabic and not in English.
  useEffect(() => {
    void (async () => {
      const stored = await loadLang();
      setLangState(stored ?? preferredLang(deviceLocale()));
      setReady(true);
    })();
  }, []);

  // Asking for the flip is separate from rendering it. `allowRTL` has to be
  // on or `forceRTL` is ignored, and both are written every time the
  // language settles rather than only when it changes: the flag is native
  // and persistent, so a phone that was last opened in Arabic and is now in
  // French needs the write just as much as the other way round.
  useEffect(() => {
    if (!ready) return;
    I18nManager.allowRTL(true);
    I18nManager.forceRTL(readsRightToLeft(lang));
  }, [lang, ready]);

  const setLang = useCallback(async (next: Lang) => {
    setLangState(next);
    await saveLang(next);
  }, []);

  const value = useMemo<LanguageState>(() => {
    // Read here rather than held in state: it is what this process was
    // started with, and nothing in this process can change it.
    const laidOutRightToLeft = I18nManager.isRTL;
    return {
      lang,
      ready,
      t: (key: Key, vars?: Vars) => translate(lang, key, vars),
      setLang,
      rightToLeft: readsRightToLeft(lang),
      laidOutRightToLeft,
      restartNeeded: restartNeeded(lang, laidOutRightToLeft),
    };
  }, [lang, ready, setLang]);

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}
