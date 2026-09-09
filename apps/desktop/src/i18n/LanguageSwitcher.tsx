// Three buttons, one per language. Each is labelled with the language's
// own name, in that language, no matter which language the UI is
// currently in: the French button always reads "Français", even on the
// Arabic screen, the way a switcher conventionally names its own targets.
// The group carries a translated aria-label so a screen reader announces
// it in whatever language is current, and the pressed one is marked with
// aria-pressed rather than a colour alone.
//
// The three names still go through t() with an identical value in every
// dictionary (coding-rules: every visible string through i18n, in ar, fr
// and en) rather than a bare constant map outside it, so a parity check
// on the JSON files still covers them.

import { useTranslation, type Lang } from "./index";

const ORDER: readonly Lang[] = ["fr", "en", "ar"];

const NAME_KEY = {
  fr: "lang_name_fr",
  en: "lang_name_en",
  ar: "lang_name_ar",
} as const;

export function LanguageSwitcher({ className }: { className?: string }) {
  const { lang, setLang, t } = useTranslation();
  return (
    <div
      role="group"
      aria-label={t("lang_switcher_label")}
      className={className === undefined ? "flex gap-2" : `flex gap-2 ${className}`}
    >
      {ORDER.map((l) => (
        <button
          key={l}
          type="button"
          aria-pressed={lang === l}
          className={`rounded border px-2 py-1 text-sm ${lang === l ? "font-semibold" : ""}`}
          onClick={() => setLang(l)}
        >
          {t(NAME_KEY[l])}
        </button>
      ))}
    </div>
  );
}
