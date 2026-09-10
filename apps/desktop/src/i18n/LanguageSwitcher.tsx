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
//
// It is drawn as one segmented control rather than three loose buttons: the
// three are one choice, and a border around the set is what says so. The
// buttons are the kit's `Button` on the ghost variant, with the chosen one
// filled, so the sizes and the focus ring are the app's and not this file's.

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

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
      data-testid="language-switcher"
      className={cn(
        "inline-flex items-center gap-0.5 rounded-md border border-border bg-card p-0.5",
        className,
      )}
    >
      {ORDER.map((l) => (
        <Button
          key={l}
          type="button"
          size="sm"
          variant={lang === l ? "default" : "ghost"}
          aria-pressed={lang === l}
          className="h-7 px-2 text-xs"
          onClick={() => setLang(l)}
        >
          {t(NAME_KEY[l])}
        </Button>
      ))}
    </div>
  );
}
