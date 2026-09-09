import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import ar from "./ar.json";
import en from "./en.json";
import fr from "./fr.json";

export const LANGS = { ar, en, fr } as const;
export type Lang = keyof typeof LANGS;
export type Key = keyof typeof en;

const RTL: ReadonlySet<Lang> = new Set(["ar"]);
const STORAGE_KEY = "dzpos-lang";

type Ctx = {
  lang: Lang;
  dir: "rtl" | "ltr";
  setLang: (l: Lang) => void;
  t: (k: Key) => string;
};

const I18nContext = createContext<Ctx | null>(null);

function isLang(v: string | null): v is Lang {
  return v !== null && v in LANGS;
}

/// English is the key set every language is checked against.
export function isKey(v: string): v is Key {
  return v in en;
}

function initialLang(): Lang {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (isLang(saved)) return saved;
  } catch {
    // no storage (tests, private mode): fall through
  }
  return "fr";
}

export function I18nProvider({ children, lang: forced }: { children: ReactNode; lang?: Lang }) {
  const [lang, setLangState] = useState<Lang>(forced ?? initialLang());
  const value = useMemo<Ctx>(
    () => ({
      lang,
      dir: RTL.has(lang) ? "rtl" : "ltr",
      setLang: (l) => {
        setLangState(l);
        try {
          localStorage.setItem(STORAGE_KEY, l);
        } catch {
          // ignore
        }
      },
      // Fall back to English so a missing key is visible, never blank.
      t: (k) => LANGS[lang][k] ?? en[k],
    }),
    [lang],
  );
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useTranslation(): Ctx {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useTranslation outside I18nProvider");
  return ctx;
}
